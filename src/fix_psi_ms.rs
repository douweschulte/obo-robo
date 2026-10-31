use std::collections::HashSet;

use mzcv::{OboIdentifier, OboOntology, OboStanzaType};

pub fn fix_psi_ms(
    ontology: &mut OboOntology,
    subset: Option<&HashSet<OboIdentifier>>,
    fix_all: bool,
) -> Result<(), Vec<(OboIdentifier, String)>> {
    let mut errors = Vec::new();

    for obj in &mut ontology.objects {
        if !fix_all && subset.is_some_and(|s| !s.contains(&obj.id)) {
            continue;
        }
        if obj.stanza_type != OboStanzaType::Term {
            continue;
        }
        if !obj.lines.contains_key("subset") {
            errors.push((obj.id.clone(), "Term has no subset defined".to_string()));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
