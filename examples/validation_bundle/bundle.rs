//! The bundle format: a directory with `manifest.json` and the files it lists.
//!
//! Pure data handling: hashing, redaction, bounded command capture, manifest
//! writing and comparison. Nothing here knows about the reference interaction.

// Some helpers (`Bundle::json`, `CommandSpec::new`) serve callers that
// extend bundles; the tests use them.
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
    /// Where it runs; the current directory when `None`.
    pub cwd: Option<String>,
}

impl CommandSpec {
    pub fn new(argv: &[&str]) -> Self {
        Self::from_argv(argv.iter().map(|s| s.to_string()).collect())
    }

    pub fn from_argv(argv: Vec<String>) -> Self {
        Self {
            argv,
            env: Vec::new(),
            cwd: None,
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

/// The start and end of `bytes` as text, at most `max_bytes` of it, with the
/// number of bytes left out in between; and that number.
pub fn bounded_text(bytes: &[u8], max_bytes: usize) -> (String, u64) {
    let mut kept = Bounded::new(max_bytes);
    kept.push(bytes);
    (kept.text(), kept.omitted())
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
    if let Some(cwd) = &spec.cwd {
        cmd.current_dir(cwd);
    }
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

    /// A binary artifact (an image), stored as it is: nothing in it is
    /// redacted, so only pass bytes that hold no paths or secrets.
    pub fn bytes(&mut self, path: &str, kind: &'static str, bytes: Vec<u8>) {
        debug_assert!(
            !path.starts_with('/') && !path.contains(".."),
            "bundle paths are relative"
        );
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
        // A list of records (the commands): each by its place, so one
        // changed command shows as that command.
        Value::Array(items) if !items.is_empty() && items.iter().all(Value::is_object) => {
            for (i, v) in items.iter().enumerate() {
                flatten(&format!("{prefix}.{i}"), v, out);
            }
        }
        // Other lists (an argv, the features) compare whole.
        other => {
            out.insert(prefix.trim_start_matches('.').to_string(), other.clone());
        }
    }
}

/// The fields of a recorded command that say how it went, not what was
/// asked: whether a tool call was answered, how a build ended, how much
/// output was cut. They are outcomes of the run. A call that goes wrong for
/// a moment (the game exits while its last reply is on the way, a log read
/// races the exit) changes them without the setup having changed.
const COMMAND_OUTCOME_FIELDS: [&str; 4] = ["ok", "status", "exit_code", "omitted_output_bytes"];

/// Whether a flattened manifest path is a command's outcome field
/// (`commands.<n>.<field>`).
fn is_command_outcome(path: &str) -> bool {
    let mut parts = path.split('.');
    parts.next() == Some("commands")
        && parts.next().is_some_and(|n| n.parse::<usize>().is_ok())
        && parts.next().is_some_and(|f| COMMAND_OUTCOME_FIELDS.contains(&f))
        && parts.next().is_none()
}

fn differing_paths(a: &Value, b: &Value, sections: &[&str]) -> Vec<(String, Value, Value)> {
    let (mut fa, mut fb) = (BTreeMap::new(), BTreeMap::new());
    for section in sections {
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

/// What differs between two bundles in the sections that describe how a run
/// was made: `build`, `settings`, `scenario`, `commands`. Results (assertions,
/// artifacts) are deliberately not compared here: they are what the difference
/// is supposed to explain. Of a command, what was asked counts (the tool, its
/// arguments, an argv, a timeout); how it went does not, see
/// [`compare_command_outcomes`]. Each entry is `(path, value in a, value in
/// b)`; `null` means absent.
pub fn compare_setups(a: &Value, b: &Value) -> Vec<(String, Value, Value)> {
    differing_paths(a, b, &["build", "settings", "scenario", "commands"])
        .into_iter()
        .filter(|(path, _, _)| !is_command_outcome(path))
        .collect()
}

/// Commands that were the same in both bundles but went differently: a tool
/// call answered in one run and not in the other, a build that ended another
/// way. Not a setup difference. Each entry is `(path, what the command was,
/// value in a, value in b)`.
pub fn compare_command_outcomes(a: &Value, b: &Value) -> Vec<(String, String, Value, Value)> {
    differing_paths(a, b, &["commands"])
        .into_iter()
        .filter(|(path, _, _)| is_command_outcome(path))
        .map(|(path, va, vb)| {
            let index: usize = path.split('.').nth(1).and_then(|n| n.parse().ok()).unwrap_or(0);
            let command = [a, b]
                .iter()
                .map(|m| &m["commands"][index])
                .find(|c| c.is_object())
                .map_or_else(String::new, |c| match c["tool"].as_str() {
                    Some(tool) => format!("{} {tool}", c["role"].as_str().unwrap_or("command")),
                    None => c["role"].as_str().unwrap_or("command").to_string(),
                });
            (path, command, va, vb)
        })
        .collect()
}

// ------------------------------------------------------ reading and verifying

/// A reviewer's notes on a bundle's images, kept beside the manifest (never
/// in it: the manifest is what the run wrote).
pub const INSPECTIONS_FILE: &str = "inspections.json";
pub const INSPECTIONS_SCHEMA: &str = "ponko-validation-inspections";

/// Artifact kinds whose bytes differ between two runs of the same setup
/// (wall-clock timings, a process's own output, rendered pixels), so their
/// hashes say nothing about whether the runs agree.
const VOLATILE_KINDS: [&str; 5] = ["performance", "screenshot", "process-output", "game-log", "step-log"];

/// Reads `dir/manifest.json`.
pub fn read_manifest(dir: &Path) -> Result<Value, String> {
    let path = dir.join("manifest.json");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let manifest: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    if manifest["schema"] != SCHEMA {
        return Err(format!("{}: not a validation bundle manifest", path.display()));
    }
    match manifest["schema_version"].as_u64() {
        Some(v) if v <= SCHEMA_VERSION => Ok(manifest),
        v => Err(format!(
            "{}: schema version {v:?}, this tool reads up to {SCHEMA_VERSION}",
            path.display()
        )),
    }
}

fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, base, out);
        } else if let Ok(rel) = path.strip_prefix(base) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// What is wrong with the bundle in `dir` against its own manifest: a
/// listed file that is missing, has another size or hash, or a file the
/// manifest doesn't list. Empty when the bundle is intact.
pub fn verify(dir: &Path, manifest: &Value) -> Vec<String> {
    let mut problems = Vec::new();
    let mut listed = Vec::new();
    for a in manifest["artifacts"].as_array().into_iter().flatten() {
        let Some(path) = a["path"].as_str() else {
            problems.push("an artifact has no path".to_string());
            continue;
        };
        listed.push(path.to_string());
        if path.starts_with('/') || path.contains("..") {
            problems.push(format!("{path}: not a path inside the bundle"));
            continue;
        }
        match std::fs::read(dir.join(path)) {
            Err(_) => problems.push(format!("{path}: listed, but missing")),
            Ok(bytes) => {
                if a["bytes"].as_u64() != Some(bytes.len() as u64) {
                    problems.push(format!(
                        "{path}: {} bytes, the manifest says {}",
                        bytes.len(),
                        a["bytes"]
                    ));
                } else if a["sha256"].as_str() != Some(sha256_hex(&bytes).as_str()) {
                    problems.push(format!("{path}: its hash is not the manifest's"));
                }
            }
        }
    }
    let mut on_disk = Vec::new();
    walk(dir, dir, &mut on_disk);
    on_disk.sort();
    for file in on_disk {
        if file != "manifest.json" && file != INSPECTIONS_FILE && !listed.contains(&file) {
            problems.push(format!("{file}: in the directory, but not in the manifest"));
        }
    }
    problems
}

// ---------------------------------------------------------- visual evidence

/// The bundle's images with what a reviewer recorded about each: an image
/// counts as inspected only when a note in `inspections.json` names its
/// current hash. The tool never judges an image itself.
pub fn visual_evidence(dir: &Path, manifest: &Value) -> Vec<Value> {
    let notes: Value = std::fs::read_to_string(dir.join(INSPECTIONS_FILE))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or(Value::Null);
    manifest["artifacts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|a| a["kind"] == "screenshot")
        .map(|a| {
            let note = notes["inspections"]
                .as_array()
                .into_iter()
                .flatten()
                .rev()
                .find(|n| n["image"] == a["path"]);
            let inspection = match note {
                None => json!({ "status": "not_inspected" }),
                Some(n) if n["sha256"] != a["sha256"] => {
                    json!({ "status": "not_inspected", "reason": "the note is about another image (its hash differs)" })
                }
                Some(n) => json!({ "status": "inspected", "verdict": n["verdict"], "note": n["note"], "by": n["by"] }),
            };
            json!({ "path": a["path"], "sha256": a["sha256"], "inspection": inspection })
        })
        .collect()
}

/// Records that someone looked at `image` (a screenshot in the bundle) and
/// what they saw. The note names the image's hash, so it stops counting if
/// the image changes. Returns the note.
pub fn record_inspection(dir: &Path, image: &str, verdict: &str, note: &str, by: &str) -> Result<Value, String> {
    if !["ok", "wrong", "unclear"].contains(&verdict) {
        return Err(format!("verdict {verdict:?}: use ok, wrong or unclear"));
    }
    if note.trim().is_empty() {
        return Err("a note saying what the image shows is required".into());
    }
    let manifest = read_manifest(dir)?;
    let artifact = manifest["artifacts"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|a| a["path"] == image && a["kind"] == "screenshot")
        .ok_or_else(|| format!("{image}: not a screenshot in this bundle"))?;
    let bytes = std::fs::read(dir.join(image)).map_err(|e| format!("{image}: {e}"))?;
    if artifact["sha256"].as_str() != Some(sha256_hex(&bytes).as_str()) {
        return Err(format!("{image}: the file is not the one the manifest lists"));
    }
    let path = dir.join(INSPECTIONS_FILE);
    let mut all: Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .filter(|v: &Value| v["schema"] == INSPECTIONS_SCHEMA)
        .unwrap_or_else(|| json!({ "schema": INSPECTIONS_SCHEMA, "schema_version": 1, "inspections": [] }));
    let entry = json!({ "image": image, "sha256": artifact["sha256"], "verdict": verdict, "note": note, "by": by });
    all["inspections"]
        .as_array_mut()
        .ok_or("inspections.json is malformed")?
        .push(entry.clone());
    let mut text = serde_json::to_string_pretty(&all).expect("JSON value serializes");
    text.push('\n');
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(entry)
}

// ------------------------------------------------------- comparing two runs

fn failure_summary(manifest: &Value) -> Value {
    let f = &manifest["result"]["failure"];
    json!({
        "outcome": manifest["result"]["outcome"],
        "failure_kind": f["kind"],
        "failed_assertion": f["assertion"],
        "tick": f["tick"],
        "ticks_run": manifest["result"]["ticks_run"],
    })
}

/// Compares two bundles on disk: whether each is intact, what differs in
/// how the runs were made, whether they came out the same, which
/// deterministic artifacts differ, and what is known about their images.
///
/// Images are never compared pixel by pixel or by hash: the same scene
/// renders differently across GPUs, drivers and scale factors. The report
/// only says whether each image was inspected.
pub fn compare_bundles(dir_a: &Path, dir_b: &Path) -> Result<Value, String> {
    let (a, b) = (read_manifest(dir_a)?, read_manifest(dir_b)?);
    let setup: Vec<Value> = compare_setups(&a, &b)
        .into_iter()
        .map(|(path, va, vb)| json!({ "path": path, "a": va, "b": vb }))
        .collect();
    let outcomes: Vec<Value> = compare_command_outcomes(&a, &b)
        .into_iter()
        .map(|(path, command, va, vb)| {
            json!({
                "path": path,
                "command": command,
                "a": va,
                "b": vb,
                "reason": "how a command went, not what was asked: it can vary between runs of the same setup",
            })
        })
        .collect();
    let (ra, rb) = (failure_summary(&a), failure_summary(&b));

    let by_id = |m: &Value| -> BTreeMap<String, Value> {
        m["assertions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|x| Some((x["id"].as_str()?.to_string(), x.clone())))
            .collect()
    };
    let (aa, ab) = (by_id(&a), by_id(&b));
    let mut ids: Vec<&String> = aa.keys().chain(ab.keys()).collect();
    ids.sort();
    ids.dedup();
    let part = |x: Option<&Value>| {
        x.map_or(
            Value::Null,
            |x| json!({ "outcome": x["outcome"], "tick": x["tick"], "actual": x["actual"] }),
        )
    };
    let assertions: Vec<Value> = ids
        .into_iter()
        .filter_map(|id| {
            let (xa, xb) = (part(aa.get(id)), part(ab.get(id)));
            (xa != xb).then(|| json!({ "id": id, "a": xa, "b": xb }))
        })
        .collect();

    let files = |m: &Value| -> BTreeMap<String, Value> {
        m["artifacts"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|x| Some((x["path"].as_str()?.to_string(), x.clone())))
            .collect()
    };
    let (fa, fb) = (files(&a), files(&b));
    let mut paths: Vec<&String> = fa.keys().chain(fb.keys()).collect();
    paths.sort();
    paths.dedup();
    let (mut differing, mut not_compared) = (Vec::new(), Vec::new());
    for path in paths {
        let (xa, xb) = (fa.get(path), fb.get(path));
        let kind = xa.or(xb).map_or(Value::Null, |x| x["kind"].clone());
        if kind.as_str().is_some_and(|k| VOLATILE_KINDS.contains(&k)) {
            let reason = if kind == "screenshot" {
                "images are inspected, not compared: pixels differ across GPUs, drivers and scale factors"
            } else {
                "varies between runs of the same setup (timings or process output)"
            };
            not_compared.push(json!({ "path": path, "kind": kind, "reason": reason }));
        } else if xa.map(|x| &x["sha256"]) != xb.map(|x| &x["sha256"]) {
            differing.push(json!({
                "path": path,
                "kind": kind,
                "a": xa.map_or(Value::Null, |x| x["sha256"].clone()),
                "b": xb.map_or(Value::Null, |x| x["sha256"].clone()),
            }));
        }
    }

    let same_setup = setup.is_empty();
    let same_result = ra == rb && assertions.is_empty() && differing.is_empty();
    let verdict = match (same_setup, same_result) {
        (true, true) => "same setup, same result".to_string(),
        (true, false) => "same recorded setup, different result: the runs are not deterministic, or something that differs was not recorded".to_string(),
        (false, true) => format!("{} setup difference(s), same result", setup.len()),
        (false, false) => format!("{} setup difference(s) and a different result", setup.len()),
    };
    Ok(json!({
        "schema": "ponko-validation-comparison",
        "schema_version": 1,
        "integrity": { "a": verify(dir_a, &a), "b": verify(dir_b, &b) },
        "same_setup": same_setup,
        "same_result": same_result,
        "verdict": verdict,
        "setup_differences": setup,
        "command_outcome_differences": outcomes,
        "result": { "a": ra, "b": rb },
        "assertion_differences": assertions,
        "artifact_differences": differing,
        "not_compared": not_compared,
        "visual_evidence": {
            "a": visual_evidence(dir_a, &a),
            "b": visual_evidence(dir_b, &b),
            "note": "an image is evidence only once someone has looked at it; identical pixels are neither expected nor required",
        },
    }))
}

/// [`compare_bundles`]' report as text for a terminal.
pub fn comparison_text(report: &Value) -> String {
    let mut out = String::new();
    let short = |v: &Value| match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    for side in ["a", "b"] {
        let problems = report["integrity"][side].as_array().cloned().unwrap_or_default();
        if problems.is_empty() {
            out.push_str(&format!("bundle {side}: intact\n"));
        } else {
            out.push_str(&format!("bundle {side}: DAMAGED\n"));
            for p in &problems {
                out.push_str(&format!("  {}\n", short(p)));
            }
        }
    }
    out.push_str(&format!("verdict: {}\n", short(&report["verdict"])));
    let list = |title: &str, items: &Value, line: &dyn Fn(&Value) -> String, out: &mut String| {
        let items = items.as_array().cloned().unwrap_or_default();
        if !items.is_empty() {
            out.push_str(&format!("{title}:\n"));
            for item in &items {
                out.push_str(&format!("  {}\n", line(item)));
            }
        }
    };
    list(
        "setup differences",
        &report["setup_differences"],
        &|d| format!("{}: {} -> {}", short(&d["path"]), d["a"], d["b"]),
        &mut out,
    );
    for side in ["a", "b"] {
        let r = &report["result"][side];
        let mut line = format!("result {side}: {}", short(&r["outcome"]));
        if !r["failure_kind"].is_null() {
            line.push_str(&format!(" ({}", short(&r["failure_kind"])));
            if !r["failed_assertion"].is_null() {
                line.push_str(&format!(": {}", short(&r["failed_assertion"])));
            }
            if !r["tick"].is_null() {
                line.push_str(&format!(" at tick {}", r["tick"]));
            }
            line.push(')');
        }
        out.push_str(&line);
        out.push('\n');
    }
    list(
        "assertions that came out differently",
        &report["assertion_differences"],
        &|d| format!("{}: {} -> {}", short(&d["id"]), d["a"], d["b"]),
        &mut out,
    );
    list(
        "deterministic artifacts that differ",
        &report["artifact_differences"],
        &|d| format!("{} ({})", short(&d["path"]), short(&d["kind"])),
        &mut out,
    );
    list(
        "commands that went differently (not a setup difference)",
        &report["command_outcome_differences"],
        &|d| {
            format!(
                "{} ({}): {} -> {}",
                short(&d["path"]),
                short(&d["command"]),
                d["a"],
                d["b"]
            )
        },
        &mut out,
    );
    list(
        "not compared",
        &report["not_compared"],
        &|d| format!("{} ({}): {}", short(&d["path"]), short(&d["kind"]), short(&d["reason"])),
        &mut out,
    );
    for side in ["a", "b"] {
        list(
            &format!("images in {side}"),
            &report["visual_evidence"][side],
            &|i| {
                let ins = &i["inspection"];
                if ins["status"] == "inspected" {
                    format!(
                        "{}: inspected by {}, {}: {}",
                        short(&i["path"]),
                        short(&ins["by"]),
                        short(&ins["verdict"]),
                        short(&ins["note"])
                    )
                } else {
                    format!("{}: NOT INSPECTED, so it is no evidence yet", short(&i["path"]))
                }
            },
            &mut out,
        );
    }
    out
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
