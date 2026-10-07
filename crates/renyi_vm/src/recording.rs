//! Recorded runs (decision P1, `06-runtime-guarantees.md` section 1): the
//! recording a run writes, one entry per primitive call made under a
//! capability, and the replay that answers calls from one. Arguments and
//! outcomes are JSON by the `ToJson` rules, so a recording is readable and
//! editable, and a replay decodes an outcome by the primitive's declared
//! types.

use sha2::{Digest, Sha256};

use crate::natives::json::{read_json, write_json, Json};

#[derive(Clone, Debug, PartialEq)]
pub struct Recording {
    pub program: String,
    pub revision: Option<String>,
    pub recorded_at: String,
    /// The grant the run executed under, one capability per entry.
    pub grant: Vec<String>,
    pub calls: Vec<Call>,
    /// The run manifest (decision Q2), filled in by `renyi record`.
    pub manifest: Manifest,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Call {
    /// From 1, in the order the calls completed.
    pub sequence: u64,
    /// The capability the call exercised, with its actual scope.
    pub capability: String,
    /// `std.http.get`.
    pub primitive: String,
    /// By parameter name, in declaration order.
    pub arguments: Vec<(String, Json)>,
    pub outcome: Outcome,
    pub duration_ms: Option<i64>,
    /// Milliseconds after the run began, for the budgets of a replay.
    pub at_ms: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Success(Json),
    Failure(Json),
}

/// What a run depended on besides its calls (decision Q2, design document
/// 07 section 3): the header of a recording made by `renyi record`,
/// printed alone by `renyi run --manifest`, checked by `renyi reproduce`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Manifest {
    /// `renyi 0.0.1`.
    pub toolchain: Option<String>,
    /// The program's path as the command line gave it.
    pub source: Option<String>,
    /// The content hash of `main`, which covers everything it reaches.
    pub code: Option<String>,
    pub arguments: Vec<String>,
    /// Each environment variable read, with the hash of its value, the
    /// redaction placeholder, or `None` when the variable was not set.
    pub environment: Vec<(String, Option<String>)>,
    /// `finished`, `failed with ...`, `exited with N` or `crashed: ...`.
    pub outcome: Option<String>,
    /// The hash of the standard output and its length in bytes.
    pub output: Option<(String, u64)>,
    /// Every package the program reaches, with the version and the hash
    /// the lockfile names (decision AC1), in name order.
    pub dependencies: Vec<Dependency>,
    /// The extensions the toolchain was built with beyond the standard
    /// library, each `name version` (decision AK1); `reproduce` warns
    /// when they differ.
    pub extensions: Vec<String>,
}

/// One dependency of the run manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub hash: String,
}

/// `sha256:` and the hex digest, as the index spells content hashes.
pub fn sha256_of(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

impl Recording {
    pub fn new(
        program: impl Into<String>,
        revision: Option<String>,
        recorded_at: String,
        grant: Vec<String>,
    ) -> Recording {
        Recording {
            program: program.into(),
            revision,
            recorded_at,
            grant,
            calls: Vec::new(),
            manifest: Manifest::default(),
        }
    }

    /// Append a call; its sequence number is assigned here.
    pub fn push(&mut self, mut call: Call) {
        call.sequence = self.calls.len() as u64 + 1;
        self.calls.push(call);
    }

    pub fn to_json(&self) -> Json {
        let mut fields = self.header();
        fields.push((
            "calls".to_string(),
            Json::Array(self.calls.iter().map(Call::to_json).collect()),
        ));
        Json::Object(fields)
    }

    /// The header: the program, the revision, the manifest and the grant,
    /// everything but the calls.
    fn header(&self) -> Vec<(String, Json)> {
        let text = |value: &str| Json::Text(value.to_string());
        let manifest = &self.manifest;
        let mut fields = vec![("program".to_string(), text(&self.program))];
        if let Some(revision) = &self.revision {
            fields.push(("revision".to_string(), text(revision)));
        }
        for (name, value) in [
            ("toolchain", &manifest.toolchain),
            ("source", &manifest.source),
            ("code", &manifest.code),
        ] {
            if let Some(value) = value {
                fields.push((name.to_string(), text(value)));
            }
        }
        if !manifest.extensions.is_empty() {
            fields.push((
                "extensions".to_string(),
                Json::Array(manifest.extensions.iter().map(|e| text(e)).collect()),
            ));
        }
        if !manifest.dependencies.is_empty() {
            fields.push((
                "dependencies".to_string(),
                Json::Object(
                    manifest
                        .dependencies
                        .iter()
                        .map(|dependency| {
                            (
                                dependency.name.clone(),
                                Json::Object(vec![
                                    ("version".to_string(), text(&dependency.version)),
                                    ("hash".to_string(), text(&dependency.hash)),
                                ]),
                            )
                        })
                        .collect(),
                ),
            ));
        }
        fields.push(("recorded_at".to_string(), text(&self.recorded_at)));
        fields.push((
            "grant".to_string(),
            Json::Array(self.grant.iter().cloned().map(Json::Text).collect()),
        ));
        if !manifest.arguments.is_empty() {
            fields.push((
                "arguments".to_string(),
                Json::Array(manifest.arguments.iter().cloned().map(Json::Text).collect()),
            ));
        }
        if !manifest.environment.is_empty() {
            fields.push((
                "environment".to_string(),
                Json::Object(
                    manifest
                        .environment
                        .iter()
                        .map(|(name, value)| {
                            (name.clone(), value.as_deref().map_or(Json::Null, text))
                        })
                        .collect(),
                ),
            ));
        }
        if let Some(outcome) = &manifest.outcome {
            fields.push(("outcome".to_string(), text(outcome)));
        }
        if let Some((hash, bytes)) = &manifest.output {
            fields.push((
                "output".to_string(),
                Json::Object(vec![
                    ("stdout".to_string(), text(hash)),
                    ("bytes".to_string(), Json::Number(bytes.to_string())),
                ]),
            ));
        }
        fields
    }

    /// The manifest alone, as indented JSON text (`renyi run --manifest`).
    pub fn render_manifest(&self) -> String {
        let mut out = String::new();
        write_json(&Json::Object(self.header()), &mut out, Some(2), 0);
        out.push('\n');
        out
    }

    /// Complete the manifest once the run has ended: its outcome, its
    /// output, and the environment variables its calls read (the hash of
    /// each value; a redacted one stays the placeholder; `None` when the
    /// variable was not set).
    pub fn finish(&mut self, outcome: String, output: (String, u64)) {
        self.manifest.outcome = Some(outcome);
        self.manifest.output = Some(output);
        let mut environment: Vec<(String, Option<String>)> = Vec::new();
        for call in &self.calls {
            if call.primitive != "std.environment.get" {
                continue;
            }
            let Some((_, Json::Text(name))) = call.arguments.first() else {
                continue;
            };
            if environment.iter().any(|(seen, _)| seen == name) {
                continue;
            }
            let value = match &call.outcome {
                Outcome::Success(Json::Text(text)) if text == REDACTED => Some(text.clone()),
                Outcome::Success(Json::Text(text)) => Some(sha256_of(text.as_bytes())),
                _ => None,
            };
            environment.push((name.clone(), value));
        }
        self.manifest.environment = environment;
    }

    /// The recording as indented JSON text ending in a newline.
    pub fn render(&self) -> String {
        let mut out = String::new();
        write_json(&self.to_json(), &mut out, Some(2), 0);
        out.push('\n');
        out
    }

    pub fn parse(text: &str) -> Result<Recording, String> {
        let json = read_json(text).map_err(|(detail, line)| {
            format!("the recording is not JSON: {detail} (line {line})")
        })?;
        let Json::Object(fields) = &json else {
            return Err("the recording is not a JSON object".to_string());
        };
        let program = text_field(fields, "program")?;
        let revision = field(fields, "revision").and_then(|j| match j {
            Json::Text(text) => Some(text.clone()),
            _ => None,
        });
        let recorded_at = text_field(fields, "recorded_at")?;
        let grant = match field(fields, "grant") {
            Some(Json::Array(items)) => items
                .iter()
                .map(|item| match item {
                    Json::Text(text) => Ok(text.clone()),
                    _ => {
                        Err("the recording's `grant` holds something that is not text".to_string())
                    }
                })
                .collect::<Result<Vec<_>, _>>()?,
            _ => return Err("the recording has no `grant` list".to_string()),
        };
        let optional_text = |name: &str| match field(fields, name) {
            Some(Json::Text(text)) => Some(text.clone()),
            _ => None,
        };
        let manifest = Manifest {
            toolchain: optional_text("toolchain"),
            source: optional_text("source"),
            code: optional_text("code"),
            arguments: match field(fields, "arguments") {
                Some(Json::Array(items)) => items
                    .iter()
                    .filter_map(|item| match item {
                        Json::Text(text) => Some(text.clone()),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            },
            environment: match field(fields, "environment") {
                Some(Json::Object(entries)) => entries
                    .iter()
                    .map(|(name, value)| {
                        let value = match value {
                            Json::Text(text) => Some(text.clone()),
                            _ => None,
                        };
                        (name.clone(), value)
                    })
                    .collect(),
                _ => Vec::new(),
            },
            outcome: optional_text("outcome"),
            output: match field(fields, "output") {
                Some(Json::Object(entries)) => {
                    match (field(entries, "stdout"), field(entries, "bytes")) {
                        (Some(Json::Text(hash)), Some(Json::Number(bytes))) => {
                            bytes.parse().ok().map(|bytes| (hash.clone(), bytes))
                        }
                        _ => None,
                    }
                }
                _ => None,
            },
            dependencies: match field(fields, "dependencies") {
                Some(Json::Object(entries)) => entries
                    .iter()
                    .filter_map(|(name, value)| {
                        let Json::Object(inner) = value else {
                            return None;
                        };
                        match (field(inner, "version"), field(inner, "hash")) {
                            (Some(Json::Text(version)), Some(Json::Text(hash))) => {
                                Some(Dependency {
                                    name: name.clone(),
                                    version: version.clone(),
                                    hash: hash.clone(),
                                })
                            }
                            _ => None,
                        }
                    })
                    .collect(),
                _ => Vec::new(),
            },
            extensions: match field(fields, "extensions") {
                Some(Json::Array(items)) => items
                    .iter()
                    .filter_map(|item| match item {
                        Json::Text(text) => Some(text.clone()),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            },
        };
        let Some(Json::Array(entries)) = field(fields, "calls") else {
            return Err("the recording has no `calls` list".to_string());
        };
        let mut calls = Vec::with_capacity(entries.len());
        for (index, entry) in entries.iter().enumerate() {
            calls.push(
                Call::parse(entry).map_err(|detail| format!("call #{}: {detail}", index + 1))?,
            );
        }
        Ok(Recording {
            program,
            revision,
            recorded_at,
            grant,
            calls,
            manifest,
        })
    }
}

impl Call {
    fn to_json(&self) -> Json {
        let outcome = match &self.outcome {
            Outcome::Success(json) => vec![("success".to_string(), json.clone())],
            Outcome::Failure(json) => vec![("failure".to_string(), json.clone())],
        };
        let mut fields = vec![
            (
                "sequence".to_string(),
                Json::Number(self.sequence.to_string()),
            ),
            (
                "capability".to_string(),
                Json::Text(self.capability.clone()),
            ),
            ("primitive".to_string(), Json::Text(self.primitive.clone())),
            (
                "arguments".to_string(),
                Json::Object(self.arguments.clone()),
            ),
            ("outcome".to_string(), Json::Object(outcome)),
        ];
        if let Some(ms) = self.duration_ms {
            fields.push(("duration_ms".to_string(), Json::Number(ms.to_string())));
        }
        fields.push(("at_ms".to_string(), Json::Number(self.at_ms.to_string())));
        Json::Object(fields)
    }

    fn parse(json: &Json) -> Result<Call, String> {
        let Json::Object(fields) = json else {
            return Err("not an object".to_string());
        };
        let sequence = number_field(fields, "sequence")? as u64;
        let capability = text_field(fields, "capability")?;
        let primitive = text_field(fields, "primitive")?;
        let arguments = match field(fields, "arguments") {
            Some(Json::Object(arguments)) => arguments.clone(),
            None => Vec::new(),
            Some(_) => return Err("`arguments` is not an object".to_string()),
        };
        let outcome = match field(fields, "outcome") {
            Some(Json::Object(outcome)) => match outcome.as_slice() {
                [(key, value)] if key == "success" => Outcome::Success(value.clone()),
                [(key, value)] if key == "failure" => Outcome::Failure(value.clone()),
                _ => {
                    return Err(
                        "`outcome` is not {\"success\": ...} or {\"failure\": ...}".to_string()
                    )
                }
            },
            _ => return Err("no `outcome`".to_string()),
        };
        let duration_ms = match field(fields, "duration_ms") {
            Some(Json::Number(text)) => text.parse().ok(),
            _ => None,
        };
        let at_ms = match field(fields, "at_ms") {
            Some(Json::Number(text)) => text.parse().unwrap_or(0),
            _ => 0,
        };
        Ok(Call {
            sequence,
            capability,
            primitive,
            arguments,
            outcome,
            duration_ms,
            at_ms,
        })
    }

    /// `std.http.get(url: "https://...")`, long arguments clipped.
    pub fn describe(&self) -> String {
        describe_call(&self.primitive, &self.arguments)
    }
}

fn field<'a>(fields: &'a [(String, Json)], name: &str) -> Option<&'a Json> {
    fields
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
}

fn text_field(fields: &[(String, Json)], name: &str) -> Result<String, String> {
    match field(fields, name) {
        Some(Json::Text(text)) => Ok(text.clone()),
        _ => Err(format!("no `{name}` text")),
    }
}

fn number_field(fields: &[(String, Json)], name: &str) -> Result<i64, String> {
    match field(fields, name) {
        Some(Json::Number(text)) => text
            .parse()
            .map_err(|_| format!("`{name}` is not a whole number")),
        _ => Err(format!("no `{name}` number")),
    }
}

/// A call as a reader sees it: the primitive and its arguments as JSON,
/// each argument clipped.
pub fn describe_call(primitive: &str, arguments: &[(String, Json)]) -> String {
    let args: Vec<String> = arguments
        .iter()
        .map(|(name, value)| {
            let mut text = String::new();
            write_json(value, &mut text, None, 0);
            format!("{name}: {}", clip(&text, 80))
        })
        .collect();
    format!("{primitive}({})", args.join(", "))
}

/// The text cut to `width` characters with an ellipsis.
pub fn clip(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    let mut out: String = text.chars().take(width.saturating_sub(3)).collect();
    out.push_str("...");
    out
}

/// What a redacted value becomes in a recording (`renyi record --redact
/// NAME`, decision S3); a replay matches it against anything.
pub const REDACTED: &str = "<redacted>";

/// Redact a call before it is recorded: an argument with one of the
/// names, an entry with one of the names (ignoring case) inside a map
/// argument such as a header list, and the value of an environment
/// variable with one of the names.
pub fn redact(names: &[String], call: &mut Call) {
    if names.is_empty() {
        return;
    }
    let placeholder = || Json::Text(REDACTED.to_string());
    for (name, value) in &mut call.arguments {
        if names.contains(name) {
            *value = placeholder();
        } else if let Json::Object(entries) = value {
            for (key, entry) in entries {
                if names.iter().any(|name| name.eq_ignore_ascii_case(key)) {
                    *entry = placeholder();
                }
            }
        }
    }
    if call.primitive == "std.environment.get" {
        let variable = match call.arguments.first() {
            Some((_, Json::Text(text))) => text.as_str(),
            _ => "",
        };
        if names.iter().any(|name| name == variable) {
            if let Outcome::Success(json) = &mut call.outcome {
                if *json != Json::Null {
                    *json = placeholder();
                }
            }
        }
    }
}

/// Whether recorded arguments answer actual ones: the same names in the
/// same order, each value equal or redacted in the recording, wherever
/// the placeholder sits.
fn arguments_match(recorded: &[(String, Json)], actual: &[(String, Json)]) -> bool {
    recorded.len() == actual.len()
        && recorded
            .iter()
            .zip(actual)
            .all(|((a, va), (b, vb))| a == b && json_matches(va, vb))
}

fn json_matches(recorded: &Json, actual: &Json) -> bool {
    match (recorded, actual) {
        (Json::Text(text), _) if text == REDACTED => true,
        (Json::Object(a), Json::Object(b)) => arguments_match(a, b),
        (Json::Array(a), Json::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(va, vb)| json_matches(va, vb))
        }
        _ => recorded == actual,
    }
}

/// A replay: the recorded calls, each used at most once, matched by
/// primitive and arguments first and by sequence only among identical
/// entries (section 1.1 of the design).
pub struct Replay {
    pub recording: Recording,
    used: Vec<bool>,
}

impl Replay {
    pub fn new(recording: Recording) -> Replay {
        let used = vec![false; recording.calls.len()];
        Replay { recording, used }
    }

    /// The index of the entry that answers a call, now used; or why none
    /// does, naming the nearest recorded call.
    pub fn take(&mut self, primitive: &str, arguments: &[(String, Json)]) -> Result<usize, String> {
        let found = self
            .recording
            .calls
            .iter()
            .enumerate()
            .find(|(index, call)| {
                !self.used[*index]
                    && call.primitive == primitive
                    && arguments_match(&call.arguments, arguments)
            })
            .map(|(index, _)| index);
        match found {
            Some(index) => {
                self.used[index] = true;
                Ok(index)
            }
            None => Err(self.mismatch(primitive, arguments)),
        }
    }

    fn mismatch(&self, primitive: &str, arguments: &[(String, Json)]) -> String {
        let wanted = describe_call(primitive, arguments);
        let nearest = self
            .recording
            .calls
            .iter()
            .enumerate()
            .filter(|(index, _)| !self.used[*index])
            .map(|(_, call)| call)
            .min_by_key(|call| usize::from(call.primitive != primitive));
        match nearest {
            Some(call) => format!(
                "the recording has no call {wanted}; the nearest recorded call is #{} {}",
                call.sequence,
                call.describe()
            ),
            None => format!("the recording has no call {wanted}; every recorded call was used"),
        }
    }

    /// The entries no call used.
    pub fn unused(&self) -> Vec<&Call> {
        self.recording
            .calls
            .iter()
            .enumerate()
            .filter(|(index, _)| !self.used[*index])
            .map(|(_, call)| call)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(primitive: &str, text: &str) -> Call {
        Call {
            sequence: 0,
            capability: "console".to_string(),
            primitive: primitive.to_string(),
            arguments: vec![("text".to_string(), Json::Text(text.to_string()))],
            outcome: Outcome::Success(Json::Null),
            duration_ms: Some(1),
            at_ms: 5,
        }
    }

    #[test]
    fn redacted_values_are_placeholders_that_match_anything() {
        let names = vec!["Authorization".to_string(), "KEY".to_string()];
        let mut post = Call {
            sequence: 0,
            capability: "network.http(\"h\")".to_string(),
            primitive: "std.http.post".to_string(),
            arguments: vec![
                ("url".to_string(), Json::Text("https://h/".to_string())),
                ("body".to_string(), Json::Text("{}".to_string())),
                (
                    "headers".to_string(),
                    Json::Object(vec![(
                        "authorization".to_string(),
                        Json::Text("Bearer dummy".to_string()),
                    )]),
                ),
            ],
            outcome: Outcome::Success(Json::Null),
            duration_ms: None,
            at_ms: 0,
        };
        redact(&names, &mut post);
        assert_eq!(
            post.arguments[2].1,
            Json::Object(vec![(
                "authorization".to_string(),
                Json::Text(REDACTED.to_string())
            )])
        );
        assert_eq!(post.arguments[1].1, Json::Text("{}".to_string()));
        let mut get = Call {
            primitive: "std.environment.get".to_string(),
            arguments: vec![("name".to_string(), Json::Text("KEY".to_string()))],
            outcome: Outcome::Success(Json::Text("dummy".to_string())),
            ..post.clone()
        };
        redact(&names, &mut get);
        assert_eq!(
            get.outcome,
            Outcome::Success(Json::Text(REDACTED.to_string()))
        );
        assert_eq!(get.arguments[0].1, Json::Text("KEY".to_string()));

        let mut recording = Recording::new("demo", None, String::new(), Vec::new());
        recording.push(post);
        let mut replay = Replay::new(recording);
        let actual = vec![
            ("url".to_string(), Json::Text("https://h/".to_string())),
            ("body".to_string(), Json::Text("{}".to_string())),
            (
                "headers".to_string(),
                Json::Object(vec![(
                    "authorization".to_string(),
                    Json::Text("Bearer other".to_string()),
                )]),
            ),
        ];
        assert_eq!(replay.take("std.http.post", &actual), Ok(0));
    }

    #[test]
    fn the_manifest_round_trips_and_names_the_variables_read() {
        let variable = |name: &str, outcome: Json| Call {
            primitive: "std.environment.get".to_string(),
            arguments: vec![("name".to_string(), Json::Text(name.to_string()))],
            outcome: Outcome::Success(outcome),
            ..call("std.console.print", "x")
        };
        let mut recording = Recording::new(
            "demo",
            None,
            "t".to_string(),
            vec!["environment".to_string()],
        );
        recording.manifest.toolchain = Some("renyi 0.0.1".to_string());
        recording.manifest.code = Some("sha256:00".to_string());
        recording.manifest.arguments = vec!["Ada".to_string()];
        recording.push(variable("HOME", Json::Text("/home/ada".to_string())));
        recording.push(variable("KEY", Json::Text(REDACTED.to_string())));
        recording.push(variable("MISSING", Json::Null));
        recording.push(variable("HOME", Json::Text("/home/ada".to_string())));
        recording.finish("finished".to_string(), (sha256_of(b"hi\n"), 3));
        assert_eq!(
            recording.manifest.environment,
            vec![
                ("HOME".to_string(), Some(sha256_of(b"/home/ada"))),
                ("KEY".to_string(), Some(REDACTED.to_string())),
                ("MISSING".to_string(), None),
            ]
        );
        let parsed = Recording::parse(&recording.render()).unwrap();
        assert_eq!(parsed, recording);
        let manifest = recording.render_manifest();
        assert!(manifest.contains("\"code\": \"sha256:00\""), "{manifest}");
        assert!(manifest.contains("\"MISSING\": null"), "{manifest}");
        assert!(!manifest.contains("calls"), "{manifest}");
    }

    #[test]
    fn recordings_round_trip_and_replays_match_by_arguments() {
        let mut recording = Recording::new(
            "demo",
            Some("abc".to_string()),
            "2026-10-05T18:42:11Z".to_string(),
            vec!["console".to_string()],
        );
        recording.push(call("std.console.print", "a"));
        recording.push(call("std.console.print", "b"));
        recording.push(call("std.console.print", "a"));
        let text = recording.render();
        let parsed = Recording::parse(&text).unwrap();
        assert_eq!(parsed, recording);
        assert_eq!(parsed.calls[2].sequence, 3);

        let mut replay = Replay::new(parsed);
        let a = vec![("text".to_string(), Json::Text("a".to_string()))];
        assert_eq!(replay.take("std.console.print", &a), Ok(0));
        assert_eq!(replay.take("std.console.print", &a), Ok(2));
        let error = replay.take("std.console.print", &a).unwrap_err();
        assert_eq!(
            error,
            "the recording has no call std.console.print(text: \"a\"); the nearest recorded call is #2 std.console.print(text: \"b\")"
        );
        assert_eq!(replay.unused().len(), 1);
        assert!(Recording::parse("{\"program\": 1}").is_err());
    }
}
