//! Test and validate and format Obo files
mod fix_psi_mod;
mod fix_xlmod;
mod obo_writer;
mod update_psi_mod;

use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    process::ExitCode,
};

use itertools::Itertools;
use mzcore::sequence::CrossId;
use mzcv::{OboIdentifier, OboOntology, OboStanzaType, RelationType, SynonymScope};

use crate::{
    obo_writer::{OboFormattingOptions, write_object},
    update_psi_mod::psi_mod_proper_style,
};

enum Action {
    Lint,
    Fmt,
    Fix,
    NewFormat,
    Search,
}

fn main() -> ExitCode {
    let Some(action) = std::env::args().nth(1) else {
        eprintln!(
            "An action should be given as first argument, any of 'lint', 'fmt', 'fix', 'newfmt', or 'search'"
        );
        return 1.into();
    };
    let Some(path) = std::env::args().nth(2) else {
        eprintln!("A path should be given as second argument");
        return 2.into();
    };

    let action = match action.as_str() {
        "lint" => Action::Lint,
        "fmt" => Action::Fmt,
        "newfmt" => Action::NewFormat,
        "fix" => Action::Fix,
        "search" => Action::Search,
        _ => {
            println!("Usage: <action> <path>");
            println!("Use any of 'lint', 'fmt', 'fix', or 'search' as action");
            println!(" 'lint' - Shows general Obo errors about the file");
            println!(" 'fmt' - Formats the Obo file with generic rules");
            println!(" 'fix' - Fix with ontology specific rules and formats the Obo file");
            println!(
                " 'newfmt' - Fix PSI-MOD to follow the Obo format better to create a proper owl file"
            );
            println!(" 'search' - Load an Obo file to search in it interactively");
            return 0.into();
        }
    };

    let mut file = match OboOntology::from_file(&path) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{}", err);
            return 3.into();
        }
    };

    match action {
        Action::Lint => {
            if let Some(lines) = std::env::args().nth(3) {
                let lines = lines
                    .split(',')
                    .filter_map(|n| n.parse::<usize>().ok())
                    .sorted()
                    .collect::<Vec<_>>();
                let items = detect_items(&lines, &path);
                println!("Changed items: {}", items.iter().sorted().join(", "));
                let _ = validate(&file, Some(&items));
            } else {
                let _ = validate(&file, None);
            }
        }
        Action::Fmt => fmt(&file, &path),
        Action::Fix => {
            let mut error = false;
            if let Err(errs) = fix(&mut file) {
                for err in errs {
                    eprintln!("::error::{err}");
                    error = true;
                }
            }
            fmt(&file, &path);
            if error {
                return 5.into();
            }
        }
        Action::NewFormat => {
            psi_mod_proper_style(&mut file);
            fmt(&file, "PSI-MOD-newstyle.obo");
        }
        Action::Search => {
            println!(
                "Version: {}, Objects: {}",
                file.version().version.unwrap_or_default(),
                file.objects.len()
            );
            let mut answer = String::new();
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
                        write_object(std::io::stdout(), obj, &OboFormattingOptions::default())
                            .unwrap();
                    }
                }
            }
        }
    }
    0.into()
}

fn fmt(ontology: &OboOntology, path: &str) {
    obo_writer::write(
        BufWriter::new(File::create(path).unwrap()),
        &ontology,
        &OboFormattingOptions {
            format_xref_as_property_value: ontology
                .headers
                .iter()
                .any(|h| h.0.as_ref() == "ontology" && h.1.as_ref() == "mod"),
        },
    )
    .unwrap()
}

fn validate(ontology: &OboOntology, subset: Option<&HashSet<OboIdentifier>>) -> bool {
    let mut warnings = Vec::new();
    let mut names = HashSet::new();
    let mut definitions = HashSet::new();
    let mut ids = HashSet::new();

    for obj in &ontology.objects {
        if subset.is_some_and(|s| !s.contains(&obj.id)) {
            continue;
        }

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

    if !warnings.is_empty() {
        println!("::warning::Some potential problems where detected")
    }
    for (id, warning) in &warnings {
        println!("::notice::{id}: {warning}");
    }

    warnings.is_empty()
}

fn fix(ontology: &mut OboOntology) -> Result<(), Vec<String>> {
    if let Some((_, name, _, _)) = ontology.headers.iter().find(|h| h.0.as_ref() == "ontology") {
        match name.as_ref() {
            "xlmod" => fix_xlmod::fix_xlmod(ontology),
            "mod" => fix_psi_mod::fix_psi_mod(ontology),
            _ => Ok(()),
        }
    } else {
        Ok(())
    }
}

/// Lines needs to be sorted
fn detect_items(numbers: &[usize], path: &str) -> HashSet<OboIdentifier> {
    let mut detected = HashSet::new();
    let mut current_item = None;
    let mut lines = BufReader::new(File::open(path).unwrap())
        .lines()
        .enumerate();

    for search_index in numbers {
        let search_index = search_index.saturating_sub(1);
        while let Some((index, line)) = lines.next() {
            let line = line.unwrap();
            if search_index < index {
                break; // Should not happen unless a single line was in there twice
            }
            if let Some(end) = line.strip_prefix("id:") {
                current_item = Some(end.trim().split_once(':').map_or_else(
                    || OboIdentifier(None, end.trim().into()),
                    |(t, v)| OboIdentifier(Some(t.trim().into()), v.trim().into()),
                ));
            }
            if index == search_index {
                if let Some(c) = &current_item {
                    detected.insert(c.clone());
                }
                break;
            }
        }
    }

    detected
}
