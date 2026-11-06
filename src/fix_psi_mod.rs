use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use mzcore::{
    chemistry::{AmbiguousMolecule, MassOutputType, MolecularFormula},
    sequence::AminoAcid,
};
use mzcv::{OboIdentifier, OboOntology};

use crate::ErrorLevel;

pub fn fix_psi_mod(
    ontology: &mut OboOntology,
    subset: Option<&HashSet<OboIdentifier>>,
    fix_all: bool,
) -> Vec<(ErrorLevel, OboIdentifier, String)> {
    let mut errors = Vec::new();
    let mut formulas = HashMap::new();

    for obj in &mut ontology.objects {
        if !fix_all && subset.is_some_and(|s| !s.contains(&obj.id)) {
            continue;
        }
        let mut diff_formula = if let Some(entry) = obj
            .xref
            .iter()
            .find(|x| x.0.0.as_ref().is_some_and(|t| t.as_ref() == "DiffFormula"))
        {
            let trimmed = entry.0.1.trim().trim_matches('\"');
            if trimmed.eq_ignore_ascii_case("none") {
                None
            } else {
                match MolecularFormula::psi_mod(trimmed) {
                    Ok(formula) => Some(formula),
                    Err(err) => {
                        errors.push((
                            ErrorLevel::Error,
                            obj.id.clone(),
                            format!("Invalid DiffFormula: {err}"),
                        ));
                        None
                    }
                }
            }
        } else {
            None
        };

        let mut full_formula = if let Some(entry) = obj
            .xref
            .iter()
            .find(|x| x.0.0.as_ref().is_some_and(|t| t.as_ref() == "Formula"))
        {
            let trimmed = entry.0.1.trim().trim_matches('\"');
            if trimmed.eq_ignore_ascii_case("none") {
                None
            } else {
                match MolecularFormula::psi_mod(trimmed) {
                    Ok(formula) => {
                        formulas.insert(obj.id.clone(), formula.clone());
                        Some(formula)
                    }
                    Err(err) => {
                        errors.push((
                            ErrorLevel::Error,
                            obj.id.clone(),
                            format!("Invalid Formula: {err}"),
                        ));
                        None
                    }
                }
            }
        } else {
            None
        };

        let smiles = if let Some(entry) = obj
            .xref
            .iter()
            .find(|x| x.0.0.as_ref().is_some_and(|t| t.as_ref() == "Smiles"))
        {
            let trimmed = entry.0.1.trim().trim_matches('\"');
            match mzcore::chemistry::StructuralFormula::from_smiles(trimmed) {
                Ok(structure) => structure.composition().map(|mut f| {
                    let c = f.charge();
                    f.set_charge(mzcore::system::isize::Charge::default());
                    (f, c)
                }),
                Err(err) => {
                    errors.push((
                        ErrorLevel::Error,
                        obj.id.clone(),
                        format!("Invalid Smiles: {err}"),
                    ));
                    None
                }
            }
        } else {
            None
        };

        let mut charge: Option<mzcore::system::isize::Charge> = if let Some(entry) = obj
            .xref
            .iter_mut()
            .find(|x| x.0.0.as_ref().is_some_and(|t| t.as_ref() == "FormalCharge"))
        {
            let trimmed = entry.0.1.trim().trim_matches('\"');
            let num = if let Some(num) = trimmed.strip_suffix("-") {
                Some((num, true))
            } else if let Some(num) = trimmed.strip_suffix("+") {
                Some((num, false))
            } else {
                errors.push((
                    ErrorLevel::Error,
                    obj.id.clone(),
                    "Invalid FormalCharge".to_string(),
                ));
                None
            };
            if let Some((num, negative)) = num {
                match num.parse::<u32>() {
                    Ok(num) => Some(mzcore::system::isize::Charge::new::<mzcore::system::e>(
                        num as isize * if negative { -1 } else { 1 },
                    )),
                    Err(err) => {
                        errors.push((
                            ErrorLevel::Error,
                            obj.id.clone(),
                            format!("Invalid FormalCharge: {err}"),
                        ));
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        let origin: Option<Vec<PsiOrigin>> = if let Some(entry) = obj
            .xref
            .iter_mut()
            .find(|x| x.0.0.as_ref().is_some_and(|t| t.as_ref() == "Origin"))
        {
            let trimmed = entry.0.1.trim().trim_matches('\"');
            if trimmed.eq_ignore_ascii_case("none") {
                None
            } else if let Ok(v) = trimmed.split(',').map(str::parse).collect() {
                Some(v)
            } else {
                errors.push((
                    ErrorLevel::Error,
                    obj.id.clone(),
                    "Invalid Origin".to_string(),
                ));
                None
            }
        } else {
            None
        };

        match (
            origin.as_ref().and_then(|o| {
                o.iter().try_fold(MolecularFormula::default(), |acc, o| {
                    o.full(&formulas).map(|b| acc + b)
                })
            }),
            &diff_formula,
            &full_formula,
            &smiles,
        ) {
            (Some(base), Some(d), Some(f), Some((s, _))) => {
                let reconstructed = &base + d;
                if reconstructed != *f {
                    errors.push((ErrorLevel::Error,obj.id.clone(), format!(
                        "DiffFormula '{d}' plus the origins '{base}' (summed '{reconstructed}') do not match with the Formula '{f}' difference is '{}' (summed - Formula)",
                        &reconstructed - f
                    )));
                }
                if reconstructed != *s {
                    errors.push((ErrorLevel::Error,obj.id.clone(), format!(
                        "DiffFormula '{d}' plus the origins '{base}' (summed '{reconstructed}') do not match with the Smiles '{s}' difference is '{}' (summed - Smiles)",
                        &reconstructed - s
                    )));
                }
                if *f != *s {
                    errors.push((ErrorLevel::Error,obj.id.clone(), format!(
                        "Formula '{f}' do not match with the Smiles '{s}' difference is '{}' (Formula - Smiles)",
                        f - s
                    )));
                }
            }
            (Some(base), Some(d), Some(f), None) => {
                let reconstructed = &base + d;
                if reconstructed != *f {
                    errors.push((ErrorLevel::Error,obj.id.clone(), format!(
                        "DiffFormula '{d}' plus the origins '{base}' (summed '{reconstructed}') do not match with the Formula '{f}' difference is '{}' (summed - Formula)",
                        &reconstructed - f
                    )));
                }
            }
            (Some(base), Some(d), None, Some((s, _))) => {
                let reconstructed = &base + d;
                if reconstructed != *s {
                    errors.push((ErrorLevel::Error,obj.id.clone(), format!(
                        "DiffFormula '{d}' plus the origins '{base}' (summed '{reconstructed}') do not match with the Smiles '{s}' difference is '{}' (summed - Smiles)",
                        &reconstructed - s
                    )));
                }
                full_formula = Some(s.clone());
            }
            (Some(base), Some(d), None, None) => {
                full_formula = Some(base + d);
            }
            (Some(base), None, Some(f), Some((s, _))) => {
                diff_formula = Some(f - base);
                if *f != *s {
                    errors.push((ErrorLevel::Error,obj.id.clone(), format!(
                        "Formula '{f}' do not match with the Smiles '{s}' difference is '{}' (Formula - Smiles)",
                        f - s
                    )));
                }
            }
            (Some(base), None, Some(f), None) => {
                diff_formula = Some(f - base);
            }
            (Some(base), None, None, Some((s, _))) => {
                diff_formula = Some(s - base);
                full_formula = Some(s.clone());
            }
            (_, _, Some(f), Some((s, _))) => {
                if *f != *s {
                    errors.push((ErrorLevel::Error,obj.id.clone(), format!(
                        "Formula '{f}' do not match with the Smiles '{s}' difference is '{}' (Formula - Smiles)",
                        f - s
                    )));
                }
            }
            (..) => (),
        }

        if let Some((_, smiles_c)) = smiles {
            if let Some(c) = charge {
                if smiles_c != c {
                    errors.push((
                        ErrorLevel::Error,
                        obj.id.clone(),
                        format!(
                            "The FormalCharge ({}) differs from the Smiles charge ({})",
                            c.value, smiles_c.value
                        ),
                    ));
                }
            } else {
                charge = Some(smiles_c);
            }
        }

        // Detect if the origin is defined (so not just Xs) to not introduce full formulas when only a diff formula makes sense
        let defined_origin = origin.as_ref().is_some_and(|o| {
            o.iter()
                .any(|o| *o != PsiOrigin::AminoAcid(AminoAcid::Unknown))
        });

        if let Some(formula) = diff_formula {
            set_xref(
                &mut obj.xref,
                "DiffFormula",
                format!(
                    "\"{}\"",
                    formula
                        .hill_notation_psi_mod()
                        .expect("Charged or additional mass in formula in PSI-MOD")
                )
                .into(),
                false,
                !defined_origin,
            );
            set_xref(
                &mut obj.xref,
                "DiffMono",
                format!("\"{:.6}\"", formula.monoisotopic_mass().value).into(),
                false,
                !defined_origin,
            );
            set_xref(
                &mut obj.xref,
                "DiffAvg",
                format!("\"{:.2}\"", formula.average_weight().value).into(),
                false,
                !defined_origin,
            );
        }

        if let Some(formula) = full_formula {
            set_xref(
                &mut obj.xref,
                "Formula",
                format!(
                    "\"{}\"",
                    formula
                        .hill_notation_psi_mod()
                        .expect("Charged or additional mass in formula in PSI-MOD")
                )
                .into(),
                false,
                !defined_origin,
            );
            set_xref(
                &mut obj.xref,
                "MassMono",
                format!("\"{:.6}\"", formula.monoisotopic_mass().value).into(),
                false,
                !defined_origin,
            );
            set_xref(
                &mut obj.xref,
                "MassAvg",
                format!("\"{:.2}\"", formula.average_weight().value).into(),
                false,
                !defined_origin,
            );
            formulas.insert(obj.id.clone(), formula.clone());
        }

        if let Some(charge) = charge {
            set_xref(
                &mut obj.xref,
                "FormalCharge",
                format!(
                    "{}{}",
                    charge.value.abs(),
                    if charge.value > 0 { '+' } else { '-' }
                )
                .into(),
                charge.value == 0,
                true,
            );
        }

        // TODO: detect missing relationships like 'MOD:00842|(13)C labeled residue' 'MOD:00843|(15)N labeled residue' and 'MOD:00902|modified L-arginine residue'
        // TODO: figure out the logic of the tagged reagent differences to the labeled residue first 'MOD:01431|(2)H deuterium tagged reagent'
        // if let Some(formula) = diff_formula {
        //     if let Some((_, _, amount)) = formula
        //         .elements()
        //         .iter()
        //         .find(|e| e.0 == Element::H && e.1.is_some_and(|i| i.get() == 2))
        //         && *amount > 0
        //     {
        //         insert_relation(
        //             obj,
        //             RelationType::IsA,
        //             OboIdentifier(Some("MOD".into()), "00839".into()),
        //             "(2)H deuterium labeled residue".into(),
        //         );
        //     }
        //     if let Some((_, _, amount)) = formula
        //         .elements()
        //         .iter()
        //         .find(|e| e.0 == Element::C && e.1.is_some_and(|i| i.get() == 13))
        //         && *amount > 0
        //     {
        //         insert_relation(
        //             obj,
        //             RelationType::IsA,
        //             OboIdentifier(Some("MOD".into()), "00842".into()),
        //             "(13)C labeled residue".into(),
        //         );
        //     }
        //     if let Some((_, _, amount)) = formula
        //         .elements()
        //         .iter()
        //         .find(|e| e.0 == Element::N && e.1.is_some_and(|i| i.get() == 15))
        //         && *amount > 0
        //     {
        //         insert_relation(
        //             obj,
        //             RelationType::IsA,
        //             OboIdentifier(Some("MOD".into()), "00843".into()),
        //             "(15)N labeled residue".into(),
        //         );
        //     }
        //     if let Some((_, _, amount)) = formula
        //         .elements()
        //         .iter()
        //         .find(|e| e.0 == Element::O && e.1.is_some_and(|i| i.get() == 18))
        //         && *amount > 0
        //     {
        //         insert_relation(
        //             obj,
        //             RelationType::IsA,
        //             OboIdentifier(Some("MOD".into()), "00844".into()),
        //             "(18)O labeled residue".into(),
        //         );
        //     }
        // }
    }

    errors
}

/// Set the specified property value (in xref because PSI-MOD), remove the property entirely if `remove` is set to true, or if `insert` is true insert the property if it is not there already.
fn set_xref(
    xref: &mut Vec<(OboIdentifier, Vec<(Box<str>, Box<str>)>, Option<Box<str>>)>,
    name: &'static str,
    value: Box<str>,
    remove: bool,
    insert: bool,
) {
    if remove {
        xref.retain(|x| x.0.0.as_ref().is_some_and(|t| t.as_ref() != name))
    } else {
        if let Some(entry) = xref
            .iter_mut()
            .find(|x| x.0.0.as_ref().is_some_and(|t| t.as_ref() == name))
        {
            entry.0.1 = value;
        } else if insert {
            xref.push((OboIdentifier(Some(name.into()), value), Vec::new(), None));
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PsiOrigin {
    AminoAcid(AminoAcid),
    Modification(OboIdentifier),
}

impl PsiOrigin {
    fn full(
        &self,
        formulas: &HashMap<OboIdentifier, MolecularFormula>,
    ) -> Option<MolecularFormula> {
        match self {
            Self::AminoAcid(a) => a.formulas().single(),
            Self::Modification(m) => formulas.get(m).cloned(),
        }
    }
}

impl FromStr for PsiOrigin {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        s.parse::<AminoAcid>().map_or_else(
            |()| {
                if s.contains(':')
                    && let Ok(id) = s.parse()
                {
                    Ok(Self::Modification(id))
                } else {
                    Err(())
                }
            },
            |aa| Ok(Self::AminoAcid(aa)),
        )
    }
}

// fn insert_relation(object: &mut OboStanza, t: RelationType, id: OboIdentifier, name: Box<str>) {
//     if !object.relationship.iter().any(|r| r.0 == t && r.1 == id) {
//         object.relationship.push((t, id, Vec::new(), Some(name)));
//     }
// }
