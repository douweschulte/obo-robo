use itertools::Itertools;
use std::{collections::HashMap, io::Write};

use mzcv::{
    OboIdentifier, OboOntology, OboStanza, OboStanzaType, OboSynonym, OboValue, RelationType,
};

#[derive(Default)]
pub struct OboFormattingOptions {
    /// Format xref lines as if they are property values
    pub format_xref_as_property_value: bool,
}

// TODO: escape all written values (e.g. regular expression names)
pub fn write<W: Write>(
    mut writer: W,
    obo: &OboOntology,
    options: &OboFormattingOptions,
) -> Result<(), std::io::Error> {
    if let Some((_, version, _, _)) = obo
        .headers
        .iter()
        .find(|(t, ..)| t.eq_ignore_ascii_case("format-version"))
    {
        writeln!(writer, "format-version: {version}")?;
    }

    if let Some(version) = &obo.data_version {
        writeln!(writer, "data-version: {version}")?;
    }
    if let Some((y, m, d, h, mi)) = obo.date {
        writeln!(writer, "date: {d:02}:{m:02}:{y:04} {h:02}:{mi:02}")?;
    }

    for (tag, value, _modifiers, _comment) in &obo.headers {
        if !tag.eq_ignore_ascii_case("format-version") {
            writeln!(writer, "{tag}: {value}")?;
        }
    }

    for object in &obo.objects {
        writeln!(writer)?;
        write_object(&mut writer, object, options)?;
    }

    Ok(())
}

pub fn write_object<W: Write>(
    mut writer: W,
    object: &OboStanza,
    options: &OboFormattingOptions,
) -> Result<(), std::io::Error> {
    writeln!(
        writer,
        "[{}]",
        match object.stanza_type {
            OboStanzaType::Typedef => "Typedef",
            OboStanzaType::Term => "Term",
            OboStanzaType::Instance => "Instance",
        }
    )?;
    match object.stanza_type {
        OboStanzaType::Typedef => write_typedef(&mut writer, object, options),
        OboStanzaType::Term => write_term(&mut writer, object, options),
        OboStanzaType::Instance => write_instance(&mut writer, object, options),
    }
}

fn write_term<W: Write>(
    writer: &mut W,
    object: &OboStanza,
    options: &OboFormattingOptions,
) -> Result<(), std::io::Error> {
    writeln!(writer, "id: {}", object.id)?;
    write_lines(writer, &object.lines, "is_anonymous")?;
    write_lines(writer, &object.lines, "name")?;
    write_lines(writer, &object.lines, "namespace")?;
    write_lines(writer, &object.lines, "alt_id")?;
    write_def(writer, &object.definition)?;
    write_lines(writer, &object.lines, "comment")?;
    write_lines(writer, &object.lines, "subset")?;
    write_synonyms(writer, &object.synonyms)?;
    write_xref(writer, &object.xref, options)?;
    write_lines(writer, &object.lines, "builtin")?;
    write_property_value(writer, &object.property_values)?;
    write_is_a(writer, &object.relationship)?;
    write_lines(writer, &object.lines, "intersection_of")?;
    write_lines(writer, &object.lines, "union_of")?;
    write_lines(writer, &object.lines, "equivalent_to")?;
    write_lines(writer, &object.lines, "disjoint_from")?;
    write_relationship(writer, &object.relationship)?;
    write_lines(writer, &object.lines, "created_by")?;
    write_lines(writer, &object.lines, "creation_date")?;
    if object.obsolete {
        writeln!(writer, "is_obsolete: true")?;
    }
    write_lines(writer, &object.lines, "replaced_by")?;
    write_lines(writer, &object.lines, "consider")?;
    for (kind, lines) in object.lines.iter().sorted_by_key(|(k, _)| *k) {
        if [
            "id",
            "is_anonymous",
            "name",
            "namespace",
            "alt_id",
            "def",
            "comment",
            "subset",
            "synonym",
            "xref",
            "builtin",
            "property_value",
            "is_a",
            "intersection_of",
            "union_of",
            "equivalent_to",
            "disjoint_from",
            "relationship",
            "created_by",
            "creation_date",
            "is_obsolete",
            "replaced_by",
            "consider ",
        ]
        .contains(&kind.as_ref())
        {
            continue;
        }
        for (value, modifiers, comment) in lines.iter().sorted_by_key(|l| &l.0) {
            write!(writer, "{kind}: {value}")?;
            write_end(writer, modifiers, comment.as_deref())?;
            writeln!(writer)?;
        }
    }
    Ok(())
}

fn write_typedef<W: Write>(
    writer: &mut W,
    object: &OboStanza,
    options: &OboFormattingOptions,
) -> Result<(), std::io::Error> {
    writeln!(writer, "id: {}", object.id)?;
    write_lines(writer, &object.lines, "is_anonymous")?;
    write_lines(writer, &object.lines, "name")?;
    write_lines(writer, &object.lines, "namespace")?;
    write_lines(writer, &object.lines, "alt_id")?;
    write_def(writer, &object.definition)?;
    write_lines(writer, &object.lines, "comment")?;
    write_lines(writer, &object.lines, "subset")?;
    write_synonyms(writer, &object.synonyms)?;
    write_xref(writer, &object.xref, options)?;
    write_property_value(writer, &object.property_values)?;
    write_lines(writer, &object.lines, "domain")?;
    write_lines(writer, &object.lines, "range")?;
    write_lines(writer, &object.lines, "builtin")?;
    write_lines(writer, &object.lines, "holds_over_chain")?;
    write_lines(writer, &object.lines, "is_anti_symmetric")?;
    write_lines(writer, &object.lines, "is_cyclic")?;
    write_lines(writer, &object.lines, "is_reflexive")?;
    write_lines(writer, &object.lines, "is_symmetric")?;
    write_lines(writer, &object.lines, "is_transitive")?;
    write_lines(writer, &object.lines, "is_functional")?;
    write_lines(writer, &object.lines, "is_inverse_functional")?;
    write_is_a(writer, &object.relationship)?;
    write_lines(writer, &object.lines, "intersection_of")?;
    write_lines(writer, &object.lines, "union_of")?;
    write_lines(writer, &object.lines, "equivalent_to")?;
    write_lines(writer, &object.lines, "disjoint_from")?;
    write_lines(writer, &object.lines, "inverse_of")?;
    write_lines(writer, &object.lines, "transitive_over")?;
    write_lines(writer, &object.lines, "equivalent_to_chain")?;
    write_lines(writer, &object.lines, "disjoint_over")?;
    write_relationship(writer, &object.relationship)?;
    if object.obsolete {
        writeln!(writer, "is_obsolete: true")?;
    }
    write_lines(writer, &object.lines, "created_by")?;
    write_lines(writer, &object.lines, "creation_date")?;
    write_lines(writer, &object.lines, "replaced_by")?;
    write_lines(writer, &object.lines, "consider")?;
    write_lines(writer, &object.lines, "expand_assertion_to")?;
    write_lines(writer, &object.lines, "expand_expression_to")?;
    write_lines(writer, &object.lines, "is_metadata_tag")?;
    write_lines(writer, &object.lines, "is_class_level")?;
    for (kind, lines) in object.lines.iter().sorted_by_key(|(k, _)| *k) {
        if [
            "id",
            "is_anonymous",
            "name",
            "namespace",
            "alt_id",
            "def",
            "comment",
            "subset",
            "synonym",
            "xref",
            "property_value",
            "domain",
            "range",
            "builtin",
            "holds_over_chain",
            "is_anti_symmetric",
            "is_cyclic",
            "is_reflexive",
            "is_symmetric",
            "is_transitive",
            "is_functional",
            "is_inverse_functional",
            "is_a",
            "intersection_of",
            "union_of",
            "equivalent_to",
            "disjoint_from",
            "inverse_of",
            "transitive_over",
            "equivalent_to_chain",
            "disjoint_over",
            "relationship",
            "is_obsolete",
            "created_by",
            "creation_date",
            "replaced_by",
            "consider",
            "expand_assertion_to",
            "expand_expression_to",
            "is_metadata_tag",
            "is_class_level ",
        ]
        .contains(&kind.as_ref())
        {
            continue;
        }
        for (value, modifiers, comment) in lines.iter().sorted_by_key(|l| &l.0) {
            write!(writer, "{kind}: {value}")?;
            write_end(writer, modifiers, comment.as_deref())?;
            writeln!(writer)?;
        }
    }
    Ok(())
}

fn write_instance<W: Write>(
    writer: &mut W,
    object: &OboStanza,
    options: &OboFormattingOptions,
) -> Result<(), std::io::Error> {
    writeln!(writer, "id: {}", object.id)?;
    write_lines(writer, &object.lines, "is_anonymous")?;
    write_lines(writer, &object.lines, "name")?;
    write_lines(writer, &object.lines, "namespace")?;
    write_lines(writer, &object.lines, "alt_id")?;
    write_def(writer, &object.definition)?;
    write_lines(writer, &object.lines, "comment")?;
    write_lines(writer, &object.lines, "subset")?;
    write_synonyms(writer, &object.synonyms)?;
    write_xref(writer, &object.xref, options)?;
    write_lines(writer, &object.lines, "instance_of")?;
    write_property_value(writer, &object.property_values)?;
    write_is_a(writer, &object.relationship)?;
    write_relationship(writer, &object.relationship)?;
    write_lines(writer, &object.lines, "created_by")?;
    write_lines(writer, &object.lines, "creation_date")?;
    if object.obsolete {
        writeln!(writer, "is_obsolete: true")?;
    }
    write_lines(writer, &object.lines, "replaced_by")?;
    write_lines(writer, &object.lines, "consider")?;
    for (kind, lines) in object.lines.iter().sorted_by_key(|(k, _)| *k) {
        if [
            "id",
            "is_anonymous",
            "name",
            "namespace",
            "alt_id",
            "def",
            "comment",
            "subset",
            "synonym",
            "xref",
            "instance_of",
            "property_value",
            "relationship",
            "created_by",
            "creation_date",
            "is_obsolete",
            "replaced_by",
            "consider ",
        ]
        .contains(&kind.as_ref())
        {
            continue;
        }
        for (value, modifiers, comment) in lines.iter().sorted_by_key(|l| &l.0) {
            write!(writer, "{kind}: {value}")?;
            write_end(writer, modifiers, comment.as_deref())?;
            writeln!(writer)?;
        }
    }
    Ok(())
}

fn write_lines<W: Write>(
    writer: &mut W,
    lines: &HashMap<Box<str>, Vec<(Box<str>, Vec<(Box<str>, Box<str>)>, Option<Box<str>>)>>,
    key: &str,
) -> Result<(), std::io::Error> {
    if let Some(values) = lines.get(key) {
        for (value, modifiers, comment) in values.iter().sorted_by_key(|l| &l.0) {
            write!(writer, "{key}: {value}")?;
            write_end(writer, modifiers, comment.as_deref())?;
            writeln!(writer)?;
        }
    }
    Ok(())
}

fn write_def<W: Write>(
    writer: &mut W,
    def: &Option<(
        Box<str>,
        Vec<(Option<Box<str>>, Box<str>)>,
        Vec<(Box<str>, Box<str>)>,
        Option<Box<str>>,
    )>,
) -> Result<(), std::io::Error> {
    if let Some((def, cross_ids, modifiers, comment)) = def {
        write!(writer, "def: \"")?;
        escape(writer, def, Some('\"'))?;
        write!(writer, "\" ")?;
        write_cross_ids(writer, cross_ids)?;
        write_end(writer, modifiers, comment.as_deref())?;
        writeln!(writer)?;
    }
    Ok(())
}

fn write_synonyms<W: Write>(writer: &mut W, synonyms: &[OboSynonym]) -> Result<(), std::io::Error> {
    for synonym in synonyms.iter().sorted_by_key(|s| &s.synonym) {
        write!(
            writer,
            "synonym: \"{}\" {} ",
            synonym.synonym,
            synonym.scope.to_string().to_ascii_uppercase()
        )?;
        if let Some(type_name) = &synonym.type_name {
            write!(writer, "{type_name} ")?;
        }
        write_cross_ids(writer, &synonym.cross_references)?;
        write_end(
            writer,
            &synonym.trailing_modifiers,
            synonym.comment.as_deref(),
        )?;
        writeln!(writer)?;
    }
    Ok(())
}

fn write_xref<W: Write>(
    writer: &mut W,
    xref: &[(OboIdentifier, Vec<(Box<str>, Box<str>)>, Option<Box<str>>)],
    options: &OboFormattingOptions,
) -> Result<(), std::io::Error> {
    for (xref, modifiers, comment) in xref.iter().sorted_by_key(|x| &x.0) {
        write!(writer, "xref: ")?;
        if options.format_xref_as_property_value
            && let Some(tag) = &xref.0
        {
            write!(writer, "{tag}: {}", xref.1)?;
        } else {
            write!(writer, "{xref}")?;
        }

        write_end(writer, modifiers, comment.as_deref())?;
        writeln!(writer)?;
    }
    Ok(())
}

fn write_property_value<W: Write>(
    writer: &mut W,
    property_values: &HashMap<
        Box<str>,
        Vec<(OboValue, Vec<(Box<str>, Box<str>)>, Option<Box<str>>)>,
    >,
) -> Result<(), std::io::Error> {
    for (key, values) in property_values.iter().sorted_by_key(|(k, _)| *k) {
        for (value, modifiers, comment) in values.iter().sorted() {
            write!(
                writer,
                "property_value: {key}: \"{value}\" xsd:{}",
                value.datatype()
            )?;
            write_end(writer, modifiers, comment.as_deref())?;
            writeln!(writer)?;
        }
    }
    Ok(())
}

fn write_is_a<W: Write>(
    writer: &mut W,
    relationships: &[(
        RelationType,
        OboIdentifier,
        Vec<(Box<str>, Box<str>)>,
        Option<Box<str>>,
    )],
) -> Result<(), std::io::Error> {
    for (_, xref, modifiers, comment) in relationships
        .iter()
        .filter(|a| a.0 == RelationType::IsA)
        .sorted_by(|a, b| a.1.cmp(&b.1))
    {
        write!(writer, "is_a: {xref}")?;
        write_end(writer, modifiers, comment.as_deref())?;
        writeln!(writer)?;
    }
    Ok(())
}

fn write_relationship<W: Write>(
    writer: &mut W,
    relationships: &[(
        RelationType,
        OboIdentifier,
        Vec<(Box<str>, Box<str>)>,
        Option<Box<str>>,
    )],
) -> Result<(), std::io::Error> {
    for (kind, xref, modifiers, comment) in relationships
        .iter()
        .filter_map(|a| {
            if let RelationType::Other(t) = &a.0 {
                Some((t, &a.1, &a.2, &a.3))
            } else {
                None
            }
        })
        .sorted_by(|a, b| a.0.cmp(b.0).then(a.1.cmp(b.1)))
    {
        write!(writer, "relationship: {kind} {xref}")?;
        write_end(writer, modifiers, comment.as_deref())?;
        writeln!(writer)?;
    }
    Ok(())
}

fn write_cross_ids<W: Write>(
    writer: &mut W,
    cross_ids: &[(Option<Box<str>>, Box<str>)],
) -> Result<(), std::io::Error> {
    write!(writer, "[")?;
    let mut first = true;
    for (tag, value) in cross_ids
        .iter()
        .unique()
        .sorted_unstable_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)))
    {
        if first {
            first = false;
        } else {
            write!(writer, ", ")?;
        }
        if let Some(tag) = &tag {
            write!(writer, "{tag}:")?;
        }
        write!(writer, "{value}")?;
    }
    write!(writer, "]")?;
    Ok(())
}

fn write_end<W: Write>(
    writer: &mut W,
    trailing_modifiers: &[(Box<str>, Box<str>)],
    comment: Option<&str>,
) -> Result<(), std::io::Error> {
    if !trailing_modifiers.is_empty() {
        write!(writer, " {{")?;
        let mut first = true;
        for (tag, value) in trailing_modifiers {
            if first {
                first = false;
            } else {
                write!(writer, ", ")?;
            }
            write!(writer, "{tag}={value}")?;
        }
        write!(writer, "}}")?;
    }
    if let Some(comment) = &comment {
        write!(writer, " ! {comment}")?;
    }
    Ok(())
}

// TODO: figure out which characters to escape, as this depends on context, check stuff like: UO:0000268
fn escape<W: Write>(
    writer: &mut W,
    text: &str,
    enclosed: Option<char>,
) -> Result<(), std::io::Error> {
    for c in text.chars() {
        match (c, enclosed) {
            ('\\', _)
            | ('!', None)
            | ('[' | ']', Some('['))
            | ('{' | '}', Some('{'))
            | ('\"', Some('\"')) => write!(writer, "\\")?,
            _ => (),
        }
        write!(writer, "{c}")?;
    }

    Ok(())
}
