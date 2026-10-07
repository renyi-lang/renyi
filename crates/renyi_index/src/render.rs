//! The map as text (one line per definition) and as JSON (design document
//! 05, section 4).

use renyi_syntax::json::Json;

use crate::{Definition, Index, Module};

/// One line per module and per definition: kind, qualified name, signature,
/// transitive effects and the metrics, in module and line order.
pub fn to_text(index: &Index) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "project {}  revision {}  toolchain {}\n",
        index.header.project, index.header.revision, index.header.toolchain
    ));
    out.push_str(&format!(
        "{} modules, {} definitions\n",
        index.modules.len(),
        index.definitions.len()
    ));
    for module in &index.modules {
        out.push('\n');
        out.push_str(&format!(
            "module {}  {}  {} definitions ({} public)  {} lines  effects: {}",
            module.name,
            module.file,
            module.definitions,
            module.public,
            module.lines,
            list_or_none(&module.effects)
        ));
        if let Some(package) = &module.package {
            out.push_str(&format!("  package: {package}"));
        }
        if module.errors > 0 {
            out.push_str(&format!("  errors: {}", module.errors));
        }
        if !module.canonical {
            out.push_str("  (not in canonical layout; lines refer to the formatted text)");
        }
        out.push('\n');
        for definition in index.definitions.iter().filter(|d| d.module == module.name) {
            out.push_str(&definition_line(definition));
            out.push('\n');
        }
    }
    out
}

fn definition_line(definition: &Definition) -> String {
    let metrics = &definition.metrics;
    format!(
        "  {:<14} {}  {}  effects: {}  lines={} depth={} branches={} effects={} fan_in={} fan_out={} library_calls={}",
        definition.kind.name(),
        definition.qualified(),
        definition.signature,
        list_or_none(&definition.effects_transitive),
        metrics.lines,
        metrics.depth,
        metrics.branches,
        metrics.effects,
        metrics.fan_in,
        metrics.fan_out,
        metrics.library_calls
    )
}

fn list_or_none(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_string()
    } else {
        items.join(", ")
    }
}

/// The map as one JSON document.
pub fn to_json(index: &Index) -> String {
    Json::Object(vec![
        ("project", string(&index.header.project)),
        ("revision", string(&index.header.revision)),
        ("toolchain", string(&index.header.toolchain)),
        (
            "modules",
            Json::Array(index.modules.iter().map(module_json).collect()),
        ),
        (
            "definitions",
            Json::Array(index.definitions.iter().map(definition_json).collect()),
        ),
    ])
    .render()
}

fn module_json(module: &Module) -> Json {
    Json::Object(vec![
        ("name", string(&module.name)),
        ("file", string(&module.file)),
        ("package", optional(module.package.as_deref())),
        ("purpose", optional(module.purpose.as_deref())),
        ("imports", strings(&module.imports)),
        ("definitions", Json::Number(module.definitions)),
        ("public", Json::Number(module.public)),
        ("lines", Json::Number(module.lines)),
        ("effects", strings(&module.effects)),
        ("ids", strings(&module.ids)),
        ("errors", Json::Number(module.errors)),
        ("canonical", Json::Bool(module.canonical)),
    ])
}

/// One definition's record (`renyi mcp` answers `definition` with it).
pub fn definition_json(definition: &Definition) -> Json {
    let metrics = &definition.metrics;
    Json::Object(vec![
        ("id", string(&definition.id)),
        ("text_hash", string(&definition.text_hash)),
        ("module", string(&definition.module)),
        ("package", optional(definition.package.as_deref())),
        ("name", string(&definition.name)),
        ("kind", string(definition.kind.name())),
        ("public", Json::Bool(definition.public)),
        ("signature", string(&definition.signature)),
        ("purpose", optional(definition.purpose.as_deref())),
        ("tags", strings(&definition.tags)),
        ("see_also", strings(&definition.see_also)),
        ("exposed_as_tool", Json::Bool(definition.exposed_as_tool)),
        (
            "effects",
            Json::Object(vec![
                ("declared", strings(&definition.effects_declared)),
                ("transitive", strings(&definition.effects_transitive)),
            ]),
        ),
        (
            "fails",
            Json::Object(vec![
                ("declared", strings(&definition.fails_declared)),
                ("transitive", strings(&definition.fails_transitive)),
            ]),
        ),
        ("calls", strings(&definition.calls)),
        ("uses", strings(&definition.uses)),
        (
            "implements",
            match &definition.implements {
                Some(implements) => Json::Object(vec![
                    ("ability", string(&implements.ability)),
                    ("target", string(&implements.target)),
                ]),
                None => Json::Null,
            },
        ),
        ("tested_by", strings(&definition.tested_by)),
        (
            "metrics",
            Json::Object(vec![
                ("lines", Json::Number(metrics.lines)),
                ("depth", Json::Number(metrics.depth)),
                ("branches", Json::Number(metrics.branches)),
                ("effects", Json::Number(metrics.effects)),
                ("fan_in", Json::Number(metrics.fan_in)),
                ("fan_out", Json::Number(metrics.fan_out)),
                ("library_calls", Json::Number(metrics.library_calls)),
            ]),
        ),
        (
            "coverage",
            Json::Object(vec![
                ("examples", Json::Number(definition.examples)),
                ("tests", Json::Number(definition.tests)),
            ]),
        ),
        (
            "location",
            Json::Object(vec![
                ("file", string(&definition.file)),
                ("line", Json::Number(definition.line)),
                ("end_line", Json::Number(definition.end_line)),
            ]),
        ),
    ])
}

fn string(value: &str) -> Json {
    Json::String(value.to_string())
}

fn strings(values: &[String]) -> Json {
    Json::Array(values.iter().map(|value| string(value)).collect())
}

fn optional(value: Option<&str>) -> Json {
    value.map_or(Json::Null, string)
}
