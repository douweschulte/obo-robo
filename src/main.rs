mod obo_writer;

use std::{
    collections::HashSet,
    fs::File,
    io::{BufWriter, Write},
};

use mzcore::sequence::CrossId;
use mzcv::{OboOntology, OboStanzaType, SynonymScope};

fn main() {
    let path = std::env::args()
        .skip(1)
        .next()
        .unwrap_or("/home/douwe/Downloads/psi-ms(1).obo".to_string());
    let file = OboOntology::from_file(&path).unwrap();
    let mut answer = String::new();
    println!("{} objects", file.objects.len());

    validate(&file);

    obo_writer::write(
        BufWriter::new(
            File::create(std::path::PathBuf::from(path).with_extension(".new.obo")).unwrap(),
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
            if obj.lines["name"][0].0.trim().eq_ignore_ascii_case(a) {
                println!("{obj:#?}")
            }
        }
    }
}

fn validate(ontology: &OboOntology) {
    let mut warnings = Vec::new();
    let mut names = HashSet::new();
    let mut ids = HashSet::new();

    for obj in &ontology.objects {
        if !ids.insert(obj.id.clone()) {
            warnings.push((obj.id.clone(), "Duplicate ID".to_string()));
        }
        if let Some(name) = obj.lines.get("name") {
            if name.len() != 1 {
                warnings.push((obj.id.clone(), "Too many names defined".to_string()));
            }
            if !names.insert(name[0].0.as_ref()) {
                warnings.push((obj.id.clone(), "Duplicate ID".to_string()));
            }
        } else {
            warnings.push((obj.id.clone(), "No name defined".to_string()));
        }
        if let Some((_def, cross_ids, _, _)) = &obj.definition {
            for id in cross_ids {
                let full = format!(
                    "{}{}",
                    id.0.as_ref().map_or(String::new(), |n| format!("{n}:")),
                    id.1
                );
                if let Ok(id) = mzcore::sequence::CrossId::try_from(id.clone()) {
                    if let CrossId::Other(o) = &id
                        && o.contains(':')
                    {
                        warnings.push((obj.id.clone(), format!("Unknown cross-id: {o}")));
                    } else if !matches!(
                        id,
                        CrossId::URL(_, _)
                            | CrossId::ChemicalBook(_)
                            | CrossId::ChemSpider(_)
                            | CrossId::PubChem(_)
                    ) && id.to_string() != full
                    {
                        warnings.push((
                            obj.id.clone(),
                            format!("Suspicious cross-id: '{full}', cleaned is: '{id}'"),
                        ));
                    }
                } else {
                    warnings.push((obj.id.clone(), format!("Invalid cross-id type: {full}",)));
                }
            }
        } else {
            warnings.push((obj.id.clone(), "No definition".to_string()));
        }
        for synonym in &obj.synonyms {
            if synonym.scope != SynonymScope::Exact {
                continue;
            }
            if !names.insert(synonym.synonym.as_ref()) {
                warnings.push((
                    obj.id.clone(),
                    format!("Duplicate synonym: {}", synonym.synonym),
                ));
            }
        }
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
    }

    for (id, warning) in warnings {
        println!("{id}: {warning}");
    }
}
