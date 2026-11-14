//! Tracks benchmark results over time and flags regressions.
//!
//! ```sh
//! # Instruction counts of the tracked workloads (Linux with valgrind):
//! cargo build --release -p ponko_bench --bin bench_track
//! target/release/bench_track counts --repeat 3 --out new.json
//! # Compare two sets of runs (files, or folders of repeated runs):
//! target/release/bench_track compare base.json new.json
//! # The history folder's README (one table per machine):
//! target/release/bench_track history <folder> --write
//! ```
//!
//! See the crate README ("Tracking results over time") for the method, the
//! file layout and the CI workflow.

mod cachegrind;
mod track;
mod workloads;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use track::{Doc, Rule, Verdict};

const USAGE: &str = "\
usage:
  bench_track list
  bench_track run <workload> <iterations> <0|1>
  bench_track counts [--repeat N] [--machine NAME] [--valgrind PATH] [--out FILE] [FILTER]
  bench_track compare [--floor PCT] [--noise-factor F] [--any-machine] [--fail-on-regression]
                      [--base-label TEXT] [--new-label TEXT] BASE NEW
  bench_track history DIR [--limit N] [--write]

BASE and NEW are result documents, or folders of them (repeated runs).";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("list") => {
            for w in workloads::WORKLOADS {
                println!("{:<24} x{:<3} {}", w.name, w.iterations, w.about);
            }
            Ok(ExitCode::SUCCESS)
        }
        Some("run") => run(&args[1..]),
        Some("counts") => counts(&args[1..]),
        Some("compare") => compare(&args[1..]),
        Some("history") => history(&args[1..]),
        _ => Err(USAGE.to_owned()),
    };
    result.unwrap_or_else(|e| {
        eprintln!("bench_track: {e}");
        ExitCode::from(2)
    })
}

/// Options and positional arguments, in order.
struct Args {
    flags: Vec<(String, Option<String>)>,
    positional: Vec<String>,
}

impl Args {
    /// `with_value` lists the options that take a value.
    fn parse(args: &[String], with_value: &[&str], switches: &[&str]) -> Result<Self, String> {
        let mut out = Args {
            flags: Vec::new(),
            positional: Vec::new(),
        };
        let mut it = args.iter();
        while let Some(a) = it.next() {
            if with_value.contains(&a.as_str()) {
                let v = it.next().ok_or_else(|| format!("{a} needs a value"))?;
                out.flags.push((a.clone(), Some(v.clone())));
            } else if switches.contains(&a.as_str()) {
                out.flags.push((a.clone(), None));
            } else if a.starts_with("--") {
                return Err(format!("unknown option {a}\n{USAGE}"));
            } else {
                out.positional.push(a.clone());
            }
        }
        Ok(out)
    }

    fn value(&self, name: &str) -> Option<&str> {
        self.flags
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .and_then(|(_, v)| v.as_deref())
    }

    fn has(&self, name: &str) -> bool {
        self.flags.iter().any(|(n, _)| n == name)
    }

    fn number(&self, name: &str) -> Result<Option<f64>, String> {
        self.value(name)
            .map(|v| v.parse::<f64>().map_err(|_| format!("{name}: {v} is not a number")))
            .transpose()
    }
}

/// What cachegrind runs: one workload, with or without its operation.
fn run(args: &[String]) -> Result<ExitCode, String> {
    let [name, iterations, measured] = args else {
        return Err(USAGE.to_owned());
    };
    let w = workloads::find(name).ok_or_else(|| format!("no workload {name} (see `bench_track list`)"))?;
    let iterations: usize = iterations.parse().map_err(|_| "iterations must be a number")?;
    (w.run)(iterations, measured == "1");
    Ok(ExitCode::SUCCESS)
}

fn counts(args: &[String]) -> Result<ExitCode, String> {
    let a = Args::parse(args, &["--repeat", "--machine", "--valgrind", "--out"], &[])?;
    let repeat = a.number("--repeat")?.map_or(3, |r| r.max(1.0) as usize);
    let valgrind = a.value("--valgrind").unwrap_or("valgrind");
    let filter = a.positional.first().map(String::as_str);
    let exe = std::env::current_exe().map_err(|e| format!("can't find this program: {e}"))?;
    let version = cachegrind::version(valgrind)?;
    let env = cachegrind::Env::detect(a.value("--machine"), &version);
    let mut results = Vec::new();
    for w in workloads::WORKLOADS {
        if filter.is_some_and(|f| !f.split('|').any(|p| w.name.contains(p))) {
            continue;
        }
        let mut samples = Vec::with_capacity(repeat);
        for _ in 0..repeat {
            samples.push(cachegrind::per_operation(valgrind, &exe, w)?);
        }
        eprintln!(
            "{:<24} {:>14.0} instructions per operation (median of {repeat}, spread {:.2}%)",
            w.name,
            track::median(&samples),
            track::spread(&samples) * 100.0
        );
        results.push((w, samples));
    }
    let json = cachegrind::to_json(&env, &results);
    match a.value("--out") {
        Some(path) => std::fs::write(path, json).map_err(|e| format!("{path}: {e}"))?,
        None => print!("{json}"),
    }
    Ok(ExitCode::SUCCESS)
}

/// Every result document in `path` (a file, or the `.json` files of a folder).
fn load(path: &Path) -> Result<Vec<Doc>, String> {
    let files: Vec<PathBuf> = if path.is_dir() {
        let mut files: Vec<PathBuf> = std::fs::read_dir(path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        files.sort();
        files
    } else {
        vec![path.to_owned()]
    };
    if files.is_empty() {
        return Err(format!("{}: no .json files", path.display()));
    }
    files
        .iter()
        .map(|f| {
            let text = std::fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display()))?;
            track::parse_doc(&text).map_err(|e| format!("{}: {e}", f.display()))
        })
        .collect()
}

fn compare(args: &[String]) -> Result<ExitCode, String> {
    let a = Args::parse(
        args,
        &["--floor", "--noise-factor", "--base-label", "--new-label"],
        &["--any-machine", "--fail-on-regression"],
    )?;
    let [base, new] = a.positional.as_slice() else {
        return Err(USAGE.to_owned());
    };
    let base_side = track::pool(&load(Path::new(base))?).map_err(|e| format!("{base}: {e}"))?;
    let new_side = track::pool(&load(Path::new(new))?).map_err(|e| format!("{new}: {e}"))?;
    let mut rule = Rule::for_method(base_side.method);
    if let Some(p) = a.number("--floor")? {
        rule.floor = p / 100.0;
    }
    if let Some(f) = a.number("--noise-factor")? {
        rule.noise_factor = f;
    }
    let c = track::compare(&base_side, &new_side, rule, a.has("--any-machine"))?;
    let label = |flag: &str, side: &track::Side, path: &str| {
        a.value(flag).map_or_else(
            || match side.git_revs.as_slice() {
                [rev] if rev != "unknown" => rev.chars().take(10).collect(),
                _ => path.to_owned(),
            },
            str::to_owned,
        )
    };
    print!(
        "{}",
        track::render_comparison(
            &c,
            &label("--base-label", &base_side, base),
            &label("--new-label", &new_side, new)
        )
    );
    if a.has("--fail-on-regression") && c.count(Verdict::Slower) > 0 {
        return Ok(ExitCode::from(1));
    }
    Ok(ExitCode::SUCCESS)
}

/// The history folder holds `<machine>/<commit>.json` and
/// `releases/<tag>/<machine>.json`.
fn history(args: &[String]) -> Result<ExitCode, String> {
    let a = Args::parse(args, &["--limit"], &["--write"])?;
    let [dir] = a.positional.as_slice() else {
        return Err(USAGE.to_owned());
    };
    let limit = a.number("--limit")?.map_or(50, |l| l.max(1.0) as usize);
    let dir = Path::new(dir);
    let mut commits = Vec::new();
    let mut releases = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !path.is_dir() || name.starts_with('.') {
            continue;
        }
        if name == "releases" {
            for tag in std::fs::read_dir(&path).map_err(|e| e.to_string())? {
                let tag = tag.map_err(|e| e.to_string())?.path();
                if tag.is_dir() {
                    let label = tag.file_name().and_then(|n| n.to_str()).unwrap_or("").to_owned();
                    for d in load(&tag)? {
                        releases.push((label.clone(), d));
                    }
                }
            }
        } else {
            commits.extend(load(&path)?);
        }
    }
    let mut md = String::from(
        "# Benchmark history\n\nWritten by `bench_track history` (crates/ponko_bench, \"Tracking results \
         over time\"). One file per commit on `main` and machine: `<machine>/<commit>.json`; releases \
         in `releases/<tag>/`.\n\n",
    );
    for (machine, docs) in track::by_machine(commits) {
        md.push_str(&track::render_history(
            &machine,
            &docs,
            Rule::for_method,
            limit,
            "commit",
        ));
        md.push('\n');
    }
    if !releases.is_empty() {
        md.push_str("# Releases\n\n");
        let mut by_machine: std::collections::BTreeMap<String, Vec<Doc>> = Default::default();
        for (tag, mut d) in releases {
            // The tag stands in for the commit in the table.
            d.git_rev = tag;
            by_machine.entry(d.machine.clone()).or_default().push(d);
        }
        for (machine, mut docs) in by_machine {
            track::sort_history(&mut docs);
            md.push_str(&track::render_history(
                &machine,
                &docs,
                Rule::for_method,
                usize::MAX,
                "release",
            ));
            md.push('\n');
        }
    }
    if a.has("--write") {
        let path = dir.join("README.md");
        std::fs::write(&path, md).map_err(|e| format!("{}: {e}", path.display()))?;
    } else {
        print!("{md}");
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn options_and_positionals_parse_in_any_order() {
        let a = Args::parse(
            &strings(&["base", "--floor", "2", "new", "--any-machine"]),
            &["--floor"],
            &["--any-machine"],
        )
        .unwrap();
        assert_eq!(a.positional, ["base", "new"]);
        assert_eq!(a.number("--floor").unwrap(), Some(2.0));
        assert!(a.has("--any-machine"));
        assert!(Args::parse(&strings(&["--nope"]), &[], &[]).is_err());
        assert!(Args::parse(&strings(&["--floor"]), &["--floor"], &[]).is_err());
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bench_track-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn doc(rev: &str, time: u64, v: f64) -> String {
        format!(
            r#"{{"schema_version": 1, "env": {{"method": "instructions", "machine": "ci", "git_rev": "{rev}",
               "commit_time": {time}}}, "results": [{{"name": "ecs/iter/10k", "samples": [{v}, {v}, {v}]}}]}}"#
        )
    }

    #[test]
    fn the_history_folder_gets_a_readme_with_commits_and_releases() {
        let dir = scratch("history");
        std::fs::create_dir_all(dir.join("ci")).unwrap();
        std::fs::create_dir_all(dir.join("releases/v0.1.0")).unwrap();
        std::fs::write(dir.join("ci/aaa.json"), doc("aaa", 1, 100.0)).unwrap();
        std::fs::write(dir.join("ci/bbb.json"), doc("bbb", 2, 150.0)).unwrap();
        std::fs::write(dir.join("releases/v0.1.0/ci.json"), doc("aaa", 1, 100.0)).unwrap();
        let _ = history(&strings(&[dir.to_str().unwrap(), "--write"])).unwrap();
        let md = std::fs::read_to_string(dir.join("README.md")).unwrap();
        assert!(md.contains("## ci"), "{md}");
        assert!(md.contains("| `bbb` | 1970-01-01 | 150 ▲+50.0% |"), "{md}");
        assert!(md.contains("# Releases"), "{md}");
        assert!(md.contains("| release | date | `ecs/iter/10k` |"), "{md}");
        assert!(md.contains("| `v0.1.0` | 1970-01-01 | 100 |"), "{md}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_folder_of_repeated_runs_is_one_side() {
        let dir = scratch("folder");
        std::fs::write(dir.join("1.json"), doc("aaa", 1, 100.0)).unwrap();
        std::fs::write(dir.join("2.json"), doc("aaa", 1, 102.0)).unwrap();
        std::fs::write(dir.join("notes.txt"), "ignored").unwrap();
        let docs = load(&dir).unwrap();
        assert_eq!(docs.len(), 2);
        let side = track::pool(&docs).unwrap();
        assert_eq!(side.samples[0].1.len(), 6);
        assert!(load(&dir.join("missing")).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }
}
