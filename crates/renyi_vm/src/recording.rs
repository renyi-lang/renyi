//! Recorded runs (decision P1, `06-runtime-guarantees.md` section 1): the
//! recording a run writes, one entry per primitive call made under a
//! capability, and the replay that answers calls from one. Arguments and
//! outcomes are JSON by the `ToJson` rules, so a recording is readable and
//! editable, and a replay decodes an outcome by the primitive's declared
//! types.

use crate::natives::json::{read_json, write_json, Json};

#[derive(Clone, Debug, PartialEq)]
pub struct Recording {
    pub program: String,
    pub revision: Option<String>,
    pub recorded_at: String,
    /// The grant the run executed under, one capability per entry.
    pub grant: Vec<String>,
    pub calls: Vec<Call>,
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
        }
    }

    /// Append a call; its sequence number is assigned here.
    pub fn push(&mut self, mut call: Call) {
        call.sequence = self.calls.len() as u64 + 1;
        self.calls.push(call);
    }

    pub fn to_json(&self) -> Json {
        let mut fields = vec![("program".to_string(), Json::Text(self.program.clone()))];
        if let Some(revision) = &self.revision {
            fields.push(("revision".to_string(), Json::Text(revision.clone())));
        }
        fields.push((
            "recorded_at".to_string(),
            Json::Text(self.recorded_at.clone()),
        ));
        fields.push((
            "grant".to_string(),
            Json::Array(self.grant.iter().cloned().map(Json::Text).collect()),
        ));
        fields.push((
            "calls".to_string(),
            Json::Array(self.calls.iter().map(Call::to_json).collect()),
        ));
        Json::Object(fields)
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
                !self.used[*index] && call.primitive == primitive && call.arguments == arguments
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
