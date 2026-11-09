//! The bundle format: a directory with `manifest.json` and the files it lists.
//!
//! Pure data handling: hashing, redaction, bounded command capture, manifest
//! writing and comparison. Nothing here knows about the reference interaction.

// `compare_setups`, `Bundle::json` and `CommandSpec::new` serve callers that
// compare or extend bundles; the tests use them.
#![allow(dead_code)]

use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, VecDeque};
use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Bumped when a manifest field is removed or changes meaning. Adding fields
/// doesn't bump it.
pub const SCHEMA_VERSION: u64 = 1;
pub const SCHEMA: &str = "ponko-validation-bundle";

/// A metric or artifact the run couldn't produce, said out loud instead of
/// being left out or filled with zeros.
pub fn unavailable(reason: &str) -> Value {
    json!({ "status": "unavailable", "reason": reason })
}

// ---------------------------------------------------------------- SHA-256

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98,
    0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
    0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8,
    0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819,
    0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
    0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
    0xc67178f2,
];

/// SHA-256 as lowercase hex. A few lines of std-only code instead of a
/// dependency; checked against the published test vectors in the tests.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for block in msg.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for (i, word) in block.as_chunks::<4>().0.iter().enumerate() {
            w[i] = u32::from_be_bytes(*word);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [
                t1.wrapping_add(t2),
                v[0],
                v[1],
                v[2],
                v[3].wrapping_add(t1),
                v[4],
                v[5],
                v[6],
            ];
        }
        for (a, b) in h.iter_mut().zip(v) {
            *a = a.wrapping_add(b);
        }
    }
    h.iter().map(|w| format!("{w:08x}")).collect()
}

// -------------------------------------------------------------- redaction

const SECRET_WORDS: [&str; 7] = [
    "token",
    "secret",
    "password",
    "passwd",
    "authorization",
    "api_key",
    "apikey",
];
const REDACTED: &str = "<redacted>";

/// Removes what must not leave the machine, before anything is written or
/// hashed: secrets (control tokens, passwords, authorization headers) always,
/// and user paths when prefixes are registered. A path prefix becomes a
/// placeholder (`<HOME>/x` stays recognizable and reproducible); a secret
/// value becomes `<redacted>`.
#[derive(Default, Clone)]
pub struct Redactor {
    prefixes: Vec<(String, String)>,
}

impl Redactor {
    /// Replaces `prefix` (in both slash styles) by `placeholder`.
    pub fn path(mut self, prefix: &str, placeholder: &str) -> Self {
        let prefix = prefix.trim_end_matches(['/', '\\']);
        if prefix.len() > 1 {
            for form in [prefix.to_string(), prefix.replace('\\', "/"), prefix.replace('/', "\\")] {
                if !self.prefixes.iter().any(|(p, _)| *p == form) {
                    self.prefixes.push((form, placeholder.to_string()));
                }
            }
            // Longest first, so `/home/u/proj` wins over `/home/u`.
            self.prefixes.sort_by_key(|(p, _)| std::cmp::Reverse(p.len()));
        }
        self
    }

    /// The placeholders in use (never the paths they replace).
    pub fn placeholders(&self) -> Vec<String> {
        let mut out: Vec<String> = self.prefixes.iter().map(|(_, ph)| ph.clone()).collect();
        out.sort();
        out.dedup();
        out
    }

    pub fn text(&self, text: &str) -> String {
        let mut out = text.to_string();
        for (prefix, placeholder) in &self.prefixes {
            if out.contains(prefix.as_str()) {
                out = out.replace(prefix.as_str(), placeholder);
            }
        }
        out.split_inclusive('\n').map(scrub_line).collect()
    }

    pub fn value(&self, value: &Value) -> Value {
        match value {
            Value::String(s) => Value::String(self.text(s)),
            Value::Array(items) => Value::Array(items.iter().map(|v| self.value(v)).collect()),
            Value::Object(map) => Value::Object(
                map.iter()
                    .map(|(k, v)| {
                        let lower = k.to_ascii_lowercase();
                        if SECRET_WORDS.iter().any(|w| lower.contains(w)) {
                            (k.clone(), json!(REDACTED))
                        } else {
                            (k.clone(), self.value(v))
                        }
                    })
                    .collect(),
            ),
            other => other.clone(),
        }
    }
}

/// Replaces the value after `word=`, `word: ` or `"word": "` on one line.
fn scrub_line(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    let mut out = String::with_capacity(line.len());
    let mut at = 0;
    while at < line.len() {
        // The earliest secret word at or after `at`.
        let found = SECRET_WORDS
            .iter()
            .filter_map(|w| lower[at..].find(w).map(|i| (at + i, *w)))
            .min_by_key(|(i, _)| *i);
        let Some((start, word)) = found else { break };
        let after = start + word.len();
        let rest = &line[after..];
        // Separators between the word and its value; one must be `:` or `=`.
        let sep_len = rest
            .find(|c: char| !matches!(c, '"' | '\'' | ':' | '=' | ' ' | '\t'))
            .unwrap_or(rest.len());
        let sep = &rest[..sep_len];
        if !sep.contains([':', '=']) || sep_len == rest.len() {
            out.push_str(&line[at..after]);
            at = after;
            continue;
        }
        let value_start = after + sep_len;
        let value = &line[value_start..];
        let value_len = if word == "authorization" {
            value.trim_end_matches(['\r', '\n']).len() // "Bearer abc" or "Basic abc": all of it
        } else {
            value
                .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | ',' | '}' | ')' | ';'))
                .unwrap_or(value.len())
        };
        out.push_str(&line[at..value_start]);
        out.push_str(REDACTED);
        at = value_start + value_len;
    }
    out.push_str(&line[at..]);
    out
}

// ---------------------------------------------------------------- commands

/// A command to run before the interaction (a build, a launch check).
#[derive(Clone, Debug)]
pub struct CommandSpec {
    pub argv: Vec<String>,
    /// Extra environment for the child. Only the names are recorded.
    pub env: Vec<(String, String)>,
}

impl CommandSpec {
    pub fn new(argv: &[&str]) -> Self {
        Self {
            argv: argv.iter().map(|s| s.to_string()).collect(),
            env: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandStatus {
    Passed,
    /// Exited non-zero (a failed build, for one).
    Failed(i32),
    /// Still running at the deadline; killed.
    TimedOut,
    /// Could not be started (missing program).
    LaunchFailed(String),
}

impl CommandStatus {
    pub fn name(&self) -> &'static str {
        match self {
            CommandStatus::Passed => "passed",
            CommandStatus::Failed(_) => "failed",
            CommandStatus::TimedOut => "timeout",
            CommandStatus::LaunchFailed(_) => "launch_failed",
        }
    }
}

pub struct CommandOutcome {
    pub status: CommandStatus,
    pub stdout: String,
    pub stderr: String,
    /// Bytes the command wrote beyond what is kept.
    pub omitted_bytes: u64,
}

/// Keeps the first and last bytes of a stream: build errors are at the end,
/// the cause is often at the start. Everything in between is counted, not kept.
struct Bounded {
    head: Vec<u8>,
    tail: VecDeque<u8>,
    half: usize,
    total: u64,
}

impl Bounded {
    fn new(max: usize) -> Self {
        Self {
            head: Vec::new(),
            tail: VecDeque::new(),
            half: (max / 2).max(1),
            total: 0,
        }
    }

    fn push(&mut self, bytes: &[u8]) {
        self.total += bytes.len() as u64;
        for &b in bytes {
            if self.head.len() < self.half {
                self.head.push(b);
            } else {
                if self.tail.len() == self.half {
                    self.tail.pop_front();
                }
                self.tail.push_back(b);
            }
        }
    }

    fn omitted(&self) -> u64 {
        self.total - (self.head.len() + self.tail.len()) as u64
    }

    fn text(&self) -> String {
        let mut bytes = self.head.clone();
        if self.omitted() > 0 {
            bytes.extend_from_slice(format!("\n[... {} bytes omitted ...]\n", self.omitted()).as_bytes());
        }
        bytes.extend(self.tail.iter());
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

fn drain<R: Read + Send + 'static>(mut from: R, into: Arc<Mutex<Bounded>>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        while let Ok(n) = from.read(&mut buf) {
            if n == 0 {
                break;
            }
            into.lock().unwrap_or_else(|e| e.into_inner()).push(&buf[..n]);
        }
    })
}

/// Runs `spec` for at most `timeout`, keeping at most `max_bytes` of each
/// output stream. A timed-out command is killed.
pub fn run_command(spec: &CommandSpec, timeout: Duration, max_bytes: usize) -> CommandOutcome {
    let failed = |status| CommandOutcome {
        status,
        stdout: String::new(),
        stderr: String::new(),
        omitted_bytes: 0,
    };
    let Some((program, args)) = spec.argv.split_first() else {
        return failed(CommandStatus::LaunchFailed("empty command".into()));
    };
    let mut cmd = Command::new(program);
    cmd.args(args)
        .envs(spec.env.iter().map(|(k, v)| (k, v)))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return failed(CommandStatus::LaunchFailed(e.to_string())),
    };
    let out = Arc::new(Mutex::new(Bounded::new(max_bytes)));
    let err = Arc::new(Mutex::new(Bounded::new(max_bytes)));
    let readers = [
        drain(child.stdout.take().expect("piped"), out.clone()),
        drain(child.stderr.take().expect("piped"), err.clone()),
    ];
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) if s.success() => break CommandStatus::Passed,
            Ok(Some(s)) => break CommandStatus::Failed(s.code().unwrap_or(-1)),
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                break CommandStatus::TimedOut;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(e) => break CommandStatus::LaunchFailed(e.to_string()),
        }
    };
    // A grandchild may keep the pipes open after the child is gone: give the
    // readers a moment, then take what they have.
    let grace = Instant::now() + Duration::from_secs(1);
    while readers.iter().any(|r| !r.is_finished()) && Instant::now() < grace {
        std::thread::sleep(Duration::from_millis(5));
    }
    let (out, err) = (
        out.lock().unwrap_or_else(|e| e.into_inner()),
        err.lock().unwrap_or_else(|e| e.into_inner()),
    );
    CommandOutcome {
        status,
        stdout: out.text(),
        stderr: err.text(),
        omitted_bytes: out.omitted() + err.omitted(),
    }
}

// ------------------------------------------------------------------ bundle

struct File {
    path: String,
    kind: &'static str,
    bytes: Vec<u8>,
}

/// Collects the files and manifest sections of one bundle, then writes them.
pub struct Bundle {
    redactor: Redactor,
    files: Vec<File>,
    sections: Map<String, Value>,
}

impl Bundle {
    pub fn new(redactor: Redactor) -> Self {
        Self {
            redactor,
            files: Vec::new(),
            sections: Map::new(),
        }
    }

    pub fn redactor(&self) -> &Redactor {
        &self.redactor
    }

    /// A manifest section (already structured). Redacted on the way in.
    pub fn section(&mut self, name: &str, value: Value) {
        self.sections.insert(name.to_string(), self.redactor.value(&value));
    }

    /// A text artifact. `path` is relative, with `/` separators.
    pub fn text(&mut self, path: &str, kind: &'static str, text: &str) {
        debug_assert!(
            !path.starts_with('/') && !path.contains(".."),
            "bundle paths are relative"
        );
        let bytes = self.redactor.text(text).into_bytes();
        self.files.push(File {
            path: path.to_string(),
            kind,
            bytes,
        });
    }

    pub fn json(&mut self, path: &str, kind: &'static str, value: &Value) {
        let mut text = serde_json::to_string_pretty(value).expect("JSON value serializes");
        text.push('\n');
        self.text(path, kind, &text);
    }

    /// Writes the files and `manifest.json` into `dir`, which must be empty
    /// or missing: a bundle is never mixed with older results. Returns the manifest.
    pub fn write(mut self, dir: &Path) -> io::Result<Value> {
        if dir.exists() && std::fs::read_dir(dir)?.next().is_some() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "bundle directory is not empty; choose a new one",
            ));
        }
        std::fs::create_dir_all(dir)?;
        self.files.sort_by(|a, b| a.path.cmp(&b.path));
        let mut artifacts = Vec::new();
        for file in &self.files {
            let target = dir.join(&file.path);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target, &file.bytes)?;
            artifacts.push(json!({
                "path": file.path,
                "kind": file.kind,
                "bytes": file.bytes.len(),
                "sha256": sha256_hex(&file.bytes),
            }));
        }
        let mut manifest = Map::new();
        manifest.insert("schema".into(), json!(SCHEMA));
        manifest.insert("schema_version".into(), json!(SCHEMA_VERSION));
        manifest.append(&mut self.sections);
        manifest.insert(
            "redaction".into(),
            json!({ "path_placeholders": self.redactor.placeholders(), "secrets": "values of token/secret/password/authorization/api_key fields and lines removed" }),
        );
        manifest.insert("artifacts".into(), Value::Array(artifacts));
        let manifest = Value::Object(manifest);
        let mut text = serde_json::to_string_pretty(&manifest).expect("manifest serializes");
        text.push('\n');
        std::fs::write(dir.join("manifest.json"), text)?;
        Ok(manifest)
    }
}

// -------------------------------------------------------------- comparison

fn flatten(prefix: &str, value: &Value, out: &mut BTreeMap<String, Value>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                flatten(&format!("{prefix}.{k}"), v, out);
            }
        }
        // Arrays (commands, steps) compare whole.
        other => {
            out.insert(prefix.trim_start_matches('.').to_string(), other.clone());
        }
    }
}

/// What differs between two bundles in the sections that describe how a run
/// was made: `build`, `settings`, `scenario`, `commands`. Results (assertions,
/// artifacts) are deliberately not compared here: they are what the difference
/// is supposed to explain. Each entry is `(path, value in a, value in b)`;
/// `null` means absent.
pub fn compare_setups(a: &Value, b: &Value) -> Vec<(String, Value, Value)> {
    let (mut fa, mut fb) = (BTreeMap::new(), BTreeMap::new());
    for section in ["build", "settings", "scenario", "commands"] {
        flatten(section, &a[section], &mut fa);
        flatten(section, &b[section], &mut fb);
    }
    let mut keys: Vec<&String> = fa.keys().chain(fb.keys()).collect();
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .filter_map(|k| {
            let (va, vb) = (
                fa.get(k).cloned().unwrap_or(Value::Null),
                fb.get(k).cloned().unwrap_or(Value::Null),
            );
            (va != vb).then(|| (k.clone(), va, vb))
        })
        .collect()
}

// -------------------------------------------------------------- assertions

/// One checked claim and what was observed. `tick` is where it failed (or
/// was decided), when that is a single tick.
#[derive(Clone, Debug)]
pub struct Assertion {
    pub id: String,
    pub description: String,
    pub passed: bool,
    pub tick: Option<u64>,
    pub expected: String,
    pub actual: String,
}

impl Assertion {
    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "description": self.description,
            "outcome": if self.passed { "passed" } else { "failed" },
            "tick": self.tick,
            "expected": self.expected,
            "actual": self.actual,
        })
    }
}
