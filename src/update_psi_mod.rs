use mzcv::{OboOntology, OboValue};

pub fn psi_mod_proper_style(ontology: &mut OboOntology) {
    for obj in &mut ontology.objects {
        obj.xref.retain(|(xref, m, c)| {
            let value = xref.1.trim_matches('\"');
            match xref.0.as_deref() {
                Some(t @ ("DiffAvg" | "MassAvg")) if let Ok(v) = value.parse::<f64>() => {
                    obj.property_values.entry(t.into()).or_default().push((
                        OboValue::Float(v, "float", Some(2)),
                        m.clone(),
                        c.clone(),
                    ));
                    false
                }
                Some(t @ ("DiffMono" | "MassMono")) if let Ok(v) = value.parse::<f64>() => {
                    obj.property_values.entry(t.into()).or_default().push((
                        OboValue::Float(v, "float", Some(6)),
                        m.clone(),
                        c.clone(),
                    ));
                    false
                }
                Some(
                    t @ ("Source" | "Origin" | "TermSpec" | "DiffFormula" | "Formula"
                    | "FormalCharge"),
                ) => {
                    obj.property_values.entry(t.into()).or_default().push((
                        OboValue::String(value.into()),
                        m.clone(),
                        c.clone(),
                    ));
                    false
                }
                Some("GNOme" | "uniprot.ptm" | "Unimod") => {
                    if let Some((_, x, _, _)) = &mut obj.definition {
                        let v = value.split_once(':').map_or_else(
                            || (None, value.into()),
                            |(t, v)| (Some(t.into()), v.into()),
                        );
                        x.push(v);
                    }
                    false
                }
                Some("Remap") => {
                    obj.lines.entry("replaced_by".into()).or_default().push((
                        value.into(),
                        m.clone(),
                        c.clone(),
                    ));
                    false
                }
                _ => value.eq_ignore_ascii_case("\"none\""),
            }
        });
    }
}
