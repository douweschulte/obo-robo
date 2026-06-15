use mzcore::chemistry::{Element, MolecularFormula};
use mzcv::{OboIdentifier, OboOntology, OboStanza, RelationType};

pub fn fix_psi_mod(ontology: &mut OboOntology) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for obj in &mut ontology.objects {
        let _diff_formula = if let Some(entry) = obj
            .xref
            .iter_mut()
            .find(|x| x.0.0.as_ref().is_some_and(|t| t.as_ref() == "DiffFormula"))
        {
            let trimmed = entry.0.1.trim().trim_matches('\"');
            if trimmed.eq_ignore_ascii_case("none") {
                None
            } else {
                match MolecularFormula::psi_mod(trimmed) {
                    Ok(formula) => {
                        entry.0.1 = format!(
                            "\"{}\"",
                            formula
                                .hill_notation_psi_mod()
                                .expect("Charged or additional mass in formula in PSI-MOD")
                        )
                        .into();

                        if let Some(entry) = obj
                            .xref
                            .iter_mut()
                            .find(|x| x.0.0.as_ref().is_some_and(|t| t.as_ref() == "DiffMono"))
                        {
                            entry.0.1 =
                                format!("\"{:.6}\"", formula.monoisotopic_mass().value).into();
                        } else {
                            obj.xref.push((
                                OboIdentifier(
                                    Some("DiffMono".into()),
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
                                    Some("DiffAvg".into()),
                                    format!("\"{:.2}\"", formula.average_weight().value).into(),
                                ),
                                Vec::new(),
                                None,
                            ));
                        }
                        Some(formula)
                    }
                    Err(err) => {
                        errors.push(format!("{}: Invalid DiffFormula: {}", obj.id, err));
                        None
                    }
                }
            }
        } else {
            None
        };

        let _full_formula = if let Some(entry) = obj
            .xref
            .iter_mut()
            .find(|x| x.0.0.as_ref().is_some_and(|t| t.as_ref() == "Formula"))
        {
            let trimmed = entry.0.1.trim().trim_matches('\"');
            if trimmed.eq_ignore_ascii_case("none") {
                None
            } else {
                match MolecularFormula::psi_mod(trimmed) {
                    Ok(formula) => {
                        entry.0.1 = format!(
                            "\"{}\"",
                            formula
                                .hill_notation_psi_mod()
                                .expect("Charged or additional mass in formula in PSI-MOD")
                        )
                        .into();

                        if let Some(entry) = obj
                            .xref
                            .iter_mut()
                            .find(|x| x.0.0.as_ref().is_some_and(|t| t.as_ref() == "MassMono"))
                        {
                            entry.0.1 =
                                format!("\"{:.6}\"", formula.monoisotopic_mass().value).into();
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

        // TODO: check that the DiffFormula and Formula are correct in relation to each other and maybe automatically calculate the other if only one is present

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

// fn insert_relation(object: &mut OboStanza, t: RelationType, id: OboIdentifier, name: Box<str>) {
//     if !object.relationship.iter().any(|r| r.0 == t && r.1 == id) {
//         object.relationship.push((t, id, Vec::new(), Some(name)));
//     }
// }
