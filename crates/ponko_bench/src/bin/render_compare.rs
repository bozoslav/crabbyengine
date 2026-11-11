//! Compares two runs of the rendering baseline (`render/scene`), given
//! their JSON documents:
//!
//! ```sh
//! # From the repository root:
//! cargo run -p ponko_bench --bin render_compare -- \
//!   crates/ponko_bench/baselines/render.json new.json
//! ```
//!
//! It first checks that the two runs measured the same thing (the same
//! adapter, backend, driver, tier, resolution, sample count, quality, frame
//! counts and scenes, on the same machine). If they did, it prints both
//! medians of every step side by side. If not, it prints every difference
//! and no table, and exits with status 1. It never judges whether a change
//! is real: that needs the percentiles and the load both runs were taken
//! under.

#[allow(dead_code)]
#[path = "../render_report.rs"]
mod render_report;

fn load(path: &str) -> serde_json::Value {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("could not read {path}: {e}");
        std::process::exit(2);
    });
    let doc: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|e| {
        eprintln!("{path} is not JSON: {e}");
        std::process::exit(2);
    });
    if !doc["render"]["scenes"].is_array() {
        eprintln!("{path} has no render section: record it with `--features render -- render/scene`");
        std::process::exit(2);
    }
    doc
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [base, new] = args.as_slice() else {
        eprintln!("usage: render_compare <base.json> <new.json>");
        std::process::exit(2);
    };
    let (base, new) = (load(base), load(new));
    for (label, doc) in [("base", &base), ("new", &new)] {
        println!(
            "{label}: revision {}, load average {}, {} frames",
            doc["env"]["git_rev"].as_str().unwrap_or("unknown"),
            doc["env"]["load_average"].as_str().unwrap_or("unknown"),
            doc["render"]["settings"]["frames"]
        );
    }
    println!();
    match render_report::compare(&base, &new) {
        Ok(table) => {
            print!("{table}");
            println!(
                "\nSame adapter, settings and scenes. A difference smaller than the runs' own spread is not a result."
            );
        }
        Err(differences) => {
            println!("NOT COMPARABLE: the two runs differ in what they measured, so no table is printed:");
            for difference in differences {
                println!("- {difference}");
            }
            std::process::exit(1);
        }
    }
}
