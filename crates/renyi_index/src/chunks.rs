//! The chunks for retrieval by embedding (stage 37, decisions AW40 to
//! AW42; design document 05, section 8): the map's modules and definitions
//! as units of text for a host to embed, each with an id and the metadata
//! a query filters by and a hit is walked from. A definition's text is the
//! one the measure of stage 37a settled (decision AW41): a header from the
//! record (the kind and the qualified name, the signature, the purpose, the
//! tags, the transitive effects and failures, the module's purpose) and
//! then the definition's canonical source; a module's text is its purpose,
//! what it needs and imports, and the signatures of its public definitions.
//! A chunk's id is the SHA-256 of its text, so that a host which keeps its
//! index by id embeds again exactly the chunks whose text changed; the
//! map's `id` and `text_hash` travel in the metadata for the semantic
//! identity. No model and no network here: the toolchain writes, the host
//! embeds.

use std::collections::HashMap;

use renyi_syntax::json::Json;
use renyi_syntax::SourceFile;
use sha2::{Digest, Sha256};

use crate::{display_path, Definition, Header, Index, Module};

/// One chunk: the text a host embeds, its id and what it keeps beside the
/// vector.
#[derive(Clone, Debug)]
pub struct Chunk {
    /// `sha256:` and the hash of `text`.
    pub id: String,
    /// The name of the module the chunk belongs to.
    pub module: String,
    /// The qualified name of the definition, or the module's name.
    pub name: String,
    /// `module`, or the definition's kind as the map spells it.
    pub kind: String,
    pub text: String,
    /// The metadata, a JSON object: the names and identities, the kind,
    /// the effects, the edges and the location.
    pub metadata: Json,
}

/// The chunks of a map, in the map's order: each module's chunk and then
/// the chunks of its definitions. `files` gives the canonical sources the
/// map's locations refer to (the files `index_checked` was given, or the
/// workspace's); a definition whose file is not among them gets its header
/// alone. A module record without a purpose and without definitions (a
/// file that does not parse) gets no chunk.
pub fn chunks<'a>(index: &Index, files: impl IntoIterator<Item = &'a SourceFile>) -> Vec<Chunk> {
    let texts: HashMap<String, &str> = files
        .into_iter()
        .map(|file| (display_path(&file.name), file.text.as_str()))
        .collect();
    let mut out = Vec::new();
    for module in &index.modules {
        let own: Vec<&Definition> = index
            .definitions
            .iter()
            .filter(|definition| definition.module == module.name)
            .collect();
        if own.is_empty() && module.purpose.is_none() {
            continue;
        }
        out.push(module_chunk(module, &own));
        let text = texts.get(&module.file).copied();
        for definition in own {
            out.push(definition_chunk(definition, module, text));
        }
    }
    out
}

/// The chunks as JSON Lines: a first line with the project, the revision,
/// the toolchain and the counts, then one object per chunk (`id`, `text`,
/// `metadata`), each on a line of its own.
pub fn chunks_lines(header: &Header, chunks: &[Chunk]) -> String {
    let modules = chunks.iter().filter(|chunk| chunk.kind == "module").count();
    let mut out = Json::Object(vec![
        ("project", string(&header.project)),
        ("revision", string(&header.revision)),
        ("toolchain", string(&header.toolchain)),
        ("chunks", Json::Number(chunks.len())),
        ("modules", Json::Number(modules)),
        ("definitions", Json::Number(chunks.len() - modules)),
    ])
    .render_line();
    out.push('\n');
    for chunk in chunks {
        out.push_str(&chunk_json(chunk).render_line());
        out.push('\n');
    }
    out
}

/// One chunk's object: `id`, `text`, `metadata`.
pub fn chunk_json(chunk: &Chunk) -> Json {
    Json::Object(vec![
        ("id", string(&chunk.id)),
        ("text", string(&chunk.text)),
        ("metadata", chunk.metadata.clone()),
    ])
}

fn definition_chunk(definition: &Definition, module: &Module, file: Option<&str>) -> Chunk {
    let mut text = header_of(definition, module);
    if let Some(file) = file {
        text.push('\n');
        text.push_str(&source_of(file, definition));
    }
    Chunk {
        id: hash_of(&text),
        module: definition.module.clone(),
        name: definition.qualified(),
        kind: definition.kind.name().to_string(),
        text,
        metadata: definition_metadata(definition),
    }
}

/// What the embedding sees before the source: what the definition is
/// called, what it is for, what it needs and what its module is for (the
/// text of decision AW41).
fn header_of(definition: &Definition, module: &Module) -> String {
    let mut lines = vec![format!(
        "{} {}",
        definition.kind.name(),
        definition.qualified()
    )];
    if !definition.signature.is_empty() {
        lines.push(definition.signature.clone());
    }
    if let Some(purpose) = &definition.purpose {
        lines.push(format!("purpose: {purpose}"));
    }
    if !definition.tags.is_empty() {
        lines.push(format!("tags: {}", definition.tags.join(", ")));
    }
    if !definition.effects_transitive.is_empty() {
        lines.push(format!(
            "needs: {}",
            definition.effects_transitive.join(", ")
        ));
    }
    if !definition.fails_transitive.is_empty() {
        lines.push(format!(
            "fails with: {}",
            definition.fails_transitive.join(", ")
        ));
    }
    if let Some(purpose) = &module.purpose {
        lines.push(format!("module {}: {purpose}", module.name));
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// The definition's lines of its canonical file, the map's `location`.
fn source_of(file: &str, definition: &Definition) -> String {
    let lines: Vec<&str> = file
        .lines()
        .skip(definition.line.saturating_sub(1))
        .take(definition.end_line + 1 - definition.line)
        .collect();
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

fn module_chunk(module: &Module, own: &[&Definition]) -> Chunk {
    let mut text = format!("module {}\n", module.name);
    if let Some(purpose) = &module.purpose {
        text.push_str(&format!("purpose: {purpose}\n"));
    }
    if !module.effects.is_empty() {
        text.push_str(&format!("needs: {}\n", module.effects.join(", ")));
    }
    if !module.imports.is_empty() {
        text.push_str(&format!("imports: {}\n", module.imports.join(", ")));
    }
    let public: Vec<&&Definition> = own.iter().filter(|d| d.public).collect();
    if !public.is_empty() {
        text.push('\n');
        for definition in public {
            text.push_str(&format!(
                "{} {}\n",
                definition.kind.name(),
                definition.signature
            ));
        }
    }
    Chunk {
        id: hash_of(&text),
        module: module.name.clone(),
        name: module.name.clone(),
        kind: "module".to_string(),
        text,
        metadata: module_metadata(module),
    }
}

fn hash_of(text: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(text.as_bytes()))
}

/// What a host filters by and walks from: the map's identities, the kind
/// and the module, the effects, the edges and the location.
fn definition_metadata(definition: &Definition) -> Json {
    Json::Object(vec![
        ("name", string(&definition.qualified())),
        ("kind", string(definition.kind.name())),
        ("module", string(&definition.module)),
        ("package", optional(definition.package.as_deref())),
        ("map_id", string(&definition.id)),
        ("text_hash", string(&definition.text_hash)),
        ("public", Json::Bool(definition.public)),
        ("signature", string(&definition.signature)),
        ("purpose", optional(definition.purpose.as_deref())),
        ("tags", strings(&definition.tags)),
        ("see_also", strings(&definition.see_also)),
        ("effects", strings(&definition.effects_transitive)),
        ("fails", strings(&definition.fails_transitive)),
        ("calls", strings(&definition.calls)),
        ("uses", strings(&definition.uses)),
        ("tested_by", strings(&definition.tested_by)),
        (
            "tested",
            Json::Bool(definition.tests > 0 || definition.examples > 0),
        ),
        (
            "location",
            location(&definition.file, definition.line, definition.end_line),
        ),
    ])
}

/// The same keys as a definition's, so that a host filters every chunk
/// alike, with the module's imports and the map ids of its definitions.
fn module_metadata(module: &Module) -> Json {
    Json::Object(vec![
        ("name", string(&module.name)),
        ("kind", string("module")),
        ("module", string(&module.name)),
        ("package", optional(module.package.as_deref())),
        ("map_id", Json::Null),
        ("text_hash", Json::Null),
        ("public", Json::Bool(true)),
        ("signature", Json::Null),
        ("purpose", optional(module.purpose.as_deref())),
        ("tags", Json::Array(Vec::new())),
        ("see_also", Json::Array(Vec::new())),
        ("effects", strings(&module.effects)),
        ("fails", Json::Array(Vec::new())),
        ("calls", Json::Array(Vec::new())),
        ("uses", Json::Array(Vec::new())),
        ("tested_by", Json::Array(Vec::new())),
        ("tested", Json::Bool(false)),
        ("location", location(&module.file, 1, module.lines)),
        ("imports", strings(&module.imports)),
        ("ids", strings(&module.ids)),
    ])
}

fn location(file: &str, line: usize, end_line: usize) -> Json {
    Json::Object(vec![
        ("file", string(file)),
        ("line", Json::Number(line)),
        ("end_line", Json::Number(end_line)),
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
