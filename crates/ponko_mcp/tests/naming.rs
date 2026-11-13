//! Exercise process settings without changing the test process environment.
use std::process::Command;

#[test]
fn registry_configuration_reads_only_the_ponko_variable() {
    for mode in ["default", "primary", "empty", "old-spelling"] {
        let out = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "registry_configuration_child", "--nocapture"])
            .env("PONKO_NAMING_CHILD", mode)
            .env("HOME", "/naming-home")
            .env("LOCALAPPDATA", "/naming-home")
            .env_remove("PONKO_INSTANCES_DIR")
            .output()
            .unwrap();
        assert!(out.status.success(), "{mode}: {}", String::from_utf8_lossy(&out.stderr));
    }
}

#[test]
fn registry_configuration_child() {
    let Ok(mode) = std::env::var("PONKO_NAMING_CHILD") else {
        return;
    };
    // SAFETY: this subprocess contains a single selected test and no other threads.
    unsafe {
        if mode == "old-spelling" {
            // The pre-rename spelling is not read any more.
            std::env::set_var("CRABBY_INSTANCES_DIR", "/old-registry");
        }
        if mode == "primary" {
            std::env::set_var("PONKO_INSTANCES_DIR", "/primary-registry");
        }
        if mode == "empty" {
            std::env::set_var("PONKO_INSTANCES_DIR", "");
        }
    }
    let config = ponko_mcp::Config::from_env();
    let want = match mode.as_str() {
        "default" | "old-spelling" if cfg!(windows) => "/naming-home/ponko/instances",
        "default" | "old-spelling" => "/naming-home/.ponko/instances",
        "primary" => "/primary-registry",
        "empty" => "",
        _ => unreachable!(),
    };
    assert_eq!(config.instances_dir, std::path::PathBuf::from(want));
}

#[test]
fn default_discovery_reads_only_the_ponko_registry() {
    use serde_json::json;
    let root = std::env::temp_dir().join(format!("ponko-discovery-naming-{}", std::process::id()));
    let primary = root
        .join(if cfg!(windows) { "ponko" } else { ".ponko" })
        .join("instances");
    let old = root
        .join(if cfg!(windows) { "crabby" } else { ".crabby" })
        .join("instances");
    std::fs::create_dir_all(&primary).unwrap();
    std::fs::create_dir_all(&old).unwrap();
    let listeners: Vec<_> = (0..2)
        .map(|_| std::net::TcpListener::bind("127.0.0.1:0").unwrap())
        .collect();
    for (dir, listener, name) in [(&primary, &listeners[0], "primary"), (&old, &listeners[1], "old")] {
        let port = listener.local_addr().unwrap().port();
        std::fs::write(
            dir.join(format!("1-{port}.json")),
            json!({
                "pid": 1, "port": port, "name": name, "token": "secret", "started": port
            })
            .to_string(),
        )
        .unwrap();
    }
    for (mode, expected) in [("default", "1"), ("primary", "1"), ("missing-primary", "0")] {
        if mode == "missing-primary" {
            std::fs::remove_dir_all(&primary).unwrap();
        }
        let mut child = Command::new(std::env::current_exe().unwrap());
        child
            .args(["--exact", "registry_discovery_child", "--nocapture"])
            .env("PONKO_DISCOVERY_CHILD", expected)
            .env("HOME", &root)
            .env("LOCALAPPDATA", &root)
            .env_remove("PONKO_INSTANCES_DIR");
        if mode == "primary" {
            child.env("PONKO_INSTANCES_DIR", &primary);
        }
        let out = child.output().unwrap();
        assert!(out.status.success(), "{mode}: {}", String::from_utf8_lossy(&out.stderr));
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn registry_discovery_child() {
    let Ok(expected) = std::env::var("PONKO_DISCOVERY_CHILD") else {
        return;
    };
    let mut server = ponko_mcp::Server::new(ponko_mcp::Config::from_env());
    let response = server
        .handle(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "list_instances", "arguments": {}}
        }))
        .unwrap();
    assert_eq!(response["result"]["isError"], false);
    let instances: serde_json::Value =
        serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    let instances = instances.as_array().unwrap();
    assert_eq!(instances.len(), expected.parse::<usize>().unwrap());
    assert!(instances.iter().all(|i| i["name"] == "primary"));
    assert!(instances.iter().all(|i| i.get("token").is_none()));
}
