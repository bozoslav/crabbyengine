//! Writes a validation bundle: one directory that connects a build, a
//! scenario, the commands run, what was observed and hashes of every file.
//!
//! The reference interaction holds a jump button down in a headless game,
//! driven by a tick-indexed input scenario. `--variant buggy` fails an
//! assertion on purpose; `--variant fixed` passes. No window is opened
//! unless you pass `--windowed`.
//!
//!   `cargo run --example validation_bundle -- --out target/bundles/buggy --variant buggy`
//!   `cargo run --example validation_bundle -- --out target/bundles/fixed --variant fixed --launch`
//!   `cargo run --example validation_bundle -- compare target/bundles/buggy target/bundles/fixed`
//!
//! Making a bundle: `--out DIR` (must be empty or missing), `--variant
//! buggy|fixed`, `--seed N`, `--step "program args"` (repeatable; run first,
//! bounded by `--step-timeout-secs`), `--redact-paths`, `--perf` (profile
//! the in-process run). The commit and compiler are found with `git` and
//! `rustc --version`; `--commit SHA`, `--rustc VERSION` and `--features a,b`
//! say them instead, `--no-capture` leaves them unavailable.
//!
//! `--launch` builds this example with `cargo build`, starts it as its own
//! process and drives it through the MCP tools (`launch`,
//! `run_input_scenario`, `run_ticks`, `read_logs`, ...). `--launch-program
//! PATH` starts a game that is already built. `--launch-cwd DIR`,
//! `--launch-example NAME`, `--launch-bin NAME`, `--launch-release` and
//! `--launch-arg ARG` and `--launch-env NAME=VALUE` (repeatable) choose what
//! is built and started; `--build-timeout-secs` and `--launch-timeout-secs`
//! bound it; `--windowed` opens a window and keeps a screenshot.
//!
//! `--scenario FILE` puts a launched game of your own (Rust, C or C++)
//! through an input-scenario file instead of the reference interaction: its
//! input steps are fed in and its check steps are the assertions.
//!
//! Exits 1 when the run failed; the bundle is written either way.
//!
//! Other commands: `compare A B [--json]` (what differs between two
//! bundles; exits 1 when anything does), `verify DIR` (the files against the
//! manifest), `inspect DIR --image PATH --verdict ok|wrong|unclear --note
//! TEXT [--by NAME]` (record that someone looked at a screenshot).

mod bundle;
mod identity;
mod launched;
mod orchestrate;
mod reference;

use bundle::CommandSpec;
use launched::Launch;
use orchestrate::Options;
use reference::Variant;
use std::path::Path;
use std::time::Duration;

/// What the command line asks for.
enum Command {
    Bundle(Box<Options>),
    /// The reference game as a process for a tool to drive (`--serve`).
    Serve {
        variant: Variant,
        seed: u64,
    },
    /// Does nothing for a while and exits: a game that never opens its
    /// control endpoint, for trying the launch timeout (`--idle SECS`).
    Idle(Duration),
}

/// One launch option, applied once the launch itself is known.
type LaunchEdit = Box<dyn FnOnce(&mut Launch)>;

fn parse(args: impl Iterator<Item = String>) -> Result<Command, String> {
    let mut args = args;
    let mut o = Options::new("", Variant::Fixed);
    o.capture_identity = true;
    let mut out = None;
    let (mut serve, mut idle) = (false, None);
    // Launch options may come in any order: collected, then applied.
    let mut launch: Option<Launch> = None;
    let mut launch_edits: Vec<LaunchEdit> = Vec::new();
    while let Some(flag) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        let secs = |name: &str, text: String| {
            text.parse()
                .map(Duration::from_secs)
                .map_err(|e| format!("{name}: {e}"))
        };
        match flag.as_str() {
            "--out" => out = Some(value("--out")?),
            "--variant" => {
                o.variant = match value("--variant")?.as_str() {
                    "buggy" => Variant::Buggy,
                    "fixed" => Variant::Fixed,
                    other => return Err(format!("unknown variant {other:?} (buggy or fixed)")),
                }
            }
            "--seed" => o.seed = value("--seed")?.parse().map_err(|e| format!("--seed: {e}"))?,
            "--commit" => o.commit = Some(value("--commit")?),
            "--rustc" => o.rustc = Some(value("--rustc")?),
            "--features" => {
                o.features = Some(
                    value("--features")?
                        .split(',')
                        .filter(|f| !f.is_empty())
                        .map(String::from)
                        .collect(),
                )
            }
            "--no-capture" => o.capture_identity = false,
            "--perf" => o.profile = true,
            "--step" => {
                let line = value("--step")?;
                o.steps.push(CommandSpec::from_argv(
                    line.split_whitespace().map(String::from).collect(),
                ));
            }
            "--step-timeout-secs" => o.step_timeout = secs("--step-timeout-secs", value("--step-timeout-secs")?)?,
            "--redact-paths" => o.redact_user_paths = true,
            "--launch" => launch = Some(launch.unwrap_or_else(Launch::cargo)),
            "--launch-program" => launch = Some(Launch::program(value("--launch-program")?)),
            "--launch-cwd" => {
                let v = value("--launch-cwd")?;
                launch_edits.push(Box::new(move |l| l.cwd = Some(v)));
            }
            "--launch-example" => {
                let v = value("--launch-example")?;
                launch_edits.push(Box::new(move |l| (l.example, l.bin) = (Some(v), None)));
            }
            "--launch-bin" => {
                let v = value("--launch-bin")?;
                launch_edits.push(Box::new(move |l| (l.example, l.bin) = (None, Some(v))));
            }
            "--launch-arg" => {
                let v = value("--launch-arg")?;
                launch_edits.push(Box::new(move |l| l.args.get_or_insert_default().push(v)));
            }
            "--launch-env" => {
                let v = value("--launch-env")?;
                let (name, value) = v.split_once('=').ok_or("--launch-env takes NAME=VALUE")?;
                let pair = (name.to_string(), value.to_string());
                launch_edits.push(Box::new(move |l| l.env.push(pair)));
            }
            "--scenario" => {
                let file = value("--scenario")?;
                let text = std::fs::read_to_string(&file).map_err(|e| format!("--scenario {file}: {e}"))?;
                o.scenario = Some(serde_json::from_str(&text).map_err(|e| format!("--scenario {file}: {e}"))?);
            }
            "--launch-release" => launch_edits.push(Box::new(|l| l.release = true)),
            "--windowed" => launch_edits.push(Box::new(|l| l.windowed = true)),
            "--build-timeout-secs" => {
                let d = secs("--build-timeout-secs", value("--build-timeout-secs")?)?;
                launch_edits.push(Box::new(move |l| l.build_timeout = d));
            }
            "--launch-timeout-secs" => {
                let d = secs("--launch-timeout-secs", value("--launch-timeout-secs")?)?;
                launch_edits.push(Box::new(move |l| l.launch_timeout = d));
            }
            "--serve" => serve = true,
            "--idle" => idle = Some(secs("--idle", value("--idle")?)?),
            other => return Err(format!("unknown option {other:?}")),
        }
    }
    if let Some(time) = idle {
        return Ok(Command::Idle(time));
    }
    if serve {
        return Ok(Command::Serve {
            variant: o.variant,
            seed: o.seed,
        });
    }
    if !launch_edits.is_empty() {
        let l = launch
            .as_mut()
            .ok_or("launch options need --launch or --launch-program")?;
        for edit in launch_edits {
            edit(l);
        }
    }
    o.launch = launch;
    o.out = out.ok_or("--out DIR is required")?.into();
    Ok(Command::Bundle(Box::new(o)))
}

/// `compare A B [--json]`: 0 when nothing differs, 1 when something does.
fn compare(args: &[String]) -> Result<i32, String> {
    let json = args.iter().any(|a| a == "--json");
    let dirs: Vec<&String> = args.iter().filter(|a| *a != "--json").collect();
    let [a, b] = dirs[..] else {
        return Err("usage: compare DIR_A DIR_B [--json]".into());
    };
    let report = bundle::compare_bundles(Path::new(a), Path::new(b))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("JSON value serializes")
        );
    } else {
        print!("{}", bundle::comparison_text(&report));
    }
    let intact = ["a", "b"]
        .iter()
        .all(|s| report["integrity"][s].as_array().is_some_and(Vec::is_empty));
    Ok(i32::from(
        !(intact && report["same_setup"] == true && report["same_result"] == true),
    ))
}

/// `verify DIR`: 0 when every file is the one the manifest lists.
fn verify(args: &[String]) -> Result<i32, String> {
    let [dir] = args else {
        return Err("usage: verify DIR".into());
    };
    let dir = Path::new(dir);
    let problems = bundle::verify(dir, &bundle::read_manifest(dir)?);
    if problems.is_empty() {
        println!("intact");
    }
    for p in &problems {
        println!("{p}");
    }
    Ok(i32::from(!problems.is_empty()))
}

/// `inspect DIR --image PATH --verdict V --note TEXT [--by NAME]`.
fn inspect(args: &[String]) -> Result<i32, String> {
    let mut args = args.iter();
    let dir = args
        .next()
        .ok_or("usage: inspect DIR --image PATH --verdict ok|wrong|unclear --note TEXT [--by NAME]")?;
    let (mut image, mut verdict, mut note, mut by) = (None, None, None, "unnamed".to_string());
    while let Some(flag) = args.next() {
        let value = args.next().ok_or(format!("{flag} needs a value"))?.clone();
        match flag.as_str() {
            "--image" => image = Some(value),
            "--verdict" => verdict = Some(value),
            "--note" => note = Some(value),
            "--by" => by = value,
            other => return Err(format!("unknown option {other:?}")),
        }
    }
    let entry = bundle::record_inspection(
        Path::new(dir),
        &image.ok_or("--image PATH is required")?,
        &verdict.ok_or("--verdict ok|wrong|unclear is required")?,
        &note.ok_or("--note TEXT is required")?,
        &by,
    )?;
    println!("recorded: {entry}");
    Ok(0)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let fail = |e: String| -> ! {
        eprintln!("validation_bundle: {e}");
        std::process::exit(2);
    };
    let sub = match args.first().map(String::as_str) {
        Some("compare") => Some(compare(&args[1..])),
        Some("verify") => Some(verify(&args[1..])),
        Some("inspect") => Some(inspect(&args[1..])),
        _ => None,
    };
    if let Some(done) = sub {
        std::process::exit(done.unwrap_or_else(|e| fail(e)));
    }
    let options = match parse(args.into_iter()).unwrap_or_else(|e| fail(e)) {
        Command::Idle(time) => return std::thread::sleep(time),
        Command::Serve { variant, seed } => return reference::serve(variant, seed, reference::SERVE_GIVE_UP),
        Command::Bundle(options) => options,
    };
    match orchestrate::run(&options) {
        Ok(report) => {
            println!("bundle: {}", options.out.display());
            println!(
                "outcome: {}",
                report.manifest["result"]["outcome"].as_str().unwrap_or("?")
            );
            if !report.passed {
                println!("failure: {}", report.manifest["result"]["failure"]);
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("validation_bundle: could not write the bundle: {e}");
            std::process::exit(2);
        }
    }
}
