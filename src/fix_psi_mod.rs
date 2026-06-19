use std::{collections::HashMap, str::FromStr};

use mzcore::{
    chemistry::{MolecularFormula, MultiChemical},
    sequence::AminoAcid,
};
use mzcv::{OboIdentifier, OboOntology};

pub fn fix_psi_mod(ontology: &mut OboOntology) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let mut formulas = HashMap::new();

    for obj in &mut ontology.objects {
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
                        errors.push(format!("{}: Invalid DiffFormula: {}", obj.id, err));
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
                        errors.push(format!("{}: Invalid Formula: {}", obj.id, err));
                        None
                    }
                }
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
            } else {
                match trimmed.split(',').map(|s| s.parse()).collect() {
                    Ok(v) => Some(v),
                    Err(()) => {
                        errors.push(format!("{}: Invalid Origin", obj.id));
                        None
                    }
                }
            }
        } else {
            None
        };

        if let Some(origin) = origin {
            if let Some(base) = origin
                .iter()
                .fold(Some(MolecularFormula::default()), |acc, o| {
                    acc.and_then(|a| o.full(&formulas).map(|b| a + b))
                })
            {
                match (&diff_formula, &full_formula) {
                    (Some(d), Some(f)) => {
                        let s = &base + d;
                        if s != *f {
                            errors.push(format!(
                                "{}: DiffFormula '{d}' plus the origins '{base}' (summed '{s}') do not match with the Formula '{f}' difference is '{}' (summed - Formula)",
                                obj.id,&s - f
                            ));
                        }
                    }
                    (Some(d), None) => {
                        full_formula = Some(base + d);
                    }
                    (None, Some(f)) => {
                        diff_formula = Some(f - base);
                    }
                    (None, None) => (),
                }
            }
        }

        if let Some(formula) = diff_formula {
            if let Some(entry) = obj
                .xref
                .iter_mut()
                .find(|x| x.0.0.as_ref().is_some_and(|t| t.as_ref() == "DiffFormula"))
            {
                entry.0.1 = format!(
                    "\"{}\"",
                    formula
                        .hill_notation_psi_mod()
                        .expect("Charged or additional mass in formula in PSI-MOD")
                )
                .into();
            } else {
                obj.xref.push((
                    OboIdentifier(
                        Some("DiffFormula".into()),
                        format!(
                            "\"{}\"",
                            formula
                                .hill_notation_psi_mod()
                                .expect("Charged or additional mass in formula in PSI-MOD")
                        )
                        .into(),
                    ),
                    Vec::new(),
                    None,
                ));
            }

            if let Some(entry) = obj
                .xref
                .iter_mut()
                .find(|x| x.0.0.as_ref().is_some_and(|t| t.as_ref() == "DiffMono"))
            {
                entry.0.1 = format!("\"{:.6}\"", formula.monoisotopic_mass().value).into();
            } else {
                obj.xref.push((
                    OboIdentifier(
                        Some("MassMono".into()),
                        format!("\"{:.6}\"", formula.monoisotopic_mass().value).into(),
                    ),
                    Vec::new(),
                    None,
                ));
            }

            if let Some(entry) = obj
                .xref
                .iter_mut()
                .find(|x| x.0.0.as_ref().is_some_and(|t| t.as_ref() == "DiffAvg"))
            {
                entry.0.1 = format!("\"{:.2}\"", formula.average_weight().value).into();
            } else {
                obj.xref.push((
                    OboIdentifier(
                        Some("MassAvg".into()),
                        format!("\"{:.2}\"", formula.average_weight().value).into(),
                    ),
                    Vec::new(),
                    None,
                ));
            }
        }

        if let Some(formula) = full_formula {
            if let Some(entry) = obj
                .xref
                .iter_mut()
                .find(|x| x.0.0.as_ref().is_some_and(|t| t.as_ref() == "Formula"))
            {
                entry.0.1 = format!(
                    "\"{}\"",
                    formula
                        .hill_notation_psi_mod()
                        .expect("Charged or additional mass in formula in PSI-MOD")
                )
                .into();
            } else {
                obj.xref.push((
                    OboIdentifier(
                        Some("Formula".into()),
                        format!(
                            "\"{}\"",
                            formula
                                .hill_notation_psi_mod()
                                .expect("Charged or additional mass in formula in PSI-MOD")
                        )
                        .into(),
                    ),
                    Vec::new(),
                    None,
                ));
            }

            if let Some(entry) = obj
                .xref
                .iter_mut()
                .find(|x| x.0.0.as_ref().is_some_and(|t| t.as_ref() == "MassMono"))
            {
                entry.0.1 = format!("\"{:.6}\"", formula.monoisotopic_mass().value).into();
            } else {
                obj.xref.push((
                    OboIdentifier(
                        Some("MassMono".into()),
                        format!("\"{:.6}\"", formula.monoisotopic_mass().value).into(),
                    ),
                    Vec::new(),
                    None,
                ));
            }

            if let Some(entry) = obj
                .xref
                .iter_mut()
                .find(|x| x.0.0.as_ref().is_some_and(|t| t.as_ref() == "MassAvg"))
            {
                entry.0.1 = format!("\"{:.2}\"", formula.average_weight().value).into();
            } else {
                obj.xref.push((
                    OboIdentifier(
                        Some("MassAvg".into()),
                        format!("\"{:.2}\"", formula.average_weight().value).into(),
                    ),
                    Vec::new(),
                    None,
                ));
            }
            formulas.insert(obj.id.clone(), formula.clone());
        }

        // TODO: detect missing relationships like `MOD:00842|(13)C labeled residue` `MOD:00843|(15)N labeled residue` and `MOD:00902|modified L-arginine residue`
        // TODO: figure out the logic of the tagged reagent differences to the labeled residue first `MOD:01431|(2)H deuterium tagged reagent`
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

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

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
            Self::AminoAcid(a) => a.single_formula(),
            Self::Modification(m) => formulas.get(m).cloned(),
        }
    }
}

impl FromStr for PsiOrigin {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        if let Ok(aa) = s.parse::<AminoAcid>() {
            Ok(Self::AminoAcid(aa))
        } else if s.contains(':')
            && let Ok(id) = s.parse()
        {
            Ok(Self::Modification(id))
        } else {
            Err(())
        }
    }
}

// fn insert_relation(object: &mut OboStanza, t: RelationType, id: OboIdentifier, name: Box<str>) {
//     if !object.relationship.iter().any(|r| r.0 == t && r.1 == id) {
//         object.relationship.push((t, id, Vec::new(), Some(name)));
//     }
// }
