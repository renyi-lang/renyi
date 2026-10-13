//! The chunks for retrieval by embedding (stage 37, decisions AW40 to
//! AW43; design document 05, section 8): the map's modules and definitions
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
//! identity. The language's own documents come as chunks too (decision
//! AW43): a section of a document, a module of the library and a public
//! declaration of it, in the same shape, so that a host embeds the language
//! beside the project. No model and no network here: the toolchain writes,
//! the host embeds.

use std::collections::HashMap;

use renyi_check::effects::Capability;
use renyi_syntax::ast::{Item, Type};
use renyi_syntax::json::Json;
use renyi_syntax::{parse_declarations, type_text, SourceFile, Span};
use sha2::{Digest, Sha256};

use crate::drafts::{ability_signature, function_signature, type_signature};
use crate::{display_path, Definition, Header, Index, Kind, Module};

/// One chunk: the text a host embeds, its id and what it keeps beside the
/// vector.
#[derive(Clone, Debug)]
pub struct Chunk {
    /// `sha256:` and the hash of `text`.
    pub id: String,
    /// The name of the module the chunk belongs to (a document's name for
    /// a section of it).
    pub module: String,
    /// The qualified name of the definition, the module's name, or the
    /// document's name and the section's heading.
    pub name: String,
    /// `module`, `section`, or the definition's kind as the map spells it.
    pub kind: String,
    pub text: String,
    /// The metadata, a JSON object: the names and identities, the kind,
    /// the effects, the edges and the location.
    pub metadata: Json,
}

/// A document whose sections become chunks: the name its chunks carry as
/// their module, the file their locations name and the text.
#[derive(Clone, Copy, Debug)]
pub struct Document<'a> {
    pub name: &'a str,
    pub file: &'a str,
    pub text: &'a str,
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

/// The language's own documents as chunks (decision AW43): one per section
/// of each document given (a `##` section with text of its own, each `###`
/// subsection under its section's heading, the text before the first
/// heading; the title line heads every chunk), then, per module of the
/// library (`renyi_check::LIBRARY`: the module's name and its declaration
/// file), the module's chunk and one per public declaration, in the shape
/// of a project's, with `package` naming the library and the version.
pub fn language_chunks(
    documents: &[Document],
    library: &[(&str, &str)],
    version: &str,
) -> Vec<Chunk> {
    let mut out = Vec::new();
    for document in documents {
        out.extend(section_chunks(document));
    }
    for (module, source) in library {
        out.extend(library_chunks(module, source, version));
    }
    out
}

/// The chunks as JSON Lines: a first line with the project, the revision,
/// the toolchain and the counts of what the lines hold (chunks, modules,
/// definitions, sections), then one object per chunk (`id`, `text`,
/// `metadata`), each on a line of its own.
pub fn chunks_lines(header: &Header, chunks: &[Chunk]) -> String {
    let modules = chunks.iter().filter(|chunk| chunk.kind == "module").count();
    let sections = chunks
        .iter()
        .filter(|chunk| chunk.kind == "section")
        .count();
    let mut out = Json::Object(vec![
        ("project", string(&header.project)),
        ("revision", string(&header.revision)),
        ("toolchain", string(&header.toolchain)),
        ("chunks", Json::Number(chunks.len())),
        ("modules", Json::Number(modules)),
        (
            "definitions",
            Json::Number(chunks.len() - modules - sections),
        ),
        ("sections", Json::Number(sections)),
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

// ---------------------------------------------------------------- a project

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
    header_lines(
        definition.kind.name(),
        &definition.qualified(),
        &definition.signature,
        definition.purpose.as_deref(),
        &definition.tags,
        &definition.effects_transitive,
        &definition.fails_transitive,
        &module.name,
        module.purpose.as_deref(),
    )
}

#[allow(clippy::too_many_arguments)] // the header's lines, in their order
fn header_lines(
    kind: &str,
    qualified: &str,
    signature: &str,
    purpose: Option<&str>,
    tags: &[String],
    effects: &[String],
    fails: &[String],
    module: &str,
    module_purpose: Option<&str>,
) -> String {
    let mut lines = vec![format!("{kind} {qualified}")];
    if !signature.is_empty() {
        lines.push(signature.to_string());
    }
    if let Some(purpose) = purpose {
        lines.push(format!("purpose: {purpose}"));
    }
    if !tags.is_empty() {
        lines.push(format!("tags: {}", tags.join(", ")));
    }
    if !effects.is_empty() {
        lines.push(format!("needs: {}", effects.join(", ")));
    }
    if !fails.is_empty() {
        lines.push(format!("fails with: {}", fails.join(", ")));
    }
    if let Some(purpose) = module_purpose {
        lines.push(format!("module {module}: {purpose}"));
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
    let public: Vec<(&str, &str)> = own
        .iter()
        .filter(|d| d.public)
        .map(|d| (d.kind.name(), d.signature.as_str()))
        .collect();
    let text = module_text(
        &module.name,
        module.purpose.as_deref(),
        &module.effects,
        &module.imports,
        &public,
    );
    Chunk {
        id: hash_of(&text),
        module: module.name.clone(),
        name: module.name.clone(),
        kind: "module".to_string(),
        text,
        metadata: module_metadata(module),
    }
}

/// A module's text: its purpose, what it needs and imports, and the kind
/// and signature of each public definition.
fn module_text(
    name: &str,
    purpose: Option<&str>,
    effects: &[String],
    imports: &[String],
    public: &[(&str, &str)],
) -> String {
    let mut text = format!("module {name}\n");
    if let Some(purpose) = purpose {
        text.push_str(&format!("purpose: {purpose}\n"));
    }
    if !effects.is_empty() {
        text.push_str(&format!("needs: {}\n", effects.join(", ")));
    }
    if !imports.is_empty() {
        text.push_str(&format!("imports: {}\n", imports.join(", ")));
    }
    if !public.is_empty() {
        text.push('\n');
        for (kind, signature) in public {
            text.push_str(&format!("{kind} {signature}\n"));
        }
    }
    text
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

// -------------------------------------------------------------- a document

/// One chunk per section of a Markdown document: a `##` section with text
/// of its own, each `###` subsection with its section's heading above it,
/// and the text before the first heading; the title line (`#`) heads every
/// chunk. A heading inside a fenced code block is text.
fn section_chunks(document: &Document) -> Vec<Chunk> {
    let lines: Vec<&str> = document.text.lines().collect();
    let mut headings: Vec<(usize, usize, &str)> = Vec::new();
    let mut fenced = false;
    for (index, line) in lines.iter().enumerate() {
        if line.starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let level = line.bytes().take_while(|byte| *byte == b'#').count();
        if (1..=3).contains(&level) && line[level..].starts_with(' ') {
            headings.push((index, level, line[level + 1..].trim()));
        }
    }
    let title_line = headings
        .iter()
        .find(|(_, level, _)| *level == 1)
        .map(|(index, _, _)| lines[*index]);
    let mut out = Vec::new();
    let mut section: Option<(usize, &str)> = None;
    for (position, &(index, level, heading)) in headings.iter().enumerate() {
        if level == 2 {
            section = Some((index, heading));
        }
        // the body: the lines after the heading up to the next one, blank
        // lines at either end dropped
        let mut first = index + 1;
        let mut last = headings
            .get(position + 1)
            .map_or(lines.len(), |next| next.0);
        while last > first && lines[last - 1].trim().is_empty() {
            last -= 1;
        }
        while first < last && lines[first].trim().is_empty() {
            first += 1;
        }
        if first == last {
            // a heading with nothing of its own: its subsections carry it
            continue;
        }
        let name = match (level, section) {
            (3, Some((_, parent))) => format!("{}: {parent} / {heading}", document.name),
            _ => format!("{}: {heading}", document.name),
        };
        let mut text = String::new();
        if level > 1 {
            if let Some(title_line) = title_line {
                text.push_str(title_line);
                text.push_str("\n\n");
            }
        }
        if level == 3 {
            if let Some((parent, _)) = section {
                text.push_str(lines[parent]);
                text.push_str("\n\n");
            }
        }
        text.push_str(lines[index]);
        text.push('\n');
        for line in &lines[first..last] {
            text.push_str(line);
            text.push('\n');
        }
        out.push(Chunk {
            id: hash_of(&text),
            module: document.name.to_string(),
            name: name.clone(),
            kind: "section".to_string(),
            text,
            metadata: plain_metadata(
                &name,
                "section",
                document.name,
                None,
                None,
                None,
                &[],
                &[],
                &[],
                &[],
                false,
                document.file,
                index + 1,
                last,
            ),
        });
    }
    out
}

// ------------------------------------------------------------- the library

/// A public declaration of a library module, as a chunk sees it.
struct Declaration<'a> {
    kind: Kind,
    name: String,
    signature: String,
    purpose: Option<&'a str>,
    tags: &'a [String],
    see_also: &'a [String],
    effects: Vec<String>,
    fails: Vec<String>,
    tested: bool,
    span: Span,
}

/// The module's chunk and one per public declaration of a library module,
/// from its declaration file.
fn library_chunks(module: &str, source: &str, version: &str) -> Vec<Chunk> {
    let file = SourceFile::new(
        format!(
            "library/std/{}.ry",
            module.strip_prefix("std.").unwrap_or(module)
        ),
        source,
    );
    let parsed = parse_declarations(source);
    let purpose = parsed.module.docs.purpose.as_deref();
    let package = format!("std {version}");
    let mut declarations: Vec<Declaration> = Vec::new();
    for item in &parsed.module.items {
        match item {
            Item::Function(function) if function.public => {
                let receiver = function.params.first().and_then(|param| {
                    if param.name.text != "self" {
                        return None;
                    }
                    match &param.ty {
                        Some(Type::Named { name, .. }) => Some(name.text.clone()),
                        _ => None,
                    }
                });
                declarations.push(Declaration {
                    kind: if receiver.is_some() {
                        Kind::Method
                    } else {
                        Kind::Function
                    },
                    name: match receiver {
                        Some(receiver) => format!("{receiver}.{}", function.name.text),
                        None => function.name.text.clone(),
                    },
                    signature: function_signature(function),
                    purpose: function.docs.purpose.as_deref(),
                    tags: &function.docs.tags,
                    see_also: &function.docs.see_also,
                    effects: function
                        .needs
                        .iter()
                        .map(|capability| Capability::from_ast(capability).spelling())
                        .collect(),
                    fails: function.fails.iter().map(type_text).collect(),
                    tested: !function.docs.examples.is_empty(),
                    span: function.span,
                });
            }
            Item::Type(def) if def.public => declarations.push(Declaration {
                kind: Kind::Type,
                name: def.name.text.clone(),
                signature: type_signature(def, &file),
                purpose: def.docs.purpose.as_deref(),
                tags: &def.docs.tags,
                see_also: &def.docs.see_also,
                effects: Vec::new(),
                fails: Vec::new(),
                tested: !def.docs.examples.is_empty(),
                span: def.span,
            }),
            Item::Ability(ability) if ability.public => declarations.push(Declaration {
                kind: Kind::Ability,
                name: ability.name.text.clone(),
                signature: ability_signature(ability),
                purpose: ability.docs.purpose.as_deref(),
                tags: &ability.docs.tags,
                see_also: &ability.docs.see_also,
                effects: Vec::new(),
                fails: Vec::new(),
                tested: !ability.docs.examples.is_empty(),
                span: ability.span,
            }),
            Item::Constant(constant) if constant.public => declarations.push(Declaration {
                kind: Kind::Constant,
                name: constant.name.text.clone(),
                signature: format!("{}: {}", constant.name.text, type_text(&constant.ty)),
                purpose: constant.docs.purpose.as_deref(),
                tags: &constant.docs.tags,
                see_also: &constant.docs.see_also,
                effects: Vec::new(),
                fails: Vec::new(),
                tested: !constant.docs.examples.is_empty(),
                span: constant.span,
            }),
            _ => {}
        }
    }

    let mut out = Vec::new();
    let public: Vec<(&str, &str)> = declarations
        .iter()
        .map(|d| (d.kind.name(), d.signature.as_str()))
        .collect();
    let text = module_text(module, purpose, &[], &[], &public);
    out.push(Chunk {
        id: hash_of(&text),
        module: module.to_string(),
        name: module.to_string(),
        kind: "module".to_string(),
        text,
        metadata: Json::Object(vec![
            ("name", string(module)),
            ("kind", string("module")),
            ("module", string(module)),
            ("package", string(&package)),
            ("map_id", Json::Null),
            ("text_hash", Json::Null),
            ("public", Json::Bool(true)),
            ("signature", Json::Null),
            ("purpose", optional(purpose)),
            ("tags", Json::Array(Vec::new())),
            ("see_also", Json::Array(Vec::new())),
            ("effects", Json::Array(Vec::new())),
            ("fails", Json::Array(Vec::new())),
            ("calls", Json::Array(Vec::new())),
            ("uses", Json::Array(Vec::new())),
            ("tested_by", Json::Array(Vec::new())),
            ("tested", Json::Bool(false)),
            ("location", location(&file.name, 1, file.line_count())),
            ("imports", Json::Array(Vec::new())),
            ("ids", Json::Array(Vec::new())),
        ]),
    });
    for declaration in &declarations {
        let qualified = format!("{module}.{}", declaration.name);
        let mut text = header_lines(
            declaration.kind.name(),
            &qualified,
            &declaration.signature,
            declaration.purpose,
            declaration.tags,
            &declaration.effects,
            &declaration.fails,
            module,
            purpose,
        );
        text.push('\n');
        text.push_str(
            source
                .get(declaration.span.start..declaration.span.end)
                .unwrap_or("")
                .trim_end(),
        );
        text.push('\n');
        let line = file.position(declaration.span.start).line;
        let end_line = file
            .position(
                declaration
                    .span
                    .end
                    .saturating_sub(1)
                    .max(declaration.span.start),
            )
            .line;
        out.push(Chunk {
            id: hash_of(&text),
            module: module.to_string(),
            name: qualified.clone(),
            kind: declaration.kind.name().to_string(),
            text,
            metadata: plain_metadata(
                &qualified,
                declaration.kind.name(),
                module,
                Some(&package),
                Some(&declaration.signature),
                declaration.purpose,
                declaration.tags,
                declaration.see_also,
                &declaration.effects,
                &declaration.fails,
                declaration.tested,
                &file.name,
                line,
                end_line,
            ),
        });
    }
    out
}

/// The metadata of a chunk the map has no record of (a section, a library
/// declaration): the same keys as a definition's, the map's identities and
/// the edges null or empty.
#[allow(clippy::too_many_arguments)] // the keys of the object, in their order
fn plain_metadata(
    name: &str,
    kind: &str,
    module: &str,
    package: Option<&str>,
    signature: Option<&str>,
    purpose: Option<&str>,
    tags: &[String],
    see_also: &[String],
    effects: &[String],
    fails: &[String],
    tested: bool,
    file: &str,
    line: usize,
    end_line: usize,
) -> Json {
    Json::Object(vec![
        ("name", string(name)),
        ("kind", string(kind)),
        ("module", string(module)),
        ("package", optional(package)),
        ("map_id", Json::Null),
        ("text_hash", Json::Null),
        ("public", Json::Bool(true)),
        ("signature", optional(signature)),
        ("purpose", optional(purpose)),
        ("tags", strings(tags)),
        ("see_also", strings(see_also)),
        ("effects", strings(effects)),
        ("fails", strings(fails)),
        ("calls", Json::Array(Vec::new())),
        ("uses", Json::Array(Vec::new())),
        ("tested_by", Json::Array(Vec::new())),
        ("tested", Json::Bool(tested)),
        ("location", location(file, line, end_line)),
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
