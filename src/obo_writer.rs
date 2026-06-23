use itertools::Itertools;
use std::{cmp::Ordering, collections::HashMap, io::Write};

use mzcv::{
    OboIdentifier, OboOntology, OboStanza, OboStanzaType, OboSynonym, OboValue, RelationType,
};

#[derive(Default)]
pub struct OboFormattingOptions {
    /// Format xref lines as if they are property values
    pub format_xref_as_property_value: bool,
    /// Sort the lines and values where possible
    pub sort: bool,
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
    write_lines(writer, &object.lines, "is_anonymous", options.sort)?;
    write_lines(writer, &object.lines, "name", options.sort)?;
    write_lines(writer, &object.lines, "namespace", options.sort)?;
    write_lines(writer, &object.lines, "alt_id", options.sort)?;
    write_def(writer, &object.definition, options.sort)?;
    write_lines(writer, &object.lines, "comment", options.sort)?;
    write_lines(writer, &object.lines, "subset", options.sort)?;
    write_synonyms(writer, &object.synonyms, options.sort)?;
    write_xref(writer, &object.xref, options)?;
    write_lines(writer, &object.lines, "builtin", options.sort)?;
    write_property_value(writer, &object.property_values, options.sort)?;
    write_is_a(writer, &object.relationship, options.sort)?;
    write_lines(writer, &object.lines, "intersection_of", options.sort)?;
    write_lines(writer, &object.lines, "union_of", options.sort)?;
    write_lines(writer, &object.lines, "equivalent_to", options.sort)?;
    write_lines(writer, &object.lines, "disjoint_from", options.sort)?;
    write_relationship(writer, &object.relationship, options.sort)?;
    write_lines(writer, &object.lines, "created_by", options.sort)?;
    write_lines(writer, &object.lines, "creation_date", options.sort)?;
    if object.obsolete {
        writeln!(writer, "is_obsolete: true")?;
    }
    write_lines(writer, &object.lines, "replaced_by", options.sort)?;
    write_lines(writer, &object.lines, "consider", options.sort)?;
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
        for (value, modifiers, comment) in
            maybe_sort(lines.iter(), |a, b| a.0.cmp(&b.0), options.sort)
        {
            escape(writer, kind, Some(':'))?;
            write!(writer, ": ")?;
            escape(writer, value, None)?;
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
    write_lines(writer, &object.lines, "is_anonymous", options.sort)?;
    write_lines(writer, &object.lines, "name", options.sort)?;
    write_lines(writer, &object.lines, "namespace", options.sort)?;
    write_lines(writer, &object.lines, "alt_id", options.sort)?;
    write_def(writer, &object.definition, options.sort)?;
    write_lines(writer, &object.lines, "comment", options.sort)?;
    write_lines(writer, &object.lines, "subset", options.sort)?;
    write_synonyms(writer, &object.synonyms, options.sort)?;
    write_xref(writer, &object.xref, options)?;
    write_property_value(writer, &object.property_values, options.sort)?;
    write_lines(writer, &object.lines, "domain", options.sort)?;
    write_lines(writer, &object.lines, "range", options.sort)?;
    write_lines(writer, &object.lines, "builtin", options.sort)?;
    write_lines(writer, &object.lines, "holds_over_chain", options.sort)?;
    write_lines(writer, &object.lines, "is_anti_symmetric", options.sort)?;
    write_lines(writer, &object.lines, "is_cyclic", options.sort)?;
    write_lines(writer, &object.lines, "is_reflexive", options.sort)?;
    write_lines(writer, &object.lines, "is_symmetric", options.sort)?;
    write_lines(writer, &object.lines, "is_transitive", options.sort)?;
    write_lines(writer, &object.lines, "is_functional", options.sort)?;
    write_lines(writer, &object.lines, "is_inverse_functional", options.sort)?;
    write_is_a(writer, &object.relationship, options.sort)?;
    write_lines(writer, &object.lines, "intersection_of", options.sort)?;
    write_lines(writer, &object.lines, "union_of", options.sort)?;
    write_lines(writer, &object.lines, "equivalent_to", options.sort)?;
    write_lines(writer, &object.lines, "disjoint_from", options.sort)?;
    write_lines(writer, &object.lines, "inverse_of", options.sort)?;
    write_lines(writer, &object.lines, "transitive_over", options.sort)?;
    write_lines(writer, &object.lines, "equivalent_to_chain", options.sort)?;
    write_lines(writer, &object.lines, "disjoint_over", options.sort)?;
    write_relationship(writer, &object.relationship, options.sort)?;
    if object.obsolete {
        writeln!(writer, "is_obsolete: true")?;
    }
    write_lines(writer, &object.lines, "created_by", options.sort)?;
    write_lines(writer, &object.lines, "creation_date", options.sort)?;
    write_lines(writer, &object.lines, "replaced_by", options.sort)?;
    write_lines(writer, &object.lines, "consider", options.sort)?;
    write_lines(writer, &object.lines, "expand_assertion_to", options.sort)?;
    write_lines(writer, &object.lines, "expand_expression_to", options.sort)?;
    write_lines(writer, &object.lines, "is_metadata_tag", options.sort)?;
    write_lines(writer, &object.lines, "is_class_level", options.sort)?;
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
        for (value, modifiers, comment) in
            maybe_sort(lines.iter(), |a, b| a.0.cmp(&b.0), options.sort)
        {
            escape(writer, kind, Some(':'))?;
            write!(writer, ": ")?;
            escape(writer, value, None)?;
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
    write_lines(writer, &object.lines, "is_anonymous", options.sort)?;
    write_lines(writer, &object.lines, "name", options.sort)?;
    write_lines(writer, &object.lines, "namespace", options.sort)?;
    write_lines(writer, &object.lines, "alt_id", options.sort)?;
    write_def(writer, &object.definition, options.sort)?;
    write_lines(writer, &object.lines, "comment", options.sort)?;
    write_lines(writer, &object.lines, "subset", options.sort)?;
    write_synonyms(writer, &object.synonyms, options.sort)?;
    write_xref(writer, &object.xref, options)?;
    write_lines(writer, &object.lines, "instance_of", options.sort)?;
    write_property_value(writer, &object.property_values, options.sort)?;
    write_is_a(writer, &object.relationship, options.sort)?;
    write_relationship(writer, &object.relationship, options.sort)?;
    write_lines(writer, &object.lines, "created_by", options.sort)?;
    write_lines(writer, &object.lines, "creation_date", options.sort)?;
    if object.obsolete {
        writeln!(writer, "is_obsolete: true")?;
    }
    write_lines(writer, &object.lines, "replaced_by", options.sort)?;
    write_lines(writer, &object.lines, "consider", options.sort)?;
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
        for (value, modifiers, comment) in
            maybe_sort(lines.iter(), |a, b| a.0.cmp(&b.0), options.sort)
        {
            escape(writer, kind, Some(':'))?;
            write!(writer, ": ")?;
            escape(writer, value, None)?;
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
    sort: bool,
) -> Result<(), std::io::Error> {
    if let Some(values) = lines.get(key) {
        for (value, modifiers, comment) in maybe_sort(values.iter(), |a, b| a.0.cmp(&b.0), sort) {
            escape(writer, key, Some(':'))?;
            write!(writer, ": ")?;
            escape(writer, value, None)?;
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
    sort: bool,
) -> Result<(), std::io::Error> {
    if let Some((def, cross_ids, modifiers, comment)) = def {
        write!(writer, "def: \"")?;
        escape(writer, def, Some('\"'))?;
        write!(writer, "\" ")?;
        write_cross_ids(writer, cross_ids, sort)?;
        write_end(writer, modifiers, comment.as_deref())?;
        writeln!(writer)?;
    }
    Ok(())
}

fn write_synonyms<W: Write>(
    writer: &mut W,
    synonyms: &[OboSynonym],
    sort: bool,
) -> Result<(), std::io::Error> {
    for synonym in maybe_sort(synonyms.iter(), |a, b| a.synonym.cmp(&b.synonym), sort) {
        write!(writer, "synonym: \"",)?;
        escape(writer, &synonym.synonym, Some('\"'))?;
        write!(
            writer,
            "\" {} ",
            synonym.scope.to_string().to_ascii_uppercase()
        )?;
        if let Some(type_name) = &synonym.type_name {
            escape(writer, type_name, None)?;
            write!(writer, " ")?;
        }
        write_cross_ids(writer, &synonym.cross_references, sort)?;
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
    for (xref, modifiers, comment) in maybe_sort(xref.iter(), |a, b| a.0.cmp(&b.0), options.sort) {
        write!(writer, "xref: ")?;
        if options.format_xref_as_property_value
            && let Some(tag) = &xref.0
        {
            escape(writer, tag, Some(':'))?;
            write!(writer, ": ")?;
            escape(writer, &xref.1, None)?;
        } else {
            escape(writer, &xref.to_string(), None)?;
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
    sort: bool,
) -> Result<(), std::io::Error> {
    for (key, values) in maybe_sort(property_values.iter(), |a, b| a.0.cmp(b.0), sort) {
        for (value, modifiers, comment) in values.iter().sorted() {
            write!(writer, "property_value: ",)?;
            escape(writer, key, Some(':'))?;
            write!(writer, ": \"")?;
            escape(writer, &value.to_string(), Some('\"'))?;
            write!(writer, "\" xsd:{}", value.datatype())?;
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
    sort: bool,
) -> Result<(), std::io::Error> {
    for (_, xref, modifiers, comment) in maybe_sort(
        relationships.iter().filter(|a| a.0 == RelationType::IsA),
        |a, b| a.1.cmp(&b.1),
        sort,
    ) {
        write!(writer, "is_a: ")?;
        escape(writer, &xref.to_string(), None)?;
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
    sort: bool,
) -> Result<(), std::io::Error> {
    for (kind, xref, modifiers, comment) in maybe_sort(
        relationships.iter().filter_map(|a| {
            if let RelationType::Other(t) = &a.0 {
                Some((t, &a.1, &a.2, &a.3))
            } else {
                None
            }
        }),
        |a, b| a.0.cmp(b.0).then(a.1.cmp(b.1)),
        sort,
    ) {
        write!(writer, "relationship: ")?;
        escape(writer, kind, None)?;
        write!(writer, " ")?;
        escape(writer, &xref.to_string(), None)?;
        write_end(writer, modifiers, comment.as_deref())?;
        writeln!(writer)?;
    }
    Ok(())
}

fn write_cross_ids<W: Write>(
    writer: &mut W,
    cross_ids: &[(Option<Box<str>>, Box<str>)],
    sort: bool,
) -> Result<(), std::io::Error> {
    write!(writer, "[")?;
    let mut first = true;
    for (tag, value) in maybe_sort(
        cross_ids.iter().unique(),
        |a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)),
        sort,
    ) {
        if first {
            first = false;
        } else {
            write!(writer, ", ")?;
        }
        if let Some(tag) = &tag {
            escape(writer, tag, Some('['))?;
            write!(writer, ":")?;
        }
        escape(writer, value, Some('['))?;
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
            escape(writer, tag, Some('{'))?;
            write!(writer, "=")?;
            escape(writer, value, Some('{'))?;
        }
        write!(writer, "}}")?;
    }
    if let Some(comment) = &comment {
        write!(writer, " ! ")?;
        escape(writer, comment, None)?;
    }
    Ok(())
}

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
            | ('\"', Some('\"'))
            | (':', Some(':')) => write!(writer, "\\")?,
            _ => (),
        }
        write!(writer, "{c}")?;
    }

    Ok(())
}

fn maybe_sort<T>(
    iter: impl Iterator<Item = T>,
    f: impl Fn(&T, &T) -> Ordering,
    sort: bool,
) -> impl Iterator<Item = T> {
    iter.sorted_by(|a, b| if sort { f(a, b) } else { Ordering::Equal })
}
