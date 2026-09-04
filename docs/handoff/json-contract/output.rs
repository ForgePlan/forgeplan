//! The one shape every `--json` answer takes (ADR-024).
//!
//! Before this module there were **118 hand-rolled `serde_json::json!` blocks
//! across 36 command files and no shared helper** — so the forms diverged
//! (`health` returned an object, `list` a bare array), `_next_action` appeared
//! in 71 of them, and `--json` did not survive a failure at all:
//!
//! ```text
//! $ forgeplan get NOSUCH-999 --json
//! Error: Artifact 'NOSUCH-999' not found      # prose, exit 1
//! ```
//!
//! An agent asking for machine output got prose exactly when it most needed to
//! know what happened. That is the defect this module exists to remove, and the
//! reason the envelope is mandatory rather than a convention: a convention is
//! what produced the 118 variants.

use serde::Serialize;
use serde_json::Value;

use forgeplan_core::hints::{self, Hint};

/// Bumped on any change that an existing consumer could not read.
///
/// Cheap insurance bought after watching the opposite cost real time twice in
/// one day — a changed command string silently broke a name extracted by
/// slicing it (#351).
pub const SCHEMA_VERSION: u32 = 1;

/// Coarse machine-readable class for a failure.
///
/// Deliberately small. A rich taxonomy invites callers to branch on cases we
/// have not thought through; these four are the distinctions an agent can act
/// on differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// The named artifact, memory, claim or file does not exist.
    NotFound,
    /// Arguments or artifact body did not satisfy a rule.
    Invalid,
    /// A gate refused: lifecycle transition, ownership, methodology.
    Refused,
    /// Anything else — I/O, index, provider. Not the caller's fault to fix.
    Internal,
}

#[derive(Debug, Serialize)]
pub struct JsonError {
    pub message: String,
    pub kind: ErrorKind,
}

/// Every `--json` response, success or failure.
///
/// `ok` is the discriminator. An agent reads one field to know which arm it is
/// in, without consulting the exit code — which is unavailable when the output
/// travels through MCP or a log file. That is the whole reason for the wrapper
/// over a bare payload (ADR-024 option b, rejected).
#[derive(Debug, Serialize)]
pub struct Envelope {
    pub schema_version: u32,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonError>,
    /// Always present, `null` when there is genuinely no next step.
    ///
    /// Not `skip_serializing_if`: an agent must be able to read the key
    /// unconditionally. Absent-versus-null is a distinction nobody wants to
    /// encode, and PRD-071 stopped being selective here on purpose.
    pub _next_action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _alternative_action: Option<String>,
}

impl Envelope {
    fn ok_with(data: Value, next: Option<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            ok: true,
            data: Some(data),
            error: None,
            _next_action: next,
            _alternative_action: None,
        }
    }

    fn err_with(message: String, kind: ErrorKind, next: Option<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            ok: false,
            data: None,
            error: Some(JsonError { message, kind }),
            _next_action: next,
            _alternative_action: None,
        }
    }

    /// The `Or:` alternative, when one exists (PROB-095 shape).
    pub fn with_alternative(mut self, cmd: impl Into<String>) -> Self {
        self._alternative_action = Some(cmd.into());
        self
    }
}

/// Print a successful response.
pub fn emit(data: impl Serialize, hints: &[Hint]) -> anyhow::Result<()> {
    let value = serde_json::to_value(data)?;
    let env = Envelope::ok_with(value, hints::primary_action(hints));
    println!("{}", serde_json::to_string_pretty(&env)?);
    Ok(())
}

/// Print a successful response and its `Or:` alternative.
pub fn emit_with_alternative(
    data: impl Serialize,
    hints: &[Hint],
    alternative: impl Into<String>,
) -> anyhow::Result<()> {
    let value = serde_json::to_value(data)?;
    let env = Envelope::ok_with(value, hints::primary_action(hints)).with_alternative(alternative);
    println!("{}", serde_json::to_string_pretty(&env)?);
    Ok(())
}

/// Print a failure response. Does **not** exit — the caller decides, so this
/// stays usable from `main`'s top-level handler and from a command that wants
/// to continue.
///
/// The `Fix:` line is stripped from `message` when it has been lifted into
/// `_next_action`: repeating the remediation inside prose an agent is not
/// meant to parse invites it to parse the prose anyway.
pub fn emit_error(message: impl Into<String>, kind: ErrorKind, next: Option<String>) {
    let raw = message.into();
    let message = if next.is_some() {
        raw.lines()
            .filter(|l| !l.trim().starts_with("Fix: "))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    } else {
        raw
    };
    let env = Envelope::err_with(message, kind, next);
    match serde_json::to_string_pretty(&env) {
        Ok(s) => println!("{s}"),
        // Serialising a struct of owned Strings cannot realistically fail, but
        // swallowing the answer entirely would be worse than a plain line.
        Err(e) => println!(
            r#"{{"schema_version":{SCHEMA_VERSION},"ok":false,"error":{{"message":"failed to serialise error: {e}","kind":"internal"}},"_next_action":null}}"#
        ),
    }
}

/// Classify an `anyhow` error for the envelope.
///
/// Text matching, and stated as such: the command layer returns `anyhow`
/// rather than a typed error, so the class has to be recovered from the
/// message. Wrong guesses degrade to `Internal`, which is the honest default —
/// it says "not something you can fix by changing your arguments".
pub fn classify(err: &anyhow::Error) -> ErrorKind {
    let s = err.to_string().to_lowercase();
    if s.contains("not found") || s.contains("no such") || s.contains("does not exist") {
        ErrorKind::NotFound
    } else if s.contains("invalid") || s.contains("must be") || s.contains("expected") {
        ErrorKind::Invalid
    } else if s.contains("cannot ") || s.contains("refused") || s.contains("not held") {
        ErrorKind::Refused
    } else {
        ErrorKind::Internal
    }
}

/// Pull the `Fix:` line out of an error message, if the producer emitted one.
///
/// Commands already append `Fix: <command>` to their error text (PRD-071).
/// Lifting it into `_next_action` means the JSON path carries the same
/// remediation the human path does, instead of dropping it.
pub fn next_action_from_error(err: &anyhow::Error) -> Option<String> {
    err.to_string()
        .lines()
        .find_map(|l| l.trim().strip_prefix("Fix: ").map(str::to_string))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> serde_json::Value {
        serde_json::from_str(s).expect("emitted output must be valid JSON")
    }

    #[test]
    fn success_carries_the_discriminator_and_the_version() {
        let env = Envelope::ok_with(serde_json::json!({"id": "PRD-001"}), None);
        let v = parse(&serde_json::to_string(&env).unwrap());
        assert_eq!(v["ok"], true);
        assert_eq!(v["schema_version"], 1);
        assert_eq!(v["data"]["id"], "PRD-001");
        assert!(v.get("error").is_none(), "a success must not carry an error");
    }

    /// The reason the wrapper exists: an agent must tell success from failure
    /// from the payload alone, because the exit code is not visible through
    /// MCP or a log.
    #[test]
    fn failure_is_distinguishable_without_the_exit_code() {
        let env = Envelope::err_with("nope".into(), ErrorKind::NotFound, None);
        let v = parse(&serde_json::to_string(&env).unwrap());
        assert_eq!(v["ok"], false);
        assert_eq!(v["error"]["kind"], "not_found");
        assert_eq!(v["error"]["message"], "nope");
        assert!(v.get("data").is_none(), "a failure must not carry data");
    }

    /// `_next_action` is serialised even when empty. Absent-versus-null is a
    /// distinction no consumer wants to encode, and PRD-071 stopped being
    /// optional in JSON on purpose (PROB-099).
    #[test]
    fn next_action_key_is_always_present() {
        let env = Envelope::ok_with(serde_json::json!({}), None);
        let s = serde_json::to_string(&env).unwrap();
        assert!(
            s.contains("\"_next_action\":null"),
            "the key must be readable unconditionally, got: {s}"
        );
    }

    #[test]
    fn alternative_action_appears_only_when_set() {
        let plain = serde_json::to_string(&Envelope::ok_with(serde_json::json!({}), None)).unwrap();
        assert!(!plain.contains("_alternative_action"));

        let with =
            Envelope::ok_with(serde_json::json!({}), None).with_alternative("forgeplan list");
        let s = serde_json::to_string(&with).unwrap();
        assert!(s.contains("\"_alternative_action\":\"forgeplan list\""));
    }

    #[test]
    fn classify_maps_the_four_classes_and_defaults_to_internal() {
        let cases = [
            ("Artifact 'X' not found", ErrorKind::NotFound),
            ("invalid agent id", ErrorKind::Invalid),
            ("Cannot activate PRD-001: gates failed", ErrorKind::Refused),
            ("lance table write failed", ErrorKind::Internal),
        ];
        for (msg, want) in cases {
            assert_eq!(
                classify(&anyhow::anyhow!(msg.to_string())),
                want,
                "message: {msg}"
            );
        }
    }

    /// The human path already prints `Fix:`. Dropping it from JSON would make
    /// the machine surface strictly less useful than the prose one.
    #[test]
    fn the_fix_line_is_lifted_into_next_action() {
        let err = anyhow::anyhow!("Error: Artifact 'X' not found\nFix: forgeplan list");
        assert_eq!(
            next_action_from_error(&err).as_deref(),
            Some("forgeplan list")
        );
        assert_eq!(
            next_action_from_error(&anyhow::anyhow!("plain failure")),
            None
        );
    }


    /// The remediation belongs in `_next_action`, not repeated inside a prose
    /// message. Leaving it in both invites an agent to parse the message.
    #[test]
    fn the_fix_line_is_not_repeated_inside_the_message() {
        let mut captured = String::new();
        let env = Envelope::err_with(
            "Artifact 'X' not found".to_string(),
            ErrorKind::NotFound,
            Some("forgeplan list".to_string()),
        );
        captured.push_str(&serde_json::to_string(&env).unwrap());
        let v: serde_json::Value = serde_json::from_str(&captured).unwrap();
        assert!(!v["error"]["message"].as_str().unwrap().contains("Fix:"));
        assert_eq!(v["_next_action"], "forgeplan list");
    }
}
