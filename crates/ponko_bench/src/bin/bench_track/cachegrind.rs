//! Counting instructions with cachegrind (valgrind), and the document the
//! counts are written to.
//!
//! An instruction count doesn't depend on what else the machine is doing,
//! so it is stable on shared CI runners, where wall-clock times are not. It
//! is not a time: a change that only moves memory traffic, or only changes
//! how many threads help, doesn't show. The wall-clock runs on a named
//! machine (`ponko_bench`) cover those.

use std::path::Path;
use std::process::Command;

use serde_json::{Value, json};

use crate::workloads::{self, Workload};

/// `valgrind --version`, or an error that says how to get it.
pub fn version(valgrind: &str) -> Result<String, String> {
    let out = Command::new(valgrind).arg("--version").output().map_err(|e| {
        format!(
            "can't run {valgrind} ({e}). Instruction counts need valgrind, which runs on Linux \
             (`apt install valgrind`); elsewhere, compare wall-clock runs of ponko_bench instead"
        )
    })?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// The total instructions of one cachegrind output file: its `summary:` line.
pub fn summary(text: &str) -> Result<u64, String> {
    let line = text
        .lines()
        .find_map(|l| l.strip_prefix("summary:"))
        .ok_or("no summary line in the cachegrind output")?;
    let first = line.split_whitespace().next().ok_or("an empty summary line")?;
    first.parse().map_err(|_| format!("summary {first:?} is not a number"))
}

/// Instructions one run of `exe run <workload> <iterations> <0|1>` executes.
fn instructions(valgrind: &str, exe: &Path, w: &Workload, measured: bool) -> Result<u64, String> {
    let out_file = std::env::temp_dir().join(format!(
        "bench_track-{}-{}.cg.out",
        std::process::id(),
        u8::from(measured)
    ));
    let status = Command::new(valgrind)
        .args(["--tool=cachegrind", "--cache-sim=no", "--quiet"])
        .arg(format!("--cachegrind-out-file={}", out_file.display()))
        .arg(exe)
        .args([
            "run",
            w.name,
            &w.iterations.to_string(),
            if measured { "1" } else { "0" },
        ])
        // One thread, so the work is split the same way every run.
        .env("RAYON_NUM_THREADS", "1")
        .status()
        .map_err(|e| format!("can't run {valgrind}: {e}"))?;
    if !status.success() {
        return Err(format!("{}: the run under cachegrind failed ({status})", w.name));
    }
    let text = std::fs::read_to_string(&out_file).map_err(|e| format!("{}: {e}", out_file.display()))?;
    let _ = std::fs::remove_file(&out_file);
    summary(&text).map_err(|e| format!("{}: {e}", w.name))
}

/// Instructions one operation of `w` takes: the measured run minus the run
/// without the operation, divided by the iterations.
pub fn per_operation(valgrind: &str, exe: &Path, w: &Workload) -> Result<f64, String> {
    let without = instructions(valgrind, exe, w, false)?;
    let with = instructions(valgrind, exe, w, true)?;
    Ok(difference_per_operation(without, with, w.iterations))
}

fn difference_per_operation(without: u64, with: u64, iterations: usize) -> f64 {
    (with as f64 - without as f64) / iterations as f64
}

/// Build and machine context of a counting run.
pub struct Env {
    pub machine: String,
    pub rustc: String,
    pub git_rev: String,
    pub git_dirty: Option<bool>,
    pub commit_time: Option<u64>,
    pub cpu: String,
    pub valgrind: String,
    pub unix_time: u64,
}

impl Env {
    /// `machine`: the name given, `PONKO_BENCH_MACHINE`, or one made from
    /// the OS, architecture and CPU.
    pub fn detect(machine: Option<&str>, valgrind: &str) -> Self {
        let cpu = cpu_name();
        let machine = machine
            .map(str::to_owned)
            .or_else(|| std::env::var("PONKO_BENCH_MACHINE").ok())
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| crate::track::machine_slug(std::env::consts::OS, std::env::consts::ARCH, &cpu));
        Env {
            machine,
            rustc: command_line("rustc", &["--version"]).unwrap_or_else(|| "unknown".into()),
            git_rev: command_line("git", &["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".into()),
            git_dirty: Command::new("git")
                .args(["status", "--porcelain", "--untracked-files=no"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| !o.stdout.is_empty()),
            commit_time: command_line("git", &["log", "-1", "--format=%ct"]).and_then(|t| t.parse().ok()),
            cpu,
            valgrind: valgrind.to_owned(),
            unix_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
        }
    }
}

fn command_line(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let s = String::from_utf8(out.stdout).ok()?;
    s.lines().next().map(|l| l.trim().to_owned()).filter(|l| !l.is_empty())
}

fn cpu_name() -> String {
    if cfg!(target_os = "macos") {
        return command_line("sysctl", &["-n", "machdep.cpu.brand_string"]).unwrap_or_else(|| "unknown".into());
    }
    std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("model name"))
                .and_then(|l| l.split_once(':'))
                .map(|(_, v)| v.trim().to_owned())
        })
        .unwrap_or_else(|| "unknown".into())
}

/// The counts as a result document (`schema_version` 1, `env.method`
/// `"instructions"`), read back by `track::parse_doc`.
pub fn to_json(env: &Env, results: &[(&Workload, Vec<f64>)]) -> String {
    let results: Vec<Value> = results
        .iter()
        .map(|(w, samples)| {
            json!({
                "name": w.name,
                "kind": "instructions",
                "about": w.about,
                "iterations": w.iterations,
                "runs": samples.len(),
                "instructions": crate::track::median(samples),
                "spread": crate::track::spread(samples),
                "samples": samples,
            })
        })
        .collect();
    let doc = json!({
        "schema_version": 1,
        "env": {
            "method": "instructions",
            "machine": env.machine,
            "rustc": env.rustc,
            "git_rev": env.git_rev,
            "git_dirty": env.git_dirty,
            "commit_time": env.commit_time,
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "cpu": env.cpu,
            "debug_assertions": cfg!(debug_assertions),
            "valgrind": env.valgrind,
            "workloads": workloads::source_hash(),
            "unix_time": env.unix_time,
        },
        "results": results,
    });
    let mut s = serde_json::to_string_pretty(&doc).expect("a JSON value always serializes");
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_summary_line_is_the_total() {
        let out = "desc: I1 cache: ...\ncmd: bench_track run x 10 1\nevents: Ir\nfl=a.rs\n1 5\nsummary: 123456789\n";
        assert_eq!(summary(out), Ok(123_456_789));
        assert!(summary("events: Ir\n").is_err());
        assert!(summary("summary: lots\n").is_err());
    }

    #[test]
    fn the_operation_is_the_difference_per_iteration() {
        assert_eq!(difference_per_operation(1_000_000, 1_500_000, 50), 10_000.0);
    }

    #[test]
    fn the_document_reads_back_as_instruction_counts() {
        let env = Env {
            machine: "ci-x64".into(),
            rustc: "rustc 1.0".into(),
            git_rev: "abc".into(),
            git_dirty: Some(false),
            commit_time: Some(42),
            cpu: "cpu".into(),
            valgrind: "valgrind-3.22.0".into(),
            unix_time: 50,
        };
        let w = &workloads::WORKLOADS[0];
        let json = to_json(&env, &[(w, vec![10.0, 12.0, 11.0])]);
        let doc = crate::track::parse_doc(&json).unwrap();
        assert_eq!(doc.method, crate::track::Method::Instructions);
        assert_eq!((doc.machine.as_str(), doc.time), ("ci-x64", 42));
        assert_eq!(doc.workloads, Some(workloads::source_hash()));
        assert_eq!(doc.results, vec![(w.name.to_owned(), vec![10.0, 12.0, 11.0])]);
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["results"][0]["instructions"], 11.0);
    }
}
