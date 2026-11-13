//! The agent acceptance suite in its deterministic mode: every task's
//! scripted solution is replayed through the MCP tools against a fresh copy
//! of its starting project and must score full marks, and every negative
//! control (a plausible wrong fix) must fail exactly the checks it names.
//! No model is involved and nothing leaves this machine.
//!
//! The scores must match `tests/suite/baseline.json`, the published
//! baseline. After a deliberate change to a task, its checks or the tools,
//! rewrite it with `PONKO_UPDATE_BASELINE=1 cargo test -p ponko_mcp --test
//! agent_suite` and explain the change in the pull request.

mod suite;

use std::path::Path;

const BASELINE: &str = include_str!("suite/baseline.json");

// One test function: the tasks' games run in this process one after
// another, each in the process's working folder, and the environment is
// set before any of them starts.
#[test]
fn every_task_is_solvable_and_its_checker_catches_wrong_fixes() {
    let root = std::env::temp_dir().join(format!("ponko-agent-suite-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let instances = root.join("instances");
    std::fs::create_dir_all(&instances).unwrap();
    // SAFETY: the first thing this test binary does; no thread runs yet.
    unsafe { suite::prepare_process(&instances) };

    tasks_are_well_formed();
    let tasks: Vec<_> = suite::TASKS.iter().collect();
    let reports = suite::replay_all(&tasks, &root, &instances);
    println!("{}", suite::table(&reports));

    for r in &reports {
        let s = &r.solution;
        assert!(
            s.solved() && s.stuck.is_none(),
            "{}: the scripted solution should solve the task; stuck: {:?}; checks: {:#?}",
            r.task.id,
            s.stuck,
            s.checks
        );
        assert!(!r.controls.is_empty(), "{}: needs a negative control", r.task.id);
        assert!(
            r.controls.iter().any(|(script, _)| script.steps.is_empty()),
            "{}: needs a do-nothing control, so the starting state fails",
            r.task.id
        );
        for (script, outcome) in &r.controls {
            assert!(!outcome.solved(), "{}: control `{}` passed every check", r.task.id, script.name);
            assert_eq!(
                outcome.failed(),
                script.fails,
                "{}: control `{}` ({}) should fail exactly these checks; checks: {:#?}",
                r.task.id,
                script.name,
                script.why,
                outcome.checks
            );
        }
    }

    let now = suite::baseline(&reports);
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/suite/baseline.json");
    if std::env::var_os("PONKO_UPDATE_BASELINE").is_some() {
        std::fs::write(&file, serde_json::to_string_pretty(&now).unwrap() + "\n").unwrap();
    } else {
        let published: serde_json::Value = serde_json::from_str(BASELINE).unwrap();
        assert_eq!(
            now, published,
            "the scores differ from the published baseline (tests/suite/baseline.json); \
             if the change is deliberate, rerun with PONKO_UPDATE_BASELINE=1"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// Every task has a goal, starting files, uniquely named checks, and
/// controls that name only its checks.
fn tasks_are_well_formed() {
    let mut ids = std::collections::HashSet::new();
    for task in suite::TASKS {
        assert!(ids.insert(task.id), "{}: two tasks with this id", task.id);
        assert!(task.goal.contains("Goal:"), "{}: the goal says what to do after `Goal:`", task.id);
        assert!(!task.files.is_empty(), "{}: starts from no files", task.id);
        let names: std::collections::HashSet<_> = task.checks.iter().collect();
        assert_eq!(names.len(), task.checks.len(), "{}: two checks share a name", task.id);
        for control in task.controls() {
            for name in &control.fails {
                assert!(
                    task.checks.contains(&name.as_str()),
                    "{}: control `{}` names an unknown check `{name}`",
                    task.id,
                    control.name
                );
            }
        }
    }
}
