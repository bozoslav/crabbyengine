//! Writes a validation bundle: one directory that connects a build, a
//! scenario, the commands run, what was observed and hashes of every file.
//!
//! The reference interaction holds a jump button down in a headless game,
//! driven by a tick-indexed input scenario. `--variant buggy` fails an
//! assertion on purpose; `--variant fixed` passes. No window is opened.
//!
//!   `cargo run --example validation_bundle -- --out target/bundles/buggy --variant buggy`
//!   `cargo run --example validation_bundle -- --out target/bundles/fixed --variant fixed --commit $(git rev-parse HEAD) --redact-paths`
//!
//! Options: `--out DIR` (must be empty or missing), `--variant buggy|fixed`,
//! `--seed N`, `--commit SHA`, `--features a,b`, `--rustc VERSION`,
//! `--step "program args"` (repeatable; run first, bounded by
//! `--step-timeout-secs`), `--redact-paths`. Exits 1 when the run failed;
//! the bundle is written either way.

mod bundle;
mod orchestrate;
mod reference;

use bundle::CommandSpec;
use orchestrate::Options;
use reference::Variant;
use std::time::Duration;

fn parse(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut args = args;
    let mut o = Options::new("", Variant::Fixed);
    let mut out = None;
    while let Some(flag) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
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
            "--step" => {
                let line = value("--step")?;
                o.steps.push(CommandSpec {
                    argv: line.split_whitespace().map(String::from).collect(),
                    env: Vec::new(),
                });
            }
            "--step-timeout-secs" => {
                let secs = value("--step-timeout-secs")?
                    .parse()
                    .map_err(|e| format!("--step-timeout-secs: {e}"))?;
                o.step_timeout = Duration::from_secs(secs);
            }
            "--redact-paths" => o.redact_user_paths = true,
            other => return Err(format!("unknown option {other:?}")),
        }
    }
    o.out = out.ok_or("--out DIR is required")?.into();
    Ok(o)
}

fn main() {
    let options = match parse(std::env::args().skip(1)) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("validation_bundle: {e}");
            std::process::exit(2);
        }
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
