use itertools::Itertools;
use std::io::Write;

use mzcv::{OboOntology, OboStanzaType, RelationType};

// TODO: escape all written values (e.g. regular expression names)
pub fn write<W: Write>(mut writer: W, obo: &OboOntology) -> Result<(), std::io::Error> {
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

    for obj in &obo.objects {
        writeln!(writer)?;
        writeln!(
            writer,
            "[{}]",
            match obj.stanza_type {
                OboStanzaType::Typedef => "Typedef",
                OboStanzaType::Term => "Term",
                OboStanzaType::Instance => "Instance",
            }
        )?;
        writeln!(writer, "id: {}", obj.id)?;
        if let Some((_, names)) = obj
            .lines
            .iter()
            .find(|(t, ..)| t.eq_ignore_ascii_case("name"))
        {
            write!(writer, "name: ")?;
            escape(&mut writer, &names[0].0, None)?;
            writeln!(writer)?;
        }
        if let Some((def, cross_ids, modifiers, comment)) = &obj.definition {
            write!(writer, "def: \"")?;
            escape(&mut writer, def, Some('\"'))?;
            write!(writer, "\" ")?;
            write_cross_ids(&mut writer, cross_ids)?;
            write_end(&mut writer, modifiers, comment)?;
            writeln!(writer)?;
        }
        for synonym in obj.synonyms.iter().sorted_by_key(|s| &s.synonym) {
            write!(
                writer,
                "synonym: \"{}\" {} ",
                synonym.synonym,
                synonym.scope.to_string().to_ascii_uppercase()
            )?;
            if let Some(type_name) = &synonym.type_name {
                write!(writer, "{type_name} ")?;
            }
            write_cross_ids(&mut writer, &synonym.cross_references)?;
            write_end(&mut writer, &synonym.trailing_modifiers, &synonym.comment)?;
            writeln!(writer)?;
        }
        for (key, values) in obj.property_values.iter().sorted_by_key(|(k, _)| *k) {
            for (value, modifiers, comment) in values {
                write!(
                    writer,
                    "property_value: {key}: \"{value}\" xsd:{}",
                    value.datatype()
                )?;
                write_end(&mut writer, modifiers, comment)?;
                writeln!(writer)?;
            }
        }
        for (xref, modifiers, comment) in obj.xref.iter().sorted_by_key(|x| &x.0) {
            write!(writer, "xref: {xref}")?;
            write_end(&mut writer, modifiers, comment)?;
            writeln!(writer)?;
        }
        for (kind, xref, modifiers, comment) in obj
            .relationship
            .iter()
            .sorted_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)))
        {
            match kind {
                RelationType::IsA => write!(writer, "is_a: {xref}")?,
                RelationType::Other(t) => write!(writer, "relationship: {t} {xref}")?,
            }
            write_end(&mut writer, modifiers, comment)?;
            writeln!(writer)?;
        }
        if obj.obsolete {
            writeln!(writer, "is_obsolete: true")?;
        }
        for (kind, lines) in obj.lines.iter().sorted_by_key(|(k, _)| *k) {
            if kind.eq_ignore_ascii_case("name") {
                continue;
            }
            for (value, modifiers, comment) in lines.iter().sorted_by_key(|l| &l.0) {
                write!(writer, "{kind}: {value}")?;
                write_end(&mut writer, modifiers, comment)?;
                writeln!(writer)?;
            }
        }
    }

    Ok(())
}

fn write_cross_ids<W: Write>(
    mut writer: W,
    cross_ids: &[(Option<Box<str>>, Box<str>)],
) -> Result<(), std::io::Error> {
    write!(writer, "[")?;
    let mut first = true;
    for (tag, value) in cross_ids
        .iter()
        .sorted_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)))
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
    mut writer: W,
    trailing_modifiers: &[(Box<str>, Box<str>)],
    comment: &Option<Box<str>>,
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
    mut writer: W,
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
