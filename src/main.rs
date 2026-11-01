//! Test and validate and format Obo files
mod fix_psi_mod;
mod fix_psi_ms;
mod fix_xlmod;
mod obo_writer;
mod update_psi_mod;

use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    process::ExitCode,
    str::FromStr,
};

use chrono::{Datelike, Timelike};
use clap::{Parser, ValueEnum};
use itertools::Itertools;
use mzcore::sequence::CrossId;
use mzcv::{OboIdentifier, OboOntology, OboStanza, OboStanzaType, RelationType, SynonymScope};

use crate::{
    obo_writer::{OboFormattingOptions, write_object},
    update_psi_mod::psi_mod_proper_style,
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[allow(clippy::struct_excessive_bools)]
struct Args {
    /// The action to take
    action: Action,
    /// The Obo file to open
    path: PathBuf,
    /// The location where to save any generated Obo files
    new_path: Option<PathBuf>,
    /// The name of the author if the version should be bumped (only works with fix)
    #[arg(long)]
    bump: Option<Box<str>>,
    /// Lint only the changed line numbers
    #[arg(long, value_parser=parse_line_numbers)]
    #[allow(unused_qualifications)] // Needed to trick clap
    changed_line_numbers: Option<std::vec::Vec<usize>>,
    /// Keep ordering as much as possible (lines are still reordered based on their line types but otherwise stay stable)
    #[arg(long)]
    keep_ordering: bool,
    /// Tell that this is a github PR so output stuff in MD and prevent dumping errors in the log
    #[arg(long)]
    gh_pr: bool,
    /// Fix all items even if `changed_line_numbers` is present
    #[arg(long)]
    fix_all: bool,
    /// Fix the cross-ids to use the proper tag
    #[arg(long)]
    fix_cross_ids: bool,
}

#[allow(clippy::unnecessary_wraps)] // Needed for clap
fn parse_line_numbers(value: &str) -> Result<Vec<usize>, &'static str> {
    Ok(value
        .split(',')
        .filter_map(|v| v.parse().ok())
        .unique()
        .sorted_unstable()
        .collect())
}

#[derive(ValueEnum, Clone, Debug)]
enum Action {
    /// Shows general Obo errors about the file
    Lint,
    /// Formats the Obo file with generic rules
    Fmt,
    /// Fix with ontology specific rules and formats the Obo file
    Fix,
    /// Fix PSI-MOD to follow the Obo format better to create a proper owl file
    Newfmt,
    /// Load an Obo file to search in it interactively
    Search,
}

fn main() -> ExitCode {
    let mut args = Args::parse();

    let mut file = match OboOntology::from_file(&args.path) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return if args.gh_pr { 0 } else { 3 }.into();
        }
    };
    let nl = if args.gh_pr { "<br>" } else { "\n" };

    let (version_touched, changed) = if let Some(lines) = &mut args.changed_line_numbers {
        lines.sort_unstable();
        let (version, items) = detect_items(lines, &args.path);

        print!("Changed items: {}{nl}", items.iter().sorted().join(", "));
        if version {
            print!("Version has been touched{nl}");
        }

        (version, Some(items))
    } else {
        (false, None)
    };

    match &args.action {
        Action::Lint => {
            print_errors(validate(&file, changed.as_ref()), args.gh_pr);
        }
        Action::Fmt => fmt(
            &file,
            args.new_path.as_ref().unwrap_or(&args.path),
            !args.keep_ordering,
            args.fix_cross_ids,
        ),
        Action::Fix => {
            let mut errors = validate(&file, changed.as_ref());

            errors.extend(fix(&mut file, changed.as_ref(), args.fix_all));
            let error = !errors.iter().all(|(l, _, _)| *l == ErrorLevel::Warning);
            
            print_errors(errors, args.gh_pr);

            if let Some(author) = args.bump {
                if let Some([a, b, c]) = file
                    .data_version
                    .as_ref()
                    .and_then(|v| {
                        v.split('.')
                            .take(3)
                            .map(str::parse::<usize>)
                            .collect::<Result<Vec<_>, _>>()
                            .ok()
                    })
                    .as_deref()
                {
                    if version_touched {
                        file.data_version = Some(format!("{a}.{b}.{c}").into());
                    } else {
                        file.data_version = Some(format!("{a}.{b}.{}", c + 1).into());
                        let now = chrono::Utc::now();
                        file.date = Some((
                            now.year_ce().1 as u16,
                            now.month() as u8,
                            now.day() as u8,
                            now.hour() as u8,
                            now.minute() as u8,
                        ));
                        if let Some(line) = file
                            .headers
                            .iter_mut()
                            .find(|h| h.0.eq_ignore_ascii_case("saved-by"))
                        {
                            line.1 = author;
                        } else {
                            file.headers
                                .push(("saved-by".into(), author, Vec::new(), None));
                        }
                    }
                } else {
                    eprintln!(
                        "::error::Invalid Obo version, expects the data_version to be set to a '<major>.<minor>.<patch>' version number"
                    );
                }
            }

            fmt(
                &file,
                args.new_path.as_ref().unwrap_or(&args.path),
                !args.keep_ordering,
                args.fix_cross_ids,
            );
            if !args.gh_pr && error {
                return 5.into();
            }
        }
        Action::Newfmt => {
            psi_mod_proper_style(&mut file);
            fmt(
                &file,
                args.new_path.as_ref().unwrap_or(&args.path),
                !args.keep_ordering,
                args.fix_cross_ids,
            );
        }
        Action::Search => {
            println!(
                "Version: {}, Objects: {}",
                file.version().version.unwrap_or_default(),
                file.objects.len()
            );
            while interactive_session(&file) {
                file = match OboOntology::from_file(&args.path) {
                    Ok(value) => value,
                    Err(err) => {
                        eprintln!("{err}");
                        return if args.gh_pr { 0 } else { 3 }.into();
                    }
                };
                println!(
                    "Version: {}, Objects: {}",
                    file.version().version.unwrap_or_default(),
                    file.objects.len()
                );
            }
        }
    }
    0.into()
}

fn escape(input: &str) -> String {
    input
        .replace('\'', "\'\\\'\'")
        .replace('\"', "\\\"")
        .replace('(', "\\(")
        .replace(')', "\\)")
        .replace('#', "\\#")
}

fn fmt(ontology: &OboOntology, path: impl AsRef<Path>, sort: bool, fix_cross_ids: bool) {
    obo_writer::write(
        BufWriter::new(File::create(path).unwrap()),
        ontology,
        &OboFormattingOptions {
            format_xref_as_property_value: ontology
                .headers
                .iter()
                .any(|h| h.0.as_ref() == "ontology" && h.1.as_ref() == "mod"),
            sort,
            fix_cross_ids,
        },
    )
    .unwrap();
}

fn print_errors(errors: Vec<(ErrorLevel, OboIdentifier, String)>, gh_pr: bool) {
    for (level, mut items) in errors.into_iter().map(|(l, id, msg)| (l, (id, msg))).into_group_map().into_iter().sorted_by(|a, b| a.cmp(b).reverse()) {
        items.sort_unstable();
        if gh_pr {
            print!("<details><summary>{} {}s</summary>", items.len(), level.name());
            for (id, err) in items {
                print!(" - {id}: {}<br>", escape(&err));
            }
            print!("</details>");
        } else {
            eprintln!("{} {}s:", items.len(), level.name());
            for (id, err) in items {
                eprintln!("::{}::{id}::{err}", level.github_name());
            }
        }
    } 
}

/// The level of an error
#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ErrorLevel {
    /// Just a warning, would be good to look into but can be safely ignored
    Warning,
    /// An error, likely makes this item incorrect
    Error,
    /// An error that makes the obo structure invalid
    InvalidatingError,
}

impl ErrorLevel {
    const fn github_name(self) -> &'static str {
        match self {
            Self::Warning => "::notice::",
            Self::Error | Self::InvalidatingError => "::error::",
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Warning => "Warning",
            Self::Error => "Error",
            Self::InvalidatingError => "Invalidating Error"
        }
    }
}

fn validate(
    ontology: &OboOntology,
    subset: Option<&HashSet<OboIdentifier>>,
) -> Vec<(ErrorLevel, OboIdentifier, String)> {
    let mut warnings = Vec::new();
    let mut names = HashSet::new();
    let mut definitions = HashSet::new();
    let mut ids = HashSet::new();

    let defined_subsets: HashSet<&str> = ontology
        .headers
        .iter()
        .filter(|h| h.0.as_ref() == "subsetdef")
        .filter_map(|h| h.1.split(' ').next())
        .collect();
    let defined_synonyms: HashSet<&str> = ontology
        .headers
        .iter()
        .filter(|h| h.0.as_ref() == "synonymtypedef")
        .filter_map(|h| h.1.split(' ').next())
        .collect();
    let defined_typedefs: HashSet<String> = ontology
        .objects
        .iter()
        .filter(|o| o.stanza_type == OboStanzaType::Typedef)
        .flat_map(|o| [o.id.to_string(), o.name().to_string()])
        .collect();

    for obj in &ontology.objects {
        if subset.is_some_and(|s| !s.contains(&obj.id)) {
            continue;
        }

        // Check for duplicate IDs
        if !ids.insert(obj.id.clone()) {
            warnings.push((
                ErrorLevel::InvalidatingError,
                obj.id.clone(),
                "Duplicate ID".to_string(),
            ));
        }

        // Check for duplicate names
        if let Some(name) = obj.lines.get("name") {
            if name.len() != 1 {
                warnings.push((
                    ErrorLevel::InvalidatingError,
                    obj.id.clone(),
                    "Too many names defined".to_string(),
                ));
            }
            if !names.insert(name[0].0.as_ref()) {
                warnings.push((
                    ErrorLevel::InvalidatingError,
                    obj.id.clone(),
                    format!("Duplicate name: '{}'", name[0].0),
                ));
            }
        } else {
            warnings.push((
                ErrorLevel::Warning,
                obj.id.clone(),
                "No name defined".to_string(),
            ));
        }

        if let Some((def, cross_ids, _, _)) = &obj.definition {
            // Check for duplicate definitions
            if !definitions.insert(def.as_ref()) {
                warnings.push((
                    ErrorLevel::Warning,
                    obj.id.clone(),
                    format!("Duplicate definition: '{def}'"),
                ));
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
                        warnings.push((
                            ErrorLevel::Warning,
                            obj.id.clone(),
                            format!("Unknown cross-id: {o}"),
                        ));
                    }
                } else {
                    warnings.push((
                        ErrorLevel::Warning,
                        obj.id.clone(),
                        format!("Invalid cross-id type: {full}"),
                    ));
                }
            }
        } else {
            warnings.push((
                ErrorLevel::Warning,
                obj.id.clone(),
                "No definition".to_string(),
            ));
        }

        // Check for duplicate synonyms
        for synonym in &obj.synonyms {
            if synonym.scope != SynonymScope::Exact {
                continue;
            }
            if !names.insert(synonym.synonym.as_ref()) {
                warnings.push((
                    ErrorLevel::Warning,
                    obj.id.clone(),
                    format!(
                        "Duplicate exact synonym (case insensitive): '{}'",
                        synonym.synonym
                    ),
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
                    warnings.push((
                        ErrorLevel::Error,
                        obj.id.clone(),
                        format!("Invalid line type: {line}"),
                    ));
                }
            }
        }

        if obj.obsolete && !obj.relationship.is_empty() {
            warnings.push((
                ErrorLevel::InvalidatingError,
                obj.id.clone(),
                "This obsoleted term has relationships".to_string(),
            ));
        }

        // Check for suspicious comments
        let mut parent_stack = Vec::new();
        for (t, rel, _, comment) in &obj.relationship {
            if let RelationType::Other(reltype) = t
                && !defined_typedefs.contains(reltype.as_ref())
            {
                warnings.push((
                    ErrorLevel::InvalidatingError,
                    obj.id.clone(),
                    format!("Undefined relationship type was referenced: '{reltype}'"),
                ));
            }
            if rel.0.as_ref().is_some_and(|t| t.as_ref() == "xsd") {
                continue;
            }
            if let Some(relation) = ontology.objects.iter().find(|o| o.id == *rel) {
                if let Some(comment) = comment
                    && *comment != relation.lines["name"][0].0
                {
                    warnings.push((
                            ErrorLevel::Error, 
                            obj.id.clone(),
                            format!("This relationship comment looks suspicious: name is '{}' comment is '{comment}'", relation.lines["name"][0].0),
                        ));
                }
                if *t == RelationType::IsA {
                    parent_stack.push(relation);
                }

                if relation.obsolete {
                    warnings.push((
                        ErrorLevel::Error, 
                        obj.id.clone(),
                        format!(
                            "This related term is marked obsolete: '{}|{}'",
                            relation.id, relation.lines["name"][0].0
                        ),
                    ));
                }
            } else {
                warnings.push((
                    ErrorLevel::InvalidatingError, 
                    obj.id.clone(),
                    format!("Referenced relation does not exist in this file: {rel}"),
                ));
            }
        }

        // Check for duplicated inherited values
        // while let Some(ancestor) = parent_stack.pop() {
        //     for (key, values) in &ancestor.property_values {
        //         if let Some(own_values) = obj.property_values.get(key) {
        //             for own_value in own_values {
        //                 if values.iter().any(|v| v.0 == own_value.0) {
        //                     warnings.push((
        //                         obj.id.clone(),
        //                         format!("Overwrote an ancestral property value with the same value: '{}' from ancestor: {}", own_value.0, ancestor.id),
        //                     ));
        //                 }
        //             }
        //         }
        //     }
        //     for (id, _, _) in &ancestor.xref {
        //         if obj.xref.iter().any(|v| v.0 == *id) {
        //             warnings.push((
        //                 obj.id.clone(),
        //                 format!("Overwrote an ancestral xref with the same value: '{}' from ancestor: {}", id, ancestor.id),
        //             ));
        //         }
        //     }
        //     for (t, rel, _, _) in &ancestor.relationship {
        //         if *t == RelationType::IsA
        //             && let Some(relation) = ontology.objects.iter().find(|o| o.id == *rel)
        //         {
        //             parent_stack.push(relation);
        //         }
        //     }
        // }

        // Check for subsetdefs
        if let Some(lines) = obj.lines.get("subset") {
            for line in lines {
                if !defined_subsets.contains(line.0.as_ref()) {
                    warnings.push((
                        ErrorLevel::InvalidatingError, 
                        obj.id.clone(),
                        format!("Undefined subset was referenced: '{}'", line.0),
                    ));
                }
            }
        }
        for synonym in &obj.synonyms {
            if let Some(t) = synonym.type_name.as_deref()
                && !defined_synonyms.contains(t)
            {
                warnings.push((
                    ErrorLevel::InvalidatingError, 
                    obj.id.clone(),
                    format!("Undefined synonym was referenced: '{t}'"),
                ));
            }
        }
    }

    warnings
}

fn fix(
    ontology: &mut OboOntology,
    subset: Option<&HashSet<OboIdentifier>>,
    fix_all: bool,
) -> Vec<(ErrorLevel, OboIdentifier, String)> {
    if let Some((_, name, _, _)) = ontology.headers.iter().find(|h| h.0.as_ref() == "ontology") {
        match name.as_ref() {
            "xlmod" => fix_xlmod::fix_xlmod(ontology, subset, fix_all),
            "mod" => fix_psi_mod::fix_psi_mod(ontology, subset, fix_all),
            "ms" => fix_psi_ms::fix_psi_ms(ontology, subset, fix_all),
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    }
}

/// Lines need to be sorted. The output is a boolean indicating if any of the version lines have been altered, and all touched items.
fn detect_items(numbers: &[usize], path: impl AsRef<Path>) -> (bool, HashSet<OboIdentifier>) {
    let mut version_touched = false;
    let mut detected = HashSet::new();
    let mut current_item = None;
    let mut lines = BufReader::new(File::open(path).unwrap())
        .lines()
        .enumerate();

    for search_index in numbers {
        let search_index = search_index.saturating_sub(1);
        for (index, line) in lines.by_ref() {
            let line = line.unwrap();
            let mut version_line = false;
            if search_index < index {
                break; // Should not happen unless a single line was in there twice
            }
            if let Some(end) = line.strip_prefix("id:") {
                current_item = Some(end.trim().split_once(':').map_or_else(
                    || OboIdentifier(None, end.trim().into()),
                    |(t, v)| OboIdentifier(Some(t.trim().into()), v.trim().into()),
                ));
            } else if (line.starts_with("data-version:")
                || line.starts_with("saved-by:")
                || line.starts_with("date:"))
                && current_item.is_none()
            {
                version_line = true;
            }
            if index == search_index {
                if let Some(c) = &current_item {
                    detected.insert(c.clone());
                } else if version_line {
                    version_touched = true;
                }
                break;
            }
        }
    }

    (version_touched, detected)
}

fn interactive_session(ontology: &OboOntology) -> bool {
    let name_index: HashMap<String, usize> = ontology
        .objects
        .iter()
        .enumerate()
        .map(|(i, o)| (o.lines["name"][0].0.trim().to_ascii_lowercase(), i))
        .collect();
    let synonym_index: HashMap<String, usize> = ontology
        .objects
        .iter()
        .enumerate()
        .flat_map(|(i, o)| {
            o.synonyms
                .iter()
                .map(move |s| (s.synonym.trim().to_ascii_lowercase(), i))
        })
        .collect();
    let word_index: HashMap<String, Vec<usize>> = ontology
        .objects
        .iter()
        .enumerate()
        .flat_map(|(i, o)| {
            get_words(&o.lines["name"][0].0)
                .into_iter()
                .chain(o.synonyms.iter().flat_map(move |s| get_words(&s.synonym)))
                .chain(o.definition.iter().flat_map(|d| get_words(&d.0)))
                .map(move |w| (w, i))
        })
        .into_group_map();
    let id_index: HashMap<OboIdentifier, usize> = ontology
        .objects
        .iter()
        .enumerate()
        .map(|(i, o)| (o.id.clone(), i))
        .collect();
    let ancestors_index = ontology
        .objects
        .iter()
        .flat_map(|o| {
            o.relationship
                .iter()
                .map(|r| (o.id.clone(), (r.1.clone(), r.0.clone())))
        })
        .into_group_map();
    let children_index = ontology
        .objects
        .iter()
        .flat_map(|o| {
            o.relationship
                .iter()
                .map(|r| (r.1.clone(), (o.id.clone(), r.0.clone())))
        })
        .into_group_map();

    let mut answer = String::new();
    loop {
        answer.clear();
        println!();
        print!(">");
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(&mut answer).unwrap();

        let Ok(command) = InteractiveCommand::from_str(answer.trim()) else {
            eprint!("Invalid command");
            continue;
        };

        match command {
            InteractiveCommand::Quit => return false,
            InteractiveCommand::Refresh => return true,
            InteractiveCommand::Ancestors(depth, term) => {
                if let Some(id) = OboIdentifier::from_str(&term)
                    .ok()
                    .and_then(|id| id_index.get(&id))
                    .or_else(|| name_index.get(&term))
                    .or_else(|| synonym_index.get(&term))
                {
                    draw_tree(ontology, *id, &id_index, &ancestors_index, depth);
                } else {
                    eprintln!("Could not find term '{term}'");
                }
            }
            InteractiveCommand::Children(depth, term) => {
                if let Some(id) = OboIdentifier::from_str(&term)
                    .ok()
                    .and_then(|id| id_index.get(&id))
                    .or_else(|| name_index.get(&term))
                    .or_else(|| synonym_index.get(&term))
                {
                    draw_tree(ontology, *id, &id_index, &children_index, depth);
                } else {
                    eprintln!("Could not find term '{term}'");
                }
            }
            InteractiveCommand::Search(term) => {
                let lowercase = term.to_ascii_lowercase();
                if let Some(id) = OboIdentifier::from_str(&term)
                    .ok()
                    .and_then(|id| id_index.get(&id))
                    .or_else(|| name_index.get(&lowercase))
                    .or_else(|| synonym_index.get(&lowercase))
                {
                    let o = &ontology.objects[*id];
                    write_object(std::io::stdout(), o, &OboFormattingOptions::default()).unwrap();
                    println!(
                        "=> {} direct children",
                        children_index.get(&o.id).map_or_default(Vec::len)
                    );
                } else {
                    let mut matches = false;
                    for (_, v, _) in name_index
                        .iter()
                        .chain(synonym_index.iter())
                        .filter_map(|(n, i)| {
                            let d = mzcv::text::levenshtein_distance(n, &lowercase);
                            if d < 5 { Some((n, i, d)) } else { None }
                        })
                        .k_smallest_by(10, |a, b| a.2.cmp(&b.2))
                    {
                        let o = &ontology.objects[*v];
                        println!(
                            "{}={}",
                            o.curie(),
                            o.definition.as_ref().map_or_default(|o| o.0.as_ref())
                        );
                        matches = true;
                    }
                    if !matches {
                        for v in get_words(&term)
                            .iter()
                            .flat_map(|w| word_index.get(w).map_or_default(Vec::as_slice))
                            .copied()
                            .unique()
                            .sorted()
                        {
                            let o = &ontology.objects[v];
                            println!(
                                "{}={}",
                                o.curie(),
                                o.definition.as_ref().map_or_default(|o| o.0.as_ref())
                            );
                        }
                    }
                }
            }
        }
    }
}

enum InteractiveCommand {
    Quit,
    Refresh,
    Ancestors(Option<usize>, String),
    Children(Option<usize>, String),
    Search(String),
}

impl FromStr for InteractiveCommand {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("quit") || s.eq_ignore_ascii_case("q") {
            return Ok(Self::Quit);
        }
        if s.eq_ignore_ascii_case("refresh") || s.eq_ignore_ascii_case("r") {
            return Ok(Self::Refresh);
        }
        if let Some(a) = s.split(' ').next() {
            let (depth, offset) = if let Some(arg) = s[a.len()..].trim().split(' ').next()
                && let Some(d) = arg.strip_prefix("d=")
                && let Ok(depth) = d.parse::<usize>()
            {
                (Some(depth), s.substr_range(arg).map_or(0, |r| r.end))
            } else {
                (None, a.len())
            };
            if a.eq_ignore_ascii_case("ancestors") {
                return Ok(Self::Ancestors(depth, s[offset..].trim().to_string()));
            }
            if a.eq_ignore_ascii_case("children") {
                return Ok(Self::Children(depth, s[offset..].trim().to_string()));
            }
        }
        Ok(Self::Search(s.to_string()))
    }
}

fn get_words(w: &str) -> Vec<String> {
    w.split_ascii_whitespace()
        .map(str::to_ascii_lowercase)
        .collect()
}

fn draw_tree(
    ontology: &OboOntology,
    term: usize,
    id_index: &HashMap<OboIdentifier, usize>,
    connections: &HashMap<OboIdentifier, Vec<(OboIdentifier, RelationType)>>,
    depth: Option<usize>,
) {
    let tree = Tree::from_stanza(
        ontology,
        &ontology.objects[term].id,
        &RelationType::IsA,
        id_index,
        connections,
        depth,
    );
    tree.draw(
        &[],
        if connections
            .get(&ontology.objects[term].id)
            .is_some_and(|c| !c.is_empty())
        {
            '╤'
        } else {
            '═'
        },
        &mut HashSet::new(),
    );
}

struct Tree {
    name: String,
    relation: RelationType,
    branches: Vec<Self>,
}

impl Tree {
    fn from_stanza(
        ontology: &OboOntology,
        term: &OboIdentifier,
        relation: &RelationType,
        id_index: &HashMap<OboIdentifier, usize>,
        connections: &HashMap<OboIdentifier, Vec<(OboIdentifier, RelationType)>>,
        depth: Option<usize>,
    ) -> Self {
        if let Some(obj) = id_index.get(term)
            && let Some(object) = ontology.objects.get(*obj)
        {
            Self {
                name: object.curie(),
                relation: relation.clone(),
                branches: if depth.is_none_or(|d| d > 0) {
                    {
                        connections
                            .get(term)
                            .iter()
                            .flat_map(|c| {
                                c.iter().map(|(c, r)| {
                                    Self::from_stanza(
                                        ontology,
                                        c,
                                        r,
                                        id_index,
                                        connections,
                                        depth.map(|d| d - 1),
                                    )
                                })
                            })
                            .collect()
                    }
                } else {
                    Vec::new()
                },
            }
        } else {
            Self {
                name: term.to_string(),
                relation: relation.clone(),
                branches: Vec::new(),
            }
        }
    }

    fn draw(&self, previous_levels: &[BranchState], indicator: char, drawn: &mut HashSet<String>) {
        print!(
            "{}{indicator}{}{}",
            previous_levels
                .iter()
                .map(|branch| branch.char())
                .collect::<String>(),
            match &self.relation {
                RelationType::IsA => "╸".to_string(),
                RelationType::Other(rel) => format!("[{rel}]"),
            },
            self.name
        );
        if drawn.contains(&self.name) {
            println!(" (*)");
        } else {
            println!();
            drawn.insert(self.name.clone());

            for (i, child) in self.branches.iter().enumerate() {
                let mut branches = previous_levels.iter().map(|b| b.next()).collect::<Vec<_>>();
                branches.push(if i == self.branches.len() - 1 {
                    BranchState::Last
                } else {
                    BranchState::MiddleConnection
                });
                child.draw(
                    &branches,
                    if child.branches.is_empty() {
                        '─'
                    } else {
                        '┬'
                    },
                    drawn,
                );
            }
        }
    }
}

#[derive(Clone, Copy)]
enum BranchState {
    MiddleConnection,
    Middle,
    Last,
    Empty,
}

impl BranchState {
    const fn next(self) -> Self {
        match self {
            Self::MiddleConnection | Self::Middle => Self::Middle,
            Self::Last | Self::Empty => Self::Empty,
        }
    }

    const fn char(self) -> char {
        match self {
            Self::MiddleConnection => '├',
            Self::Middle => '│',
            Self::Last => '└',
            Self::Empty => ' ',
        }
    }
}

trait OboExtended {
    fn name(&self) -> &str;
    fn curie(&self) -> String;
}

impl OboExtended for OboStanza {
    fn name(&self) -> &str {
        &self.lines["name"][0].0
    }
    fn curie(&self) -> String {
        format!("{}|{}", self.id, self.name())
    }
}
