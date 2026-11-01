use std::collections::HashSet;

use mzcv::{OboIdentifier, OboOntology, OboStanzaType};

use crate::ErrorLevel;

pub fn fix_psi_ms(
    ontology: &mut OboOntology,
    subset: Option<&HashSet<OboIdentifier>>,
    fix_all: bool,
) -> Vec<(ErrorLevel, OboIdentifier, String)> {
    let mut errors = Vec::new();

    for obj in &mut ontology.objects {
        if !fix_all && subset.is_some_and(|s| !s.contains(&obj.id)) {
            continue;
        }
        if obj.stanza_type != OboStanzaType::Term {
            continue;
        }
        if !obj.lines.contains_key("subset") {
            errors.push((
                ErrorLevel::Error,
                obj.id.clone(),
                "Term has no subset defined".to_string(),
            ));
        }
    }

    errors
}
