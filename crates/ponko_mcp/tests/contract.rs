//! MCP contract compatibility: the tool names and parameters that agents
//! rely on, checked against a retained baseline instead of the server's own
//! current output.
//!
//! `tests/fixtures/tools_v1.json` is the `tools/list` reply of the server at a
//! recorded engine revision (see `tools_v1.provenance.json` and
//! `tests/fixtures/README.md`), with the descriptions left out: wording is not
//! part of the contract. STABILITY.md promises that minor releases only add
//! tools and optional parameters. The comparison here is additive-aware: new
//! tools and new optional parameters pass; a removed tool or parameter, a new
//! required parameter, or a parameter that accepts fewer values than before
//! fails with a message that names the tool and the parameter.
//!
//! Never regenerate the fixture to make a test pass.

use std::collections::BTreeSet;
use std::path::PathBuf;

use ponko_mcp::{Config, Server};
use serde_json::{Map, Value, json};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn load_fixture(name: &str) -> Value {
    let path = fixture_path(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("can't read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{name} is not JSON: {e}"))
}

/// The first retained baseline: tools, parameters, required lists, hints.
fn baseline() -> Value {
    load_fixture("tools_v1.json")
}

/// The second: the same, plus each tool's other top-level `inputSchema`
/// keywords (`type`, `additionalProperties`, ...), which v1 did not record.
fn baseline_v2() -> Value {
    load_fixture("tools_v2.json")
}

fn server(read_only: bool) -> Server {
    let dir = std::env::temp_dir().join("ponko-mcp-contract-unused");
    Server::new(Config {
        instances_dir: dir.clone(),
        logs_dir: dir.join("logs"),
        read_only,
    })
}

fn rpc(server: &mut Server, method: &str, params: Value) -> Value {
    server
        .handle(&json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }))
        .expect("requests get replies")["result"]
        .clone()
}

/// The server's `tools/list`, reduced to the contract: per tool its
/// parameters (without descriptions), required list and safety hints.
fn current_contract(read_only: bool) -> Value {
    let reply = rpc(&mut server(read_only), "tools/list", json!({}));
    let tools: Vec<Value> = reply["tools"]
        .as_array()
        .expect("tools/list returns a tools array")
        .iter()
        .map(contract_of_tool)
        .collect();
    json!({ "tools": tools })
}

fn contract_of_tool(t: &Value) -> Value {
    let mut properties = Map::new();
    for (name, schema) in t["inputSchema"]["properties"].as_object().expect("properties") {
        properties.insert(name.clone(), strip_descriptions(schema));
    }
    // Every other keyword of the tool's own input schema: `additionalProperties: false`
    // there would reject arguments that were accepted before.
    let schema: Map<String, Value> = t["inputSchema"]
        .as_object()
        .expect("inputSchema")
        .iter()
        .filter(|(k, _)| !matches!(k.as_str(), "properties" | "required" | "description"))
        .map(|(k, v)| (k.clone(), strip_descriptions(v)))
        .collect();
    json!({
        "name": t["name"],
        "required": t["inputSchema"]["required"],
        "properties": properties,
        "schema": schema,
        "read_only": t["annotations"]["readOnlyHint"],
    })
}

fn strip_descriptions(schema: &Value) -> Value {
    match schema {
        Value::Object(m) => Value::Object(
            m.iter()
                .filter(|(k, _)| k.as_str() != "description")
                .map(|(k, v)| (k.clone(), strip_descriptions(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

// ---------------------------------------------------------------- comparison

fn type_set(schema: &Value) -> BTreeSet<String> {
    match &schema["type"] {
        Value::String(s) => [s.clone()].into(),
        Value::Array(a) => a.iter().filter_map(|v| v.as_str().map(String::from)).collect(),
        _ => BTreeSet::new(),
    }
}

/// Keywords where a larger value in the new schema accepts less.
const LOWER_BOUNDS: &[&str] = &["minimum", "exclusiveMinimum", "minLength", "minItems"];
/// Keywords where a smaller value in the new schema accepts less.
const UPPER_BOUNDS: &[&str] = &["maximum", "exclusiveMaximum", "maxLength", "maxItems"];

/// `additionalProperties` widening and narrowing. Absent and `true` both accept
/// any extra key; `false` accepts none; a schema accepts values it allows.
fn additional_properties(path: &str, old: Option<&Value>, new: Option<&Value>, out: &mut Vec<String>) {
    let open = |v: Option<&Value>| matches!(v, None | Some(Value::Bool(true)));
    let closed = |v: Option<&Value>| matches!(v, Some(Value::Bool(false)));
    if open(new) || closed(old) {
        return; // accepts everything it accepted, or nothing was accepted before
    }
    if closed(new) {
        out.push(format!(
            "{path}: extra properties are no longer accepted (additionalProperties: false)"
        ));
    } else if open(old) {
        out.push(format!("{path}: extra properties are now restricted by a schema"));
    } else if let (Some(o), Some(n)) = (old, new) {
        narrowing(&format!("{path} (extra properties)"), o, n, out);
    }
}

/// Reasons why `new` accepts something `old` accepted but no longer does.
/// Accepting more (a wider type list, a larger enum, a looser bound) is fine.
fn narrowing(path: &str, old: &Value, new: &Value, out: &mut Vec<String>) {
    // Types: every old type must still be accepted ("integer" is also a "number").
    let (old_types, new_types) = (type_set(old), type_set(new));
    if !old_types.is_empty() {
        let accepted = |t: &String| new_types.contains(t) || (t == "integer" && new_types.contains("number"));
        if new_types.is_empty() {
            // no type at all accepts everything
        } else if let Some(lost) = old_types.iter().find(|t| !accepted(t)) {
            out.push(format!("{path}: no longer accepts type `{lost}`"));
        }
    } else if !new_types.is_empty() {
        out.push(format!("{path}: now restricted to type {new_types:?}"));
    }

    // Enumerations: a new `enum` must contain every previously accepted value.
    match (old.get("enum"), new.get("enum")) {
        (_, None) => {}
        (None, Some(_)) => out.push(format!("{path}: values are now restricted to an enum")),
        (Some(o), Some(n)) => {
            let n = n.as_array().cloned().unwrap_or_default();
            for v in o.as_array().cloned().unwrap_or_default() {
                if !n.contains(&v) {
                    out.push(format!("{path}: enum value {v} is no longer accepted"));
                }
            }
        }
    }

    for k in LOWER_BOUNDS.iter().chain(UPPER_BOUNDS) {
        let upper = UPPER_BOUNDS.contains(k);
        match (old.get(*k).and_then(Value::as_f64), new.get(*k).and_then(Value::as_f64)) {
            (_, None) => {}
            (None, Some(n)) => out.push(format!("{path}: new bound `{k}` = {n}")),
            (Some(o), Some(n)) if (upper && n < o) || (!upper && n > o) => {
                out.push(format!("{path}: `{k}` tightened from {o} to {n}"));
            }
            _ => {}
        }
    }

    // Array items accept what they accepted.
    match (old.get("items"), new.get("items")) {
        (Some(o), Some(n)) => narrowing(&format!("{path}[]"), o, n, out),
        (None, Some(_)) => out.push(format!("{path}: items are now restricted")),
        _ => {}
    }

    // Extra keys of an object: absent or `true` accepts anything, `false` accepts none,
    // a schema accepts what it describes.
    additional_properties(
        path,
        old.get("additionalProperties"),
        new.get("additionalProperties"),
        out,
    );

    // Any other constraint keyword may only stay as it was or disappear.
    let known = ["type", "enum", "items", "description", "additionalProperties"];
    let mut constraints = BTreeSet::new();
    for schema in [old, new] {
        for k in schema.as_object().into_iter().flat_map(|m| m.keys()) {
            if !known.contains(&k.as_str())
                && !LOWER_BOUNDS.contains(&k.as_str())
                && !UPPER_BOUNDS.contains(&k.as_str())
            {
                constraints.insert(k.clone());
            }
        }
    }
    for k in constraints {
        match (old.get(&k), new.get(&k)) {
            (_, None) => {}
            (o, Some(n)) if o != Some(n) => out.push(format!("{path}: constraint `{k}` changed or was added")),
            _ => {}
        }
    }
}

/// What a client written against `old` (a contract) can no longer do against
/// `new`. Empty means `new` is a compatible evolution of `old`.
fn incompatibilities(old: &Value, new: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let new_tools: Vec<&Value> = new["tools"].as_array().expect("tools").iter().collect();
    for old_tool in old["tools"].as_array().expect("tools") {
        let name = old_tool["name"].as_str().expect("tool name");
        let Some(new_tool) = new_tools.iter().find(|t| t["name"] == name) else {
            out.push(format!("tool `{name}` was removed"));
            continue;
        };
        let old_required: BTreeSet<&str> = old_tool["required"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        for r in new_tool["required"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !old_required.contains(r) {
                out.push(format!("tool `{name}`: parameter `{r}` became required"));
            }
        }
        let new_props = new_tool["properties"].as_object().expect("properties");
        for (param, old_schema) in old_tool["properties"].as_object().expect("properties") {
            match new_props.get(param) {
                None => out.push(format!("tool `{name}`: parameter `{param}` was removed")),
                Some(new_schema) => narrowing(
                    &format!("tool `{name}`: parameter `{param}`"),
                    old_schema,
                    new_schema,
                    &mut out,
                ),
            }
        }
        // The tool's own input schema, where the baseline recorded it (v2 on).
        if let Some(old_schema) = old_tool.get("schema") {
            narrowing(
                &format!("tool `{name}`: input schema"),
                old_schema,
                &new_tool["schema"],
                &mut out,
            );
        }
        if old_tool["read_only"] == true && new_tool["read_only"] != true {
            out.push(format!("tool `{name}` was read-only and now changes the game"));
        }
    }
    out
}

fn tool_mut<'a>(contract: &'a mut Value, name: &str) -> &'a mut Value {
    contract["tools"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|t| t["name"] == name)
        .unwrap_or_else(|| panic!("no tool {name}"))
}

fn expect_one(found: Vec<String>, fragment: &str) {
    assert!(
        found.iter().any(|m| m.contains(fragment)),
        "expected a failure mentioning {fragment:?}, got {found:?}"
    );
}

// --------------------------------------------------- the server against it

#[test]
fn server_still_offers_every_baseline_tool_and_parameter() {
    for (file, base) in [("tools_v1.json", baseline()), ("tools_v2.json", baseline_v2())] {
        let found = incompatibilities(&base, &current_contract(false));
        assert!(
            found.is_empty(),
            "the MCP tool contract changed incompatibly with the retained baseline \
             (tests/fixtures/{file}); fix the server, or make an explicit \
             compatibility decision (STABILITY.md), never edit the fixture:\n  {}",
            found.join("\n  ")
        );
    }
}

#[test]
fn baseline_covers_the_whole_tool_surface() {
    // A guard against an emptied fixture making the comparison vacuous.
    let b = baseline();
    let tools = b["tools"].as_array().unwrap();
    assert!(tools.len() >= 30, "only {} tools in the baseline", tools.len());
    for must in [
        "list_instances",
        "world_summary",
        "get_entity",
        "set_component",
        "run_ticks",
        "launch",
    ] {
        assert!(tools.iter().any(|t| t["name"] == must), "baseline lacks {must}");
    }
    let set = tools.iter().find(|t| t["name"] == "set_component").unwrap();
    assert_eq!(set["required"], json!(["entity", "component", "value"]));
}

#[test]
fn read_only_mode_hides_exactly_the_baseline_writing_tools() {
    let b = baseline();
    let visible = current_contract(true);
    let visible: Vec<&str> = visible["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    for t in b["tools"].as_array().unwrap() {
        let name = t["name"].as_str().unwrap();
        let read_only = t["read_only"] == true;
        assert_eq!(
            visible.contains(&name),
            read_only,
            "`{name}` is {} in the baseline but {} with --read-only",
            if read_only { "read-only" } else { "writing" },
            if visible.contains(&name) { "visible" } else { "hidden" }
        );
    }
}

#[test]
fn initialize_accepts_every_baseline_protocol_version() {
    let b = baseline();
    for v in b["protocol_versions"].as_array().unwrap() {
        let reply = rpc(&mut server(false), "initialize", json!({ "protocolVersion": v }));
        assert_eq!(
            &reply["protocolVersion"], v,
            "protocol version {v} is no longer negotiated"
        );
        assert_eq!(reply["serverInfo"]["name"], b["server_name"]);
        assert_eq!(reply["capabilities"]["tools"]["listChanged"], false);
    }
}

#[test]
fn calling_a_baseline_tool_without_its_required_parameters_is_a_tool_error() {
    // The error surface a client sees stays an `isError` result, not a
    // protocol failure, for every tool that has required parameters.
    let b = baseline();
    for t in b["tools"].as_array().unwrap() {
        if t["required"].as_array().unwrap().is_empty() {
            continue;
        }
        let r = rpc(
            &mut server(false),
            "tools/call",
            json!({ "name": t["name"], "arguments": {} }),
        );
        assert_eq!(
            r["isError"], true,
            "{} accepted a call without its required parameters",
            t["name"]
        );
    }
}

// --------------------------------------- the comparison itself (synthetic)

#[test]
fn identical_contracts_are_compatible() {
    assert!(incompatibilities(&baseline(), &baseline()).is_empty());
}

#[test]
fn a_removed_tool_is_incompatible() {
    let mut now = baseline();
    now["tools"]
        .as_array_mut()
        .unwrap()
        .retain(|t| t["name"] != "get_entity");
    expect_one(incompatibilities(&baseline(), &now), "tool `get_entity` was removed");
}

#[test]
fn a_removed_parameter_is_incompatible() {
    let mut now = baseline();
    tool_mut(&mut now, "list_entities")["properties"]
        .as_object_mut()
        .unwrap()
        .remove("offset");
    expect_one(
        incompatibilities(&baseline(), &now),
        "`list_entities`: parameter `offset` was removed",
    );
}

#[test]
fn a_parameter_that_became_required_is_incompatible() {
    let mut now = baseline();
    tool_mut(&mut now, "list_entities")["required"] = json!(["with"]);
    expect_one(
        incompatibilities(&baseline(), &now),
        "`list_entities`: parameter `with` became required",
    );
}

#[test]
fn a_new_required_parameter_on_an_existing_tool_is_incompatible() {
    let mut now = baseline();
    let t = tool_mut(&mut now, "get_entity");
    t["properties"]["generation"] = json!({ "type": "integer" });
    t["required"] = json!(["entity", "generation"]);
    expect_one(
        incompatibilities(&baseline(), &now),
        "parameter `generation` became required",
    );
}

#[test]
fn a_changed_parameter_type_is_incompatible() {
    let mut now = baseline();
    tool_mut(&mut now, "get_entity")["properties"]["entity"] = json!({ "type": "string" });
    expect_one(incompatibilities(&baseline(), &now), "no longer accepts type `integer`");
}

#[test]
fn narrowing_accepted_values_is_incompatible() {
    // A higher minimum, a new maximum, an enum where anything was accepted,
    // and a smaller list of enum values each reject input that worked before.
    let mut now = baseline();
    let props = &mut tool_mut(&mut now, "list_entities")["properties"];
    props["limit"]["minimum"] = json!(1);
    props["offset"]["maximum"] = json!(100);
    props["with"]["items"] = json!({ "type": "string", "enum": ["Transform2D"] });
    let found = incompatibilities(&baseline(), &now);
    expect_one(found.clone(), "`limit`: `minimum` tightened from 0 to 1");
    expect_one(found.clone(), "`offset`: new bound `maximum` = 100");
    expect_one(found, "parameter `with`[]: values are now restricted to an enum");

    let mut old = baseline();
    tool_mut(&mut old, "list_entities")["properties"]["with"]["items"] =
        json!({ "type": "string", "enum": ["A", "B"] });
    let mut now = old.clone();
    tool_mut(&mut now, "list_entities")["properties"]["with"]["items"] = json!({ "type": "string", "enum": ["A"] });
    expect_one(incompatibilities(&old, &now), "enum value \"B\" is no longer accepted");
}

#[test]
fn a_read_only_tool_that_starts_writing_is_incompatible() {
    let mut now = baseline();
    tool_mut(&mut now, "get_entity")["read_only"] = json!(false);
    expect_one(
        incompatibilities(&baseline(), &now),
        "was read-only and now changes the game",
    );
}

#[test]
fn additive_changes_are_compatible() {
    let mut now = baseline();
    // A new tool, even with a required parameter of its own.
    now["tools"].as_array_mut().unwrap().push(json!({
        "name": "brand_new_tool",
        "required": ["thing"],
        "properties": { "thing": { "type": "string" } },
        "read_only": true,
    }));
    // A new optional parameter on an existing tool.
    tool_mut(&mut now, "get_entity")["properties"]["verbose"] = json!({ "type": "boolean" });
    // A looser bound, a wider type, and a parameter that stopped being required.
    tool_mut(&mut now, "list_entities")["properties"]["limit"]["minimum"] = json!(-5);
    tool_mut(&mut now, "list_entities")["properties"]["offset"]["type"] = json!("number");
    tool_mut(&mut now, "set_component")["required"] = json!(["entity", "component"]);
    assert_eq!(incompatibilities(&baseline(), &now), Vec::<String>::new());
}

#[test]
fn widening_an_enum_is_compatible() {
    let mut old = baseline();
    tool_mut(&mut old, "list_entities")["properties"]["with"]["items"] = json!({ "type": "string", "enum": ["A"] });
    let mut now = old.clone();
    tool_mut(&mut now, "list_entities")["properties"]["with"]["items"] =
        json!({ "type": "string", "enum": ["A", "B"] });
    assert!(incompatibilities(&old, &now).is_empty());
}

#[test]
fn baseline_v2_records_every_tools_input_schema() {
    let b = baseline_v2();
    for t in b["tools"].as_array().unwrap() {
        assert_eq!(
            t["schema"]["type"], "object",
            "{} lost its input schema record",
            t["name"]
        );
    }
    // v2 holds everything v1 held.
    assert!(incompatibilities(&baseline(), &b).is_empty());
}

#[test]
fn a_tool_that_stops_accepting_extra_arguments_is_incompatible() {
    let mut now = baseline_v2();
    tool_mut(&mut now, "get_entity")["schema"]["additionalProperties"] = json!(false);
    expect_one(
        incompatibilities(&baseline_v2(), &now),
        "tool `get_entity`: input schema: extra properties are no longer accepted",
    );
    let mut now = baseline_v2();
    tool_mut(&mut now, "get_entity")["schema"]["additionalProperties"] = json!({ "type": "string" });
    expect_one(
        incompatibilities(&baseline_v2(), &now),
        "input schema: extra properties are now restricted by a schema",
    );
    let mut now = baseline_v2();
    tool_mut(&mut now, "get_entity")["schema"]["type"] = json!("array");
    expect_one(
        incompatibilities(&baseline_v2(), &now),
        "input schema: no longer accepts type `object`",
    );
}

#[test]
fn a_tool_that_accepts_more_extra_arguments_is_compatible() {
    let mut old = baseline_v2();
    tool_mut(&mut old, "get_entity")["schema"]["additionalProperties"] = json!(false);
    let mut now = baseline_v2();
    tool_mut(&mut now, "get_entity")["schema"]["additionalProperties"] = json!(true);
    assert!(incompatibilities(&old, &now).is_empty(), "false -> true widens");
    assert!(
        incompatibilities(&old, &baseline_v2()).is_empty(),
        "false -> absent widens"
    );
}

/// `launch.env` maps names to strings: an object with `additionalProperties`.
fn with_env_extra(mut contract: Value, extra: Option<Value>) -> Value {
    let env = &mut tool_mut(&mut contract, "launch")["properties"]["env"];
    match extra {
        Some(v) => env["additionalProperties"] = v,
        None => {
            env.as_object_mut().unwrap().remove("additionalProperties");
        }
    }
    contract
}

#[test]
fn widening_additional_properties_is_compatible() {
    let string = Some(json!({ "type": "string" }));
    let old = with_env_extra(baseline(), string.clone());
    for wider in [
        None,
        Some(json!(true)),
        Some(json!({ "type": ["string", "integer"] })),
        string,
    ] {
        let now = with_env_extra(baseline(), wider.clone());
        assert_eq!(incompatibilities(&old, &now), Vec::<String>::new(), "-> {wider:?}");
    }
    // A closed object accepting nothing extra can open up in any way.
    let closed = with_env_extra(baseline(), Some(json!(false)));
    for any in [None, Some(json!(true)), Some(json!({ "type": "integer" }))] {
        let now = with_env_extra(baseline(), any.clone());
        assert_eq!(
            incompatibilities(&closed, &now),
            Vec::<String>::new(),
            "false -> {any:?}"
        );
    }
}

#[test]
fn narrowing_additional_properties_is_incompatible() {
    let old = with_env_extra(baseline(), Some(json!({ "type": "string" })));
    let closed = with_env_extra(baseline(), Some(json!(false)));
    expect_one(
        incompatibilities(&old, &closed),
        "parameter `env`: extra properties are no longer accepted",
    );
    let integers = with_env_extra(baseline(), Some(json!({ "type": "integer" })));
    expect_one(
        incompatibilities(&old, &integers),
        "parameter `env` (extra properties): no longer accepts type `string`",
    );
    let free = with_env_extra(baseline(), None);
    let restricted = with_env_extra(baseline(), Some(json!({ "type": "string" })));
    expect_one(
        incompatibilities(&free, &restricted),
        "parameter `env`: extra properties are now restricted by a schema",
    );
}

// ---------------------------------------------------------------- generator

/// Writes a baseline file from the server at the checked-out revision, to the
/// path in `PONKO_MCP_FIXTURE_OUT`; refuses to overwrite. Only for retaining a
/// new baseline (see tests/fixtures/README.md), never to update an existing one.
fn write_fixture(with_schema: bool) {
    let out = std::env::var("PONKO_MCP_FIXTURE_OUT").expect("set PONKO_MCP_FIXTURE_OUT to the file to write");
    let out = std::path::Path::new(&out);
    assert!(
        !out.exists(),
        "{} exists; fixtures are never overwritten",
        out.display()
    );
    let init = rpc(&mut server(false), "initialize", json!({}));
    let mut contract = current_contract(false);
    if !with_schema {
        // v1 predates the record of each tool's own input schema.
        for t in contract["tools"].as_array_mut().unwrap() {
            t.as_object_mut().unwrap().remove("schema");
        }
    }
    contract["server_name"] = init["serverInfo"]["name"].clone();
    // Every version the server negotiates: ask for each known one.
    let versions: Vec<Value> = ["2025-06-18", "2025-03-26", "2024-11-05"]
        .iter()
        .filter(|v| rpc(&mut server(false), "initialize", json!({ "protocolVersion": v }))["protocolVersion"] == **v)
        .map(|v| json!(v))
        .collect();
    contract["protocol_versions"] = Value::Array(versions);
    std::fs::write(out, serde_json::to_string_pretty(&contract).unwrap() + "\n").unwrap();
}

/// Writes `tools_v1.json` (without the top-level schema record).
#[test]
#[ignore = "writes a fixture file; see crates/ponko_mcp/tests/fixtures/README.md"]
fn write_v1_fixture() {
    write_fixture(false);
}

/// Writes `tools_v2.json`: v1's content plus each tool's other `inputSchema` keywords.
#[test]
#[ignore = "writes a fixture file; see crates/ponko_mcp/tests/fixtures/README.md"]
fn write_v2_fixture() {
    write_fixture(true);
}
