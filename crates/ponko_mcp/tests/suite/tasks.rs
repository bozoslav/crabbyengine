//! The tasks: each has a starting project (files in `tasks/<id>/files`), a
//! goal for the agent (`goal.md`), a checker, a scripted solution
//! (`solution.json`) and negative controls (`controls.json`).

use super::games::{self, START};
use super::{Check, Probe};
use ponko::prelude::*;
use serde_json::{Value, json};

/// A task's game, and the setup it runs before the first tick.
pub type GameAndSetup = (Game, fn(&mut Ctx));

/// One task of the suite.
pub struct Task {
    /// Short name, used on the command line and in the baseline.
    pub id: &'static str,
    /// One line about it.
    pub title: &'static str,
    /// What kind of work it is: tuning, content, bug, performance, shader, ui.
    pub kind: &'static str,
    /// What the agent is told.
    pub goal: &'static str,
    /// The starting files, relative to the task's folder.
    pub files: &'static [(&'static str, &'static str)],
    /// The game, and its setup.
    pub game: fn() -> GameAndSetup,
    /// The checker's checks, in order.
    pub checks: &'static [&'static str],
    /// Scores the result.
    pub check: fn(&mut Probe) -> Vec<Check>,
    /// The scripted solution (JSON).
    pub solution: &'static str,
    /// The negative controls (JSON).
    pub controls: &'static str,
}

macro_rules! task_files {
    ($id:literal: $($path:literal),*) => {
        &[$(($path, include_str!(concat!("tasks/", $id, "/files/", $path)))),*]
    };
}

/// Every task, in the order they are scored.
pub static TASKS: &[Task] = &[
    Task {
        id: "tune-speed",
        title: "Tune the player's speed in a data file so a run takes the right time",
        kind: "tuning",
        goal: include_str!("tasks/tune-speed/goal.md"),
        files: task_files!("tune-speed": "assets/settings.json"),
        game: games::arena_game,
        checks: &[
            "settings file is valid",
            "game uses the file's speed",
            "flag reached in time",
        ],
        check: check_tune_speed,
        solution: include_str!("tasks/tune-speed/solution.json"),
        controls: include_str!("tasks/tune-speed/controls.json"),
    },
    Task {
        id: "coin-level",
        title: "Build a level with the tools and save it as a scene",
        kind: "content",
        goal: include_str!("tasks/coin-level/goal.md"),
        files: task_files!("coin-level": "assets/settings.json"),
        game: games::arena_game,
        checks: &[
            "level file saved",
            "level loads",
            "player and five coins",
            "holding right collects them all",
        ],
        check: check_coin_level,
        solution: include_str!("tasks/coin-level/solution.json"),
        controls: include_str!("tasks/coin-level/controls.json"),
    },
    Task {
        id: "immortal-enemy",
        title: "Reproduce and fix a seeded bug: an enemy that never dies",
        kind: "bug",
        goal: include_str!("tasks/immortal-enemy/goal.md"),
        files: task_files!("immortal-enemy": "assets/enemies.json"),
        game: games::enemies_game,
        checks: &[
            "roster file is fixed",
            "no live enemy heals",
            "a fresh wave dies in 5 seconds",
        ],
        check: check_immortal_enemy,
        solution: include_str!("tasks/immortal-enemy/solution.json"),
        controls: include_str!("tasks/immortal-enemy/controls.json"),
    },
    Task {
        id: "slow-crowd",
        title: "Find and cut a tick-time regression without changing the result",
        kind: "performance",
        goal: include_str!("tasks/slow-crowd/goal.md"),
        files: task_files!("slow-crowd": "assets/tuning.json"),
        game: games::crowd_game,
        checks: &[
            "radius unchanged",
            "game uses the file",
            "work cut 10x",
            "same pairs found",
        ],
        check: check_slow_crowd,
        solution: include_str!("tasks/slow-crowd/solution.json"),
        controls: include_str!("tasks/slow-crowd/controls.json"),
    },
    Task {
        id: "broken-shader",
        title: "Fix a sprite shader that doesn't compile, while the game runs",
        kind: "shader",
        goal: include_str!("tasks/broken-shader/goal.md"),
        files: task_files!("broken-shader": "assets/settings.json", "assets/shaders/glow.wgsl"),
        game: games::glow_game,
        checks: &["shader compiles in the game", "shader still glows"],
        check: check_broken_shader,
        solution: include_str!("tasks/broken-shader/solution.json"),
        controls: include_str!("tasks/broken-shader/controls.json"),
    },
    Task {
        id: "hidden-score",
        title: "Make the HUD show the score: what the player sees, checked headless",
        kind: "ui",
        goal: include_str!("tasks/hidden-score/goal.md"),
        files: task_files!("hidden-score": "assets/settings.json", "assets/hud.json"),
        game: games::hud_game,
        checks: &["score label shown", "label reads the score"],
        check: check_hidden_score,
        solution: include_str!("tasks/hidden-score/solution.json"),
        controls: include_str!("tasks/hidden-score/controls.json"),
    },
];

/// The player entity's id.
fn player(p: &mut Probe) -> Result<Value, String> {
    let r = p.call("list_entities", json!({ "with": ["Player"] }))?;
    match r["entities"].as_array().map(Vec::as_slice) {
        Some([one]) => Ok(one["entity"].clone()),
        _ => Err(format!("expected one Player, found {}", r["total"])),
    }
}

/// Puts the player back at the start, then holds Right for `ticks` ticks.
fn hold_right(p: &mut Probe, ticks: u64) -> Result<Value, String> {
    let id = player(p)?;
    p.call(
        "set_component",
        json!({ "entity": id, "component": "Transform2D", "value": { "position": [START.x, START.y] } }),
    )?;
    p.scenario(json!({
        "version": 1,
        "duration": ticks,
        "steps": [{ "tick": 0, "type": "key", "key": "Right", "down": true }]
    }))
}

fn number(v: &Value, what: &str) -> Result<f64, String> {
    v.as_f64().ok_or_else(|| format!("{what} is not a number: {v}"))
}

// ------------------------------------------------------------------ tune-speed

fn check_tune_speed(p: &mut Probe) -> Vec<Check> {
    let file = p
        .json_file("assets/settings.json")
        .and_then(|s| number(&s["speed"], "speed"));
    let mut checks = vec![Check::from(
        "settings file is valid",
        file.clone().map(|speed| format!("speed {speed}")),
    )];
    checks.push(Check::from(
        "game uses the file's speed",
        (|| {
            let file = file.clone()?;
            let live = number(&p.resource("Settings")?["speed"], "Settings.speed")?;
            if (live - file).abs() < 1e-3 {
                Ok(format!("{live}"))
            } else {
                Err(format!("the game runs at {live}, the file says {file}"))
            }
        })(),
    ));
    checks.push(Check::from(
        "flag reached in time",
        (|| {
            p.call(
                "set_resource",
                json!({ "resource": "Flag", "value": { "reached_tick": null } }),
            )?;
            let start = p.call("instance_info", json!({}))?["tick"]
                .as_u64()
                .ok_or("instance_info has no tick")?;
            hold_right(p, 130)?;
            let reached = p.resource("Flag")?["reached_tick"].as_u64();
            match reached.map(|t| t - start) {
                Some(ticks) if (91..=121).contains(&ticks) => Ok(format!("in {ticks} ticks")),
                Some(ticks) => Err(format!("in {ticks} ticks, wanted 91 to 121")),
                None => Err("not within 130 ticks".into()),
            }
        })(),
    ));
    checks
}

// ------------------------------------------------------------------ coin-level

const LEVEL: &str = "levels/coins.json";

fn check_coin_level(p: &mut Probe) -> Vec<Check> {
    let saved = p.json_file(&format!("assets/{LEVEL}"));
    let mut checks = vec![Check::from(
        "level file saved",
        saved.map(|_| format!("assets/{LEVEL}")),
    )];
    checks.push(Check::from(
        "level loads",
        p.call("load_scene", json!({ "path": LEVEL }))
            .map(|r| format!("{} entities", r["entities"])),
    ));
    checks.push(Check::from(
        "player and five coins",
        (|| {
            let coins = p.call("entity_stats", json!({ "fields": ["Coin.value"], "with": ["Coin"] }))?;
            let stats = &coins["fields"]["Coin.value"];
            let (count, min, max) = (coins["entities"].as_u64(), stats["min"].as_f64(), stats["max"].as_f64());
            if (count, min, max) != (Some(5), Some(1.0), Some(1.0)) {
                return Err(format!("wanted five coins worth 1, found {coins}"));
            }
            let id = player(p)?;
            let at = p.call("get_entity", json!({ "entity": id }))?["Transform2D"]["position"].clone();
            let at = (number(&at[0], "x")?, number(&at[1], "y")?);
            if (at.0 - f64::from(START.x)).abs() > 0.5 || (at.1 - f64::from(START.y)).abs() > 0.5 {
                return Err(format!("the player starts at {at:?}, not {START}"));
            }
            Ok("one player at the start, five coins".into())
        })(),
    ));
    checks.push(Check::from(
        "holding right collects them all",
        (|| {
            p.call("set_resource", json!({ "resource": "Score", "value": { "points": 0 } }))?;
            hold_right(p, 120)?;
            let points = p.resource("Score")?["points"].as_u64();
            let left = p.count(&["Coin"])?;
            match (points, left) {
                (Some(5), 0) => Ok("5 points in 2 seconds".into()),
                _ => Err(format!("{points:?} points, {left} coins left")),
            }
        })(),
    ));
    checks
}

// ------------------------------------------------------------------ immortal-enemy

/// The roster the task starts with: names and health must stay.
const ROSTER: &str = include_str!("tasks/immortal-enemy/files/assets/enemies.json");

fn check_immortal_enemy(p: &mut Probe) -> Vec<Check> {
    let mut checks = vec![Check::from(
        "roster file is fixed",
        (|| {
            let now = p.json_file("assets/enemies.json")?;
            let was: Value = serde_json::from_str(ROSTER).unwrap();
            let (now, was) = (
                now["enemies"].as_array().cloned().unwrap_or_default(),
                was["enemies"].as_array().unwrap(),
            );
            if now.len() != was.len() {
                return Err(format!("{} enemies, the roster had {}", now.len(), was.len()));
            }
            for (n, w) in now.iter().zip(was) {
                if n["name"] != w["name"] || n["health"] != w["health"] {
                    return Err(format!("{} changed more than its armor: {n}", w["name"]));
                }
                let armor = number(&n["armor"], "armor")?;
                if !(0.0..1.0).contains(&armor) {
                    return Err(format!("{} still has armor {armor}", n["name"]));
                }
            }
            Ok("every armor in [0, 1)".into())
        })(),
    )];
    checks.push(Check::from(
        "no live enemy heals",
        (|| {
            let r = p.call(
                "list_entities",
                json!({ "with": ["Enemy"], "where": ["Enemy.armor >= 1"] }),
            )?;
            match r["total"].as_u64() {
                Some(0) => Ok("none".into()),
                _ => Err(format!("{} enemies have armor >= 1", r["total"])),
            }
        })(),
    ));
    checks.push(Check::from(
        "a fresh wave dies in 5 seconds",
        (|| {
            let size = serde_json::from_str::<Value>(ROSTER).unwrap()["enemies"]
                .as_array()
                .unwrap()
                .len();
            p.call(
                "set_resource",
                json!({ "resource": "Waves", "value": { "wave": true } }),
            )?;
            p.call("subscribe_events", json!({ "types": ["Died"] }))?;
            p.call("run_ticks", json!({ "ticks": 300 }))?;
            let died = p.call("take_events", json!({}))?["events"]
                .as_array()
                .map_or(0, Vec::len);
            let left = p.count(&["Enemy"])?;
            if died == size && left == 0 {
                Ok(format!("{died} died"))
            } else {
                Err(format!("{died} of {size} died, {left} left"))
            }
        })(),
    ));
    checks
}

// ------------------------------------------------------------------ slow-crowd

/// The radius the crowd is measured with.
const RADIUS: f64 = 24.0;

fn check_slow_crowd(p: &mut Probe) -> Vec<Check> {
    let file = p.json_file("assets/tuning.json");
    let mut checks = vec![Check::from(
        "radius unchanged",
        (|| {
            let radius = number(&file.clone()?["radius"], "radius")?;
            if radius == RADIUS {
                Ok(format!("{radius}"))
            } else {
                Err(format!("radius is {radius}, was {RADIUS}"))
            }
        })(),
    )];
    checks.push(Check::from(
        "game uses the file",
        (|| {
            let file = file.clone()?;
            let live = p.resource("Tuning")?;
            if super::same(&live["radius"], &file["radius"]) && super::same(&live["cell_size"], &file["cell_size"]) {
                Ok(format!("{live}"))
            } else {
                Err(format!("the game uses {live}, the file says {file}"))
            }
        })(),
    ));
    let crowding = p
        .call("run_ticks", json!({ "ticks": 1 }))
        .and_then(|_| p.resource("Crowding"));
    let n = games::VILLAGERS as u64;
    let everything = n * (n - 1) / 2;
    checks.push(Check::from(
        "work cut 10x",
        (|| {
            let checks = crowding.clone()?["checks"].as_u64().ok_or("no Crowding.checks")?;
            if checks * 10 <= everything {
                Ok(format!("{checks} distance checks a tick (all pairs: {everything})"))
            } else {
                Err(format!("{checks} distance checks a tick; all pairs are {everything}"))
            }
        })(),
    ));
    checks.push(Check::from(
        "same pairs found",
        (|| {
            let pairs = crowding.clone()?["pairs"].as_u64().ok_or("no Crowding.pairs")?;
            // Counted here from the positions the game reports, all pairs.
            let r = p.call("list_entities", json!({ "with": ["Villager"], "limit": 10000 }))?;
            let at: Vec<(f64, f64)> = r["entities"]
                .as_array()
                .ok_or("no entities")?
                .iter()
                .map(|e| {
                    let pos = &e["components"]["Transform2D"]["position"];
                    (pos[0].as_f64().unwrap_or(f64::NAN), pos[1].as_f64().unwrap_or(f64::NAN))
                })
                .collect();
            let mut want = 0;
            for (i, a) in at.iter().enumerate() {
                for b in &at[i + 1..] {
                    if ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt() < RADIUS {
                        want += 1;
                    }
                }
            }
            if pairs == want {
                Ok(format!("{pairs} pairs"))
            } else {
                Err(format!("the game found {pairs} pairs, there are {want}"))
            }
        })(),
    ));
    checks
}

// ------------------------------------------------------------------ broken-shader

fn check_broken_shader(p: &mut Probe) -> Vec<Check> {
    let mut checks = vec![Check::from(
        "shader compiles in the game",
        (|| {
            let status = p.call("shader_status", json!({}))?;
            let glow = status["shaders"]
                .as_array()
                .and_then(|s| s.iter().find(|s| s["shader"] == games::GLOW_SHADER))
                .ok_or_else(|| format!("{} is not in shader_status: {status}", games::GLOW_SHADER))?;
            if glow["ok"] == true {
                Ok(format!("{}", glow["outcome"]))
            } else {
                Err(format!("{}: {}", glow["outcome"], glow["diagnostics"][0]["text"]))
            }
        })(),
    )];
    checks.push(Check::from(
        "shader still glows",
        (|| {
            let source = p.text_file(&format!("assets/{}", games::GLOW_SHADER))?;
            let code: String = source
                .lines()
                .map(|l| l.split("//").next().unwrap_or_default())
                .collect::<Vec<_>>()
                .join("\n");
            if code.contains("material.params[0]") && code.contains("fn fragment") {
                Ok("reads the glow parameter".into())
            } else {
                Err("the shader no longer reads material.params[0], the glow amount".into())
            }
        })(),
    ));
    checks
}

// ------------------------------------------------------------------ hidden-score

fn check_hidden_score(p: &mut Probe) -> Vec<Check> {
    let ui = |tick: u64, check: Value| {
        let mut step = json!({ "tick": tick, "type": "expect_ui", "node": "hud.score" });
        step.as_object_mut().unwrap().extend(check.as_object().unwrap().clone());
        json!({ "version": 1, "duration": tick + 1, "steps": [step] })
    };
    let mut checks = vec![Check::from(
        "score label shown",
        p.scenario(ui(1, json!({ "shown": true }))).map(|_| "shown".into()),
    )];
    checks.push(Check::from(
        "label reads the score",
        (|| {
            p.call("set_resource", json!({ "resource": "Score", "value": { "points": 7 } }))?;
            p.scenario(ui(1, json!({ "text": "Score: 7" })))?;
            Ok("\"Score: 7\" at 7 points".into())
        })(),
    ));
    checks
}
