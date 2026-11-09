//! The validation bundle example: a deliberate failure, the corrected run,
//! bounded step failures, secret and path redaction, and run comparison.
//!
//! The example's modules are compiled into this test, so the tests call the
//! same code `cargo run --example validation_bundle` runs.

#[path = "../examples/validation_bundle/bundle.rs"]
mod bundle;
#[path = "../examples/validation_bundle/orchestrate.rs"]
mod orchestrate;
#[path = "../examples/validation_bundle/reference.rs"]
mod reference;

use bundle::{CommandSpec, Redactor, compare_setups, sha256_hex};
use orchestrate::Options;
use reference::Variant;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// A fresh, missing directory under the target directory.
fn out_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("validation-bundle-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn read(dir: &Path, rel: &str) -> String {
    std::fs::read_to_string(dir.join(rel)).unwrap()
}

/// Every file in the bundle, relative to `dir`, with `/` separators.
fn files(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, base, out);
            } else {
                out.push(p.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// Every artifact the manifest lists exists and has the listed size and hash,
/// and the manifest lists every file but itself.
fn assert_artifacts_match(dir: &Path, manifest: &Value) {
    let listed: Vec<&str> = manifest["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["path"].as_str().unwrap())
        .collect();
    let mut on_disk = files(dir);
    on_disk.retain(|f| f != "manifest.json");
    assert_eq!(listed, on_disk);
    for a in manifest["artifacts"].as_array().unwrap() {
        let bytes = std::fs::read(dir.join(a["path"].as_str().unwrap())).unwrap();
        assert_eq!(a["bytes"], bytes.len());
        assert_eq!(a["sha256"], sha256_hex(&bytes), "{}", a["path"]);
    }
    let on_disk_manifest: Value = serde_json::from_str(&read(dir, "manifest.json")).unwrap();
    assert_eq!(&on_disk_manifest, manifest);
}

#[test]
fn sha256_matches_the_published_vectors() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    // 56 bytes: the padding spills into a second block.
    assert_eq!(
        sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

#[test]
fn the_buggy_run_writes_a_failing_bundle_that_points_at_the_tick() {
    let dir = out_dir("buggy");
    let report = orchestrate::run(&Options::new(&dir, Variant::Buggy)).unwrap();
    let m = &report.manifest;
    assert!(!report.passed);
    assert_eq!(
        (m["schema"].as_str(), m["schema_version"].as_u64()),
        (Some("ponko-validation-bundle"), Some(1))
    );
    assert_eq!(m["result"]["outcome"], "failed");
    assert_eq!(m["result"]["failure"]["kind"], "assertion");
    assert_eq!(m["result"]["failure"]["assertion"], "one-jump-per-press");
    // The second jump starts when the body lands again, 71 ticks in.
    assert_eq!(m["result"]["failure"]["tick"], 71);
    let failed: Vec<&str> = m["assertions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["outcome"] == "failed")
        .map(|a| a["id"].as_str().unwrap())
        .collect();
    assert_eq!(failed, ["one-jump-per-press", "lands-by-the-end"]);

    // Identity, scenario and the unavailable pieces are all said out loud.
    assert_eq!(m["build"]["engine_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(m["build"]["commit"]["status"], "unavailable");
    assert_eq!(
        (m["scenario"]["id"].as_str(), m["scenario"]["seed"].as_u64()),
        (Some("hold-jump"), Some(7))
    );
    assert_eq!(m["scenario"]["steps"].as_array().unwrap().len(), 2);
    assert_eq!(m["metrics"]["screenshots"]["status"], "unavailable");
    assert_eq!(m["metrics"]["performance"]["status"], "unavailable");
    assert_eq!(m["commands"][0]["role"], "reproduce");

    // The log names the second jump; the trace has a row per tick.
    assert!(read(&dir, "logs/run.log").contains("tick 71: jump #2 started"));
    assert_eq!(read(&dir, "trace.csv").lines().count(), 1 + 120);
    assert_artifacts_match(&dir, m);
}

#[test]
fn the_corrected_run_passes_and_the_comparison_names_what_changed() {
    let (bad_dir, good_dir) = (out_dir("cmp-buggy"), out_dir("cmp-fixed"));
    let mut bad = Options::new(&bad_dir, Variant::Buggy);
    bad.commit = Some("1111111".into());
    let mut good = Options::new(&good_dir, Variant::Fixed);
    good.commit = Some("2222222".into());
    let (bad, good) = (orchestrate::run(&bad).unwrap(), orchestrate::run(&good).unwrap());

    assert!(good.passed);
    assert_eq!(good.manifest["result"]["outcome"], "passed");
    assert_eq!(good.manifest["result"]["failure"], Value::Null);
    assert!(
        good.manifest["assertions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|a| a["outcome"] == "passed")
    );
    assert_artifacts_match(&good_dir, &good.manifest);

    let diff = compare_setups(&bad.manifest, &good.manifest);
    let paths: Vec<&str> = diff.iter().map(|(p, ..)| p.as_str()).collect();
    assert_eq!(paths, ["build.commit", "commands", "settings.variant"]);
    assert_eq!(diff[2].1, "buggy");
    assert_eq!(diff[2].2, "fixed");

    // Seed and feature differences show up too; identical setups show none.
    let mut other = Options::new(out_dir("cmp-seed"), Variant::Fixed);
    other.commit = Some("2222222".into());
    other.seed = 9;
    other.features = Some(vec!["audio".into()]);
    let other = orchestrate::run(&other).unwrap();
    let paths: Vec<String> = compare_setups(&good.manifest, &other.manifest)
        .into_iter()
        .map(|(p, ..)| p)
        .collect();
    assert_eq!(
        paths,
        ["build.features", "build.features_source", "commands", "scenario.seed"]
    );
    assert!(compare_setups(&good.manifest, &good.manifest).is_empty());
}

#[test]
fn the_same_inputs_give_the_same_bundle_bytes() {
    let (a, b) = (out_dir("same-a"), out_dir("same-b"));
    orchestrate::run(&Options::new(&a, Variant::Buggy)).unwrap();
    orchestrate::run(&Options::new(&b, Variant::Buggy)).unwrap();
    assert_eq!(files(&a), files(&b));
    for f in files(&a) {
        assert_eq!(
            std::fs::read(a.join(&f)).unwrap(),
            std::fs::read(b.join(&f)).unwrap(),
            "{f}"
        );
    }
}

#[test]
fn supplied_build_identity_is_recorded_and_repeats_in_the_reproduce_command() {
    let mut o = Options::new(out_dir("identity"), Variant::Fixed);
    o.commit = Some("abc1234".into());
    o.rustc = Some("rustc 1.95.0".into());
    o.features = Some(vec!["physics".into(), "3d".into()]);
    let m = orchestrate::run(&o).unwrap().manifest;
    assert_eq!(m["build"]["commit"], "abc1234");
    assert_eq!(m["build"]["rustc"], "rustc 1.95.0");
    assert_eq!(m["build"]["features"], json!(["physics", "3d"]));
    assert_eq!(m["build"]["features_source"], "supplied");
    let argv = m["commands"][0]["argv"].to_string();
    assert!(argv.contains("abc1234") && argv.contains("physics,3d"), "{argv}");
}

#[test]
fn a_bundle_directory_is_never_reused() {
    let dir = out_dir("reuse");
    orchestrate::run(&Options::new(&dir, Variant::Fixed)).unwrap();
    let err = orchestrate::run(&Options::new(&dir, Variant::Buggy))
        .err()
        .expect("second write is refused");
    assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
    // The first bundle is untouched.
    assert_eq!(
        serde_json::from_str::<Value>(&read(&dir, "manifest.json")).unwrap()["settings"]["variant"],
        "fixed"
    );
}

// ------------------------------------------------------------ step helpers

/// Run with `--exact helper_step` and a mode in the environment, this test
/// binary acts as the command the orchestrator runs: it fails, hangs or
/// floods its output. Without the variable it does nothing.
#[test]
fn helper_step() {
    let Ok(mode) = std::env::var("PONKO_BUNDLE_TEST_STEP") else {
        return;
    };
    match mode.as_str() {
        "fail" => {
            println!("   Compiling demo v0.1.0");
            eprintln!("error[E0425]: cannot find value `nope` in this scope");
            eprintln!("note: sent with control token=deadbeefcafe0123 from /home/alice/project/src/main.rs");
            std::process::exit(101);
        }
        "hang" => {
            println!("launching, waiting for the control endpoint");
            loop {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        "flood" => {
            for i in 0..20_000 {
                println!("line {i} of a very chatty build");
            }
            eprintln!("final error: the end");
            std::process::exit(1);
        }
        _ => {}
    }
}

fn step(mode: &str) -> CommandSpec {
    let exe = std::env::current_exe().unwrap().to_string_lossy().into_owned();
    let mut spec = CommandSpec::new(&[&exe, "--exact", "helper_step", "--nocapture", "--test-threads=1"]);
    spec.env.push(("PONKO_BUNDLE_TEST_STEP".into(), mode.into()));
    spec
}

#[test]
fn a_failed_step_ends_the_run_with_its_bounded_error_output() {
    let dir = out_dir("step-fail");
    let mut o = Options::new(&dir, Variant::Fixed);
    o.steps = vec![step("fail")];
    let report = orchestrate::run(&o).unwrap();
    let m = &report.manifest;
    assert!(!report.passed);
    assert_eq!(m["result"]["failure"]["kind"], "step_failed");
    assert_eq!(m["commands"][1]["status"], "failed");
    assert_eq!(m["commands"][1]["exit_code"], 101);
    assert_eq!(m["commands"][1]["env_names"], json!(["PONKO_BUNDLE_TEST_STEP"]));
    // No interaction ran after a failed build; that is stated, not implied.
    assert_eq!(m["assertions"]["status"], "unavailable");
    assert_eq!(m["result"]["ticks_run"], 0);
    let log = read(&dir, "logs/step-1.log");
    assert!(log.contains("cannot find value `nope`"), "{log}");
    // The control token in the output never reaches the bundle.
    assert!(!log.contains("deadbeefcafe0123"), "{log}");
    assert!(log.contains("token=<redacted>"), "{log}");
    assert!(!dir.join("logs/run.log").exists());
    assert_artifacts_match(&dir, m);
}

#[test]
fn a_step_that_hangs_is_killed_at_its_timeout() {
    let dir = out_dir("step-hang");
    let mut o = Options::new(&dir, Variant::Fixed);
    o.steps = vec![step("hang")];
    o.step_timeout = Duration::from_millis(600);
    let started = std::time::Instant::now();
    let report = orchestrate::run(&o).unwrap();
    assert!(started.elapsed() < Duration::from_secs(30));
    let m = &report.manifest;
    assert_eq!(m["result"]["failure"]["kind"], "step_timeout");
    assert_eq!(m["commands"][1]["status"], "timeout");
    assert_eq!(m["commands"][1]["timeout_secs"], 0);
    assert!(read(&dir, "logs/step-1.log").contains("waiting for the control endpoint"));
}

#[test]
fn a_missing_program_is_a_launch_failure_not_a_crash() {
    let dir = out_dir("step-missing");
    let mut o = Options::new(&dir, Variant::Fixed);
    o.steps = vec![CommandSpec::new(&["definitely-not-a-program-ponko"])];
    let m = orchestrate::run(&o).unwrap().manifest;
    assert_eq!(m["commands"][1]["status"], "launch_failed");
    assert_eq!(m["result"]["outcome"], "failed");
    assert!(read(&dir, "logs/step-1.log").contains("could not start"));
}

#[test]
fn step_output_is_bounded_and_keeps_the_start_and_the_end() {
    let dir = out_dir("step-flood");
    let mut o = Options::new(&dir, Variant::Fixed);
    o.steps = vec![step("flood")];
    o.step_output_limit = 2_000;
    let m = orchestrate::run(&o).unwrap().manifest;
    let log = read(&dir, "logs/step-1.log");
    assert!(log.len() < 6_000, "{} bytes", log.len());
    assert!(log.contains("line 0 of a very chatty build"));
    assert!(log.contains("bytes omitted"));
    assert!(log.contains("final error: the end"));
    assert!(m["commands"][1]["omitted_output_bytes"].as_u64().unwrap() > 100_000);
}

// --------------------------------------------------------------- redaction

#[test]
fn secrets_are_removed_from_text_and_json_without_touching_prose() {
    let r = Redactor::default();
    assert_eq!(r.text("connect token=abc123 now\n"), "connect token=<redacted> now\n");
    assert_eq!(
        r.text(r#"{"id": 1, "token": "abc123", "port": 5}"#),
        r#"{"id": 1, "token": "<redacted>", "port": 5}"#
    );
    assert_eq!(
        r.text("Authorization: Bearer abc.def.ghi\nnext"),
        "Authorization: <redacted>\nnext"
    );
    assert_eq!(r.text("PASSWORD: hunter2, ok"), "PASSWORD: <redacted>, ok");
    assert_eq!(r.text("the token was rejected\n"), "the token was rejected\n");
    assert_eq!(
        r.text("two token=a secret=b\n"),
        "two token=<redacted> secret=<redacted>\n"
    );

    // A registry entry as the control endpoint writes it: the token goes, the rest stays.
    let registry = json!({"name": "demo", "pid": 42, "port": 5000, "token": "0123456789abcdef", "launch_id": "x"});
    let clean = r.value(&registry);
    assert_eq!(
        clean,
        json!({"name": "demo", "pid": 42, "port": 5000, "token": "<redacted>", "launch_id": "x"})
    );
}

#[test]
fn user_paths_become_placeholders_and_the_rest_of_the_path_survives() {
    let r = Redactor::default()
        .path("/home/alice", "<HOME>")
        .path("/home/alice/project", "<CWD>")
        .path(r"C:\Users\alice", "<HOME>");
    // The longest prefix wins, and both slash styles are covered.
    assert_eq!(
        r.text("/home/alice/project/target/debug/demo"),
        "<CWD>/target/debug/demo"
    );
    assert_eq!(r.text("/home/alice/.cache/x"), "<HOME>/.cache/x");
    assert_eq!(r.text(r"C:\Users\alice\game\run.log"), r"<HOME>\game\run.log");
    assert_eq!(r.text("C:/Users/alice/game"), "<HOME>/game");
    assert_eq!(r.placeholders(), ["<CWD>", "<HOME>"]);
    assert!(!r.placeholders().iter().any(|p| p.contains("alice")));
}

#[test]
fn redacted_bundles_keep_their_reproduction_metadata_and_stay_consistent() {
    let dir = out_dir("redact");
    let mut o = Options::new(&dir, Variant::Buggy);
    o.redact_user_paths = true;
    o.commit = Some("abc1234".into());
    // A failing step whose command line holds this test binary's own path,
    // normally inside the working directory.
    o.steps = vec![step("fail")];
    let report = orchestrate::run(&o).unwrap();
    let m = &report.manifest;
    assert_eq!(m["build"]["commit"], "abc1234");
    assert_eq!(m["redaction"]["path_placeholders"].as_array().unwrap().len(), 3);
    let exe = m["commands"][1]["argv"][0].as_str().unwrap();
    assert!(exe.starts_with("<CWD>/") || exe.starts_with("<HOME>/"), "{exe}");
    assert!(
        exe.contains("validation_bundle"),
        "the rest of the path survives: {exe}"
    );
    for f in files(&dir) {
        let text = read(&dir, &f);
        for secret in [
            std::env::var("HOME").unwrap_or_default(),
            std::env::current_dir().unwrap().to_string_lossy().into_owned(),
        ] {
            if secret.len() > 1 {
                assert!(!text.contains(&secret), "{f} still holds {secret}");
            }
        }
    }
    // Hashes cover the redacted files, so the bundle still verifies.
    assert_artifacts_match(&dir, m);
}
