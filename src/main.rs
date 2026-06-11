//! Test and validate and format Obo files
mod obo_writer;

use std::{
    collections::HashSet,
    fs::File,
    io::{BufWriter, Write},
};

use mzcore::sequence::CrossId;
use mzcv::{OboOntology, OboStanzaType, RelationType, SynonymScope};

use crate::obo_writer::write_object;

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/home/douwe/Downloads/psi-ms(1).obo".to_string());
    let file = OboOntology::from_file(&path).unwrap();
    let mut answer = String::new();
    println!("{} objects", file.objects.len());

    validate(&file);

    obo_writer::write(
        BufWriter::new(
            File::create(std::path::PathBuf::from(path).with_extension("new.obo")).unwrap(),
        ),
        &file,
    )
    .unwrap();

    loop {
        answer.clear();
        print!(">");
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(&mut answer).unwrap();

        let a = answer.trim();
        if a.eq_ignore_ascii_case("quit") || a.eq_ignore_ascii_case("q") {
            break;
        }

        for obj in &file.objects {
            if obj.lines["name"][0].0.trim().eq_ignore_ascii_case(a)
                || obj
                    .synonyms
                    .iter()
                    .any(|s| s.synonym.eq_ignore_ascii_case(a))
            {
                write_object(std::io::stdout(), obj).unwrap();
            }
        }
    }
}

fn validate(ontology: &OboOntology) {
    let mut warnings = Vec::new();
    let mut names = HashSet::new();
    let mut definitions = HashSet::new();
    let mut ids = HashSet::new();

    for obj in &ontology.objects {
        // Check for duplicate IDs
        if !ids.insert(obj.id.clone()) {
            warnings.push((obj.id.clone(), "Duplicate ID".to_string()));
        }

        // Check for duplicate names
        if let Some(name) = obj.lines.get("name") {
            if name.len() != 1 {
                warnings.push((obj.id.clone(), "Too many names defined".to_string()));
            }
            if !names.insert(name[0].0.as_ref()) {
                warnings.push((obj.id.clone(), format!("Duplicate name: '{}'", name[0].0)));
            }
        } else {
            warnings.push((obj.id.clone(), "No name defined".to_string()));
        }

        if let Some((def, cross_ids, _, _)) = &obj.definition {
            // Check for duplicate definitions
            if !definitions.insert(def.as_ref()) {
                warnings.push((obj.id.clone(), format!("Duplicate definition: '{def}'")));
            }

            // Validate that the cross-ids are valid and written properly
            for id in cross_ids {
                let full = format!(
                    "{}{}",
                    id.0.as_ref().map_or(String::new(), |n| format!("{n}:")),
                    id.1
                );
                if let Ok(id) = CrossId::try_from(id.clone()) {
                    if let CrossId::Other(o) = &id
                        && o.contains(':')
                    {
                        warnings.push((obj.id.clone(), format!("Unknown cross-id: {o}")));
                    } else if !matches!(
                        id,
                        CrossId::URL(_, _) | CrossId::ChemicalBook(_) | CrossId::ChemSpider(_)
                    ) && id.to_string() != full
                    {
                        warnings.push((
                            obj.id.clone(),
                            format!("Not normalised cross-id: '{full}', cleaned is: '{id}'"),
                        ));
                    }
                } else {
                    warnings.push((obj.id.clone(), format!("Invalid cross-id type: {full}")));
                }
            }
        } else {
            warnings.push((obj.id.clone(), "No definition".to_string()));
        }

        // Check for duplicate synonyms
        for synonym in &obj.synonyms {
            if synonym.scope != SynonymScope::Exact {
                continue;
            }
            if !names.insert(synonym.synonym.as_ref()) {
                warnings.push((
                    obj.id.clone(),
                    format!("Duplicate exact synonym: '{}'", synonym.synonym),
                ));
            }
        }

        // Validate the lines are actually valid Obo lines
        if obj.stanza_type == OboStanzaType::Term {
            for line in obj.lines.keys() {
                if ![
                    "alt_id",
                    "builtin",
                    "comment",
                    "consider",
                    "created_by",
                    "creation_date",
                    "def",
                    "disjoint_from",
                    "equivalent_to",
                    "intersection_of",
                    "is_anonymous",
                    "name",
                    "namespace",
                    "replaced_by",
                    "subset",
                    "union_of",
                ]
                .contains(&line.as_ref())
                {
                    warnings.push((obj.id.clone(), format!("Invalid line type: {line}")));
                }
            }
        }

        // Check for suspicious comments
        let mut parent_stack = Vec::new();
        for (t, rel, _, comment) in &obj.relationship {
            if rel.0.as_ref().is_some_and(|t| t.as_ref() == "xsd") {
                continue;
            }
            if let Some(relation) = ontology.objects.iter().find(|o| o.id == *rel) {
                if *t == RelationType::IsA {
                    if let Some(comment) = comment
                        && *comment != relation.lines["name"][0].0
                    {
                        warnings.push((
                            obj.id.clone(),
                            format!("This relationship comment looks suspicious: name is '{}' comment is '{comment}'", relation.lines["name"][0].0),
                        ));
                    }
                    parent_stack.push(relation);
                }
            } else {
                warnings.push((
                    obj.id.clone(),
                    format!("Referenced relation does not exist in this file: {rel}"),
                ));
            }
        }

        // Check for duplicated inherited values
        while let Some(ancestor) = parent_stack.pop() {
            for (key, values) in &ancestor.property_values {
                if let Some(own_values) = obj.property_values.get(key) {
                    for own_value in own_values {
                        if values.iter().any(|v| v.0 == own_value.0) {
                            warnings.push((
                                obj.id.clone(),
                                format!("Overwrote an ancestral property value with the same value: '{}' from ancestor: {}", own_value.0, ancestor.id),
                            ));
                        }
                    }
                }
            }
            for (id, _, _) in &ancestor.xref {
                if obj.xref.iter().any(|v| v.0 == *id) {
                    warnings.push((
                        obj.id.clone(),
                        format!("Overwrote an ancestral xref with the same value: '{}' from ancestor: {}", id, ancestor.id),
                    ));
                }
            }
            for (t, rel, _, _) in &ancestor.relationship {
                if *t == RelationType::IsA
                    && let Some(relation) = ontology.objects.iter().find(|o| o.id == *rel)
                {
                    parent_stack.push(relation);
                }
            }
        }
    }

    for (id, warning) in warnings {
        println!("{id}: {warning}");
    }
}
