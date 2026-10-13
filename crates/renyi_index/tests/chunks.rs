//! The chunks for retrieval by embedding (stage 37, decisions AW40 to
//! AW42): one per module and per definition of the corpus, in the text of
//! decision AW41; the id the hash of the text, the same after a
//! formatting-only change and after a dependency's change, different
//! after the definition's own text changed; the JSON Lines.

use std::path::PathBuf;

use renyi_check::{check_project_in, Library};
use renyi_index::{
    canonical_files, chunks, chunks_lines, index_checked, load_project, Chunk, Header, Index,
};
use renyi_syntax::json::Json;
use renyi_syntax::SourceFile;
use sha2::{Digest, Sha256};

fn header() -> Header {
    Header {
        project: "test".into(),
        revision: "0".into(),
        toolchain: "renyi test".into(),
    }
}

/// The map and the chunks of the files, as `renyi index --chunks` builds
/// them.
fn chunks_of(files: &[SourceFile]) -> (Index, Vec<Chunk>) {
    let (canonical, was_canonical) = canonical_files(files);
    let checked = check_project_in(&Library::standard(), &canonical, &[]);
    let index = index_checked(&canonical, &was_canonical, &checked, header());
    let chunks = chunks(&index, &canonical);
    (index, chunks)
}

fn corpus() -> (Index, Vec<Chunk>) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let files = load_project(&dir).expect("the corpus loads");
    chunks_of(&files)
}

fn program(source: &str) -> (Index, Vec<Chunk>) {
    chunks_of(&[SourceFile::new("demo.ry", source)])
}

fn find<'a>(chunks: &'a [Chunk], name: &str) -> &'a Chunk {
    chunks
        .iter()
        .find(|chunk| chunk.name == name)
        .unwrap_or_else(|| panic!("no chunk named {name}"))
}

fn field<'a>(json: &'a Json, key: &str) -> &'a Json {
    match json {
        Json::Object(fields) => {
            &fields
                .iter()
                .find(|(name, _)| *name == key)
                .unwrap_or_else(|| panic!("no field `{key}`"))
                .1
        }
        other => panic!("not an object: {other:?}"),
    }
}

fn text_of(json: &Json) -> &str {
    match json {
        Json::String(text) => text,
        other => panic!("not a string: {other:?}"),
    }
}

fn ids(chunks: &[Chunk]) -> Vec<(String, String)> {
    chunks
        .iter()
        .map(|chunk| (chunk.name.clone(), chunk.id.clone()))
        .collect()
}

const DEMO: &str = "module demo
  purpose: A caller and a helper.

public function caller(value: Integer) returns Integer
  purpose: One more than the helper.

  return helper(value) + 1
end

function helper(value: Integer) returns Integer
  return value + 1
end
";

#[test]
fn the_corpus_has_a_chunk_per_module_and_per_definition() {
    let (index, chunks) = corpus();
    assert_eq!(chunks.len(), index.modules.len() + index.definitions.len());
    for chunk in &chunks {
        assert_eq!(
            chunk.id,
            format!("sha256:{:x}", Sha256::digest(chunk.text.as_bytes())),
            "{} is the hash of its text",
            chunk.name
        );
        assert!(chunk.text.ends_with('\n'), "{}", chunk.name);
        assert!(!chunk.text.contains("\n\n\n"), "{}", chunk.name);
    }
    // the map's order: a module's chunk, then its definitions' in order
    assert_eq!(chunks[0].name, index.modules[0].name);
    assert_eq!(chunks[0].kind, "module");
    assert_eq!(chunks[1].name, index.definitions[0].qualified());

    let total = find(&chunks, "invoice.total");
    let record = index
        .definitions
        .iter()
        .find(|d| d.qualified() == "invoice.total")
        .expect("invoice.total is in the map");
    assert_eq!(total.kind, "function");
    assert_eq!(total.module, "invoice");
    assert!(
        total.text.starts_with(
            "function invoice.total\ntotal(invoice: Invoice) returns Decimal\npurpose: "
        ),
        "{}",
        total.text
    );
    assert!(
        total
            .text
            .contains("\nmodule invoice: Compute invoice totals"),
        "{}",
        total.text
    );
    assert!(
        total
            .text
            .contains("\n\npublic function total(invoice: Invoice) returns Decimal\n"),
        "{}",
        total.text
    );
    assert!(total.text.ends_with("\nend\n"), "{}", total.text);
    assert_eq!(text_of(field(&total.metadata, "name")), "invoice.total");
    assert_eq!(text_of(field(&total.metadata, "map_id")), record.id);
    assert_eq!(
        text_of(field(&total.metadata, "text_hash")),
        record.text_hash
    );
    assert!(matches!(field(&total.metadata, "public"), Json::Bool(true)));
    assert!(matches!(field(&total.metadata, "tested"), Json::Bool(true)));
    let location = field(&total.metadata, "location");
    assert!(matches!(field(location, "line"), Json::Number(line) if *line == record.line));
    assert!(
        matches!(field(&total.metadata, "calls"), Json::Array(calls) if calls.len() == record.calls.len())
    );

    let module = find(&chunks, "invoice");
    assert_eq!(module.kind, "module");
    assert!(
        module
            .text
            .starts_with("module invoice\npurpose: Compute invoice totals"),
        "{}",
        module.text
    );
    assert!(
        module
            .text
            .contains("\nfunction total(invoice: Invoice) returns Decimal\n"),
        "{}",
        module.text
    );
    assert!(matches!(field(&module.metadata, "map_id"), Json::Null));
    assert!(
        matches!(field(&module.metadata, "ids"), Json::Array(ids) if ids.len() == record_count(&index, "invoice"))
    );

    let test = chunks
        .iter()
        .find(|chunk| chunk.kind == "test" && chunk.module == "invoice")
        .expect("a test of invoice is a chunk");
    assert!(test.text.starts_with("test invoice.test:"), "{}", test.text);
    assert!(test.text.contains("\n\ntest \""), "{}", test.text);
}

fn record_count(index: &Index, module: &str) -> usize {
    index
        .definitions
        .iter()
        .filter(|d| d.module == module)
        .count()
}

#[test]
fn an_id_follows_the_text_not_the_layout_or_the_dependencies() {
    let (_, before) = program(DEMO);
    assert_eq!(
        before.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
        ["demo", "demo.caller", "demo.helper"]
    );

    // a formatting-only change: the chunks are of the canonical text
    let relaid = DEMO
        .replace("  return helper(value) + 1", "  return   helper( value )+1")
        .replace("end\n\nfunction helper", "end\n\n\n\nfunction helper");
    let (_, same) = program(&relaid);
    assert_eq!(ids(&same), ids(&before));

    // the helper's body changed: its chunk changes, the caller's does not,
    // though the caller's map id does (its dependency changed), which the
    // metadata carries beside the unchanged text hash
    let (_, after) = program(&DEMO.replace("  return value + 1", "  return value + 2"));
    assert_ne!(
        find(&after, "demo.helper").id,
        find(&before, "demo.helper").id
    );
    assert_eq!(
        find(&after, "demo.caller").id,
        find(&before, "demo.caller").id
    );
    assert_eq!(find(&after, "demo").id, find(&before, "demo").id);
    let (caller_before, caller_after) = (find(&before, "demo.caller"), find(&after, "demo.caller"));
    assert_ne!(
        text_of(field(&caller_after.metadata, "map_id")),
        text_of(field(&caller_before.metadata, "map_id"))
    );
    assert_eq!(
        text_of(field(&caller_after.metadata, "text_hash")),
        text_of(field(&caller_before.metadata, "text_hash"))
    );

    // the module's purpose is in every header: a change to it changes
    // every chunk of the module
    let (_, reworded) =
        program(&DEMO.replace("A caller and a helper.", "A helper and its caller."));
    for (chunk, original) in reworded.iter().zip(&before) {
        assert_ne!(chunk.id, original.id, "{}", chunk.name);
    }

    // a renamed helper: the texts change, the text hashes do not
    let (_, renamed) = program(&DEMO.replace("helper(", "successor("));
    assert_ne!(
        find(&renamed, "demo.caller").id,
        find(&before, "demo.caller").id
    );
    assert_eq!(
        text_of(field(&find(&renamed, "demo.caller").metadata, "text_hash")),
        text_of(field(&find(&before, "demo.caller").metadata, "text_hash"))
    );
}

#[test]
fn the_lines_are_one_object_each() {
    let (index, chunks) = corpus();
    let rendered = chunks_lines(&index.header, &chunks);
    let lines: Vec<&str> = rendered.lines().collect();
    assert_eq!(lines.len(), chunks.len() + 1);
    assert_eq!(
        lines[0],
        format!(
            "{{\"project\": \"test\", \"revision\": \"0\", \"toolchain\": \"renyi test\", \"chunks\": {}, \"modules\": {}, \"definitions\": {}}}",
            chunks.len(),
            index.modules.len(),
            index.definitions.len()
        )
    );
    for (line, chunk) in lines[1..].iter().zip(&chunks) {
        assert!(
            line.starts_with(&format!("{{\"id\": \"{}\", \"text\": \"", chunk.id)),
            "{line}"
        );
        assert!(
            line.contains(&format!(
                "\", \"metadata\": {{\"name\": \"{}\", \"kind\": \"{}\", ",
                chunk.name, chunk.kind
            )),
            "{line}"
        );
        assert!(line.ends_with("}}"), "{line}");
    }
    // a narrowed set counts what it holds
    let invoice: Vec<Chunk> = chunks
        .iter()
        .filter(|chunk| chunk.module == "invoice")
        .cloned()
        .collect();
    let narrowed = chunks_lines(&index.header, &invoice);
    assert!(
        narrowed.starts_with(&format!(
            "{{\"project\": \"test\", \"revision\": \"0\", \"toolchain\": \"renyi test\", \"chunks\": {}, \"modules\": 1, \"definitions\": {}}}\n",
            invoice.len(),
            invoice.len() - 1
        )),
        "{narrowed}"
    );
}
