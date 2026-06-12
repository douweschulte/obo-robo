use mzcore::chemistry::MolecularFormula;
use mzcv::{OboIdentifier, OboOntology};

pub fn fix_psi_mod(ontology: &mut OboOntology) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for obj in &mut ontology.objects {
        if let Some(entry) = obj
            .xref
            .iter_mut()
            .find(|x| x.0.0.as_ref().is_some_and(|t| t.as_ref() == "DiffFormula"))
        {
            let trimmed = entry.0.1.trim().trim_matches('\"');
            if !trimmed.eq_ignore_ascii_case("none") {
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
                    }
                    Err(err) => errors.push(format!("{}: Invalid DiffFormula: {}", obj.id, err)),
                }
            }
        }

        if let Some(entry) = obj
            .xref
            .iter_mut()
            .find(|x| x.0.0.as_ref().is_some_and(|t| t.as_ref() == "Formula"))
        {
            let trimmed = entry.0.1.trim().trim_matches('\"');
            if !trimmed.eq_ignore_ascii_case("none") {
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
                    }
                    Err(err) => errors.push(format!("{}: Invalid Formula: {}", obj.id, err)),
                }
            }
        }

        // TODO: check that the DiffFormula and Formula are correct in relation to each other and maybe automatically calculate the other if only one is present
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
