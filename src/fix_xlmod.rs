use mzcore::chemistry::MolecularFormula;
use mzcv::{OboOntology, OboValue};

pub fn fix_xlmod(ontology: &mut OboOntology) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for obj in &mut ontology.objects {
        let formula_key = if obj.property_values.contains_key("bridgeFormula") {
            "bridgeFormula"
        } else {
            "deadEndFormula"
        };

        if let Some(entry) = obj.property_values.get_mut(formula_key) {
            if entry.len() != 1 {
                errors.push(format!(
                    "{}: Too many {formula_key}s, can only have 1",
                    obj.id
                ));
            }
            match MolecularFormula::xlmod(&entry[0].0.to_string()) {
                Ok(formula) => {
                    entry[0].0 = OboValue::String(
                        formula
                            .hill_notation_xlmod()
                            .expect("Charged or additional mass in formula in XLMOD"),
                    );

                    obj.property_values.insert(
                        "monoIsotopicMass".into(),
                        vec![(
                            OboValue::Float(formula.monoisotopic_mass().value, "double", Some(6)),
                            Vec::new(),
                            None,
                        )],
                    );
                }
                Err(err) => errors.push(format!("{}: Invalid {formula_key}: {}", obj.id, err)),
            }
        }

        if let Some(entries) = obj.property_values.get_mut("neutralLossFormula") {
            for entry in entries {
                match MolecularFormula::xlmod(&entry.0.to_string()) {
                    Ok(formula) => {
                        entry.0 = OboValue::String(
                            formula
                                .hill_notation_xlmod()
                                .expect("Charged or additional mass in formula in XLMOD"),
                        );
                    }
                    Err(err) => {
                        errors.push(format!("{}: Invalid neutralLossFormula: {}", obj.id, err));
                    }
                }
            }
        }

        if let Some(entry) = obj.property_values.get_mut("reactionSites") {
            if entry.len() != 1 {
                errors.push(format!(
                    "{}: Too many reactionSites entries, can only have 1",
                    obj.id
                ));
            }
            if let OboValue::Integer(n, _) = entry[0].0 {
                if n < 0 {
                    errors.push(format!("{}: reactionSites is negative", obj.id));
                } else {
                    entry[0].0 = OboValue::Integer(n, "nonNegativeInteger");
                    if n > 10 {
                        errors.push(format!(
                            "{}: Too high number of reactionSites, has to be below 10",
                            obj.id
                        ));
                    }
                }
            }
        }

        if let Some(entry) = obj.property_values.get_mut("hydrophilicPEGchain") {
            if entry.len() != 1 {
                errors.push(format!(
                    "{}: Too many hydrophilicPEGchain entries, can only have 1",
                    obj.id
                ));
            }
            if let OboValue::Integer(n, _) = entry[0].0 {
                if n < 0 {
                    errors.push(format!("{}: hydrophilicPEGchain is negative", obj.id));
                } else {
                    entry[0].0 = OboValue::Integer(n, "nonNegativeInteger");
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
