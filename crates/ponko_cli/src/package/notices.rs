//! `THIRD-PARTY-NOTICES.txt`: every library linked into the program, with
//! the license it declares and the license texts its source ships.
//!
//! The list is `cargo tree` for the platform the package is for, with the
//! build's features (so it names what that program links, not every
//! optional dependency); the texts are the license files at the top of each
//! crate's source (two folders deep, code folders left out), each text once
//! with the libraries it belongs to. A library that declares no license
//! fails the packaging by name instead of being listed as "unknown".

use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A crate as `cargo metadata` describes it.
#[derive(Clone, Debug, Default)]
pub struct Crate {
    pub name: String,
    pub version: String,
    pub license: Option<String>,
    pub license_file: Option<String>,
    pub authors: Vec<String>,
    /// The folder of its `Cargo.toml`.
    pub dir: PathBuf,
    /// A member of the game's own workspace.
    pub own: bool,
}

impl Crate {
    fn label(&self) -> String {
        format!("{} {}", self.name, self.version)
    }
}

/// The crates of `cargo metadata` output, by (name, version).
pub fn crates(metadata: &Value) -> Result<BTreeMap<(String, String), Crate>, String> {
    let members: Vec<&str> = metadata["workspace_members"]
        .as_array()
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let mut out = BTreeMap::new();
    for p in metadata["packages"]
        .as_array()
        .ok_or("cargo metadata listed no packages")?
    {
        let text = |k: &str| p[k].as_str().map(str::to_string);
        let (Some(name), Some(version), Some(manifest)) = (text("name"), text("version"), text("manifest_path")) else {
            return Err(format!("cargo metadata: a package without name, version or path: {p}"));
        };
        let krate = Crate {
            license: text("license"),
            license_file: text("license_file"),
            authors: p["authors"]
                .as_array()
                .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
                .unwrap_or_default(),
            dir: Path::new(&manifest).parent().unwrap_or(Path::new("")).to_path_buf(),
            own: p["id"].as_str().is_some_and(|id| members.contains(&id)),
            name: name.clone(),
            version: version.clone(),
        };
        out.insert((name, version), krate);
    }
    Ok(out)
}

/// (name, version) of every crate `cargo tree --prefix none -f {p}` printed.
pub fn tree_crates(tree: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = tree
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let name = words.next()?;
            let version = words.next()?.strip_prefix('v')?;
            Some((name.to_string(), version.to_string()))
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The license a crate declares, `None` for the game's own code that
/// declares none, or an error naming the crate.
fn declared_license(krate: &Crate) -> Result<Option<String>, String> {
    if let Some(l) = &krate.license {
        return Ok(Some(l.clone()));
    }
    if let Some(f) = &krate.license_file {
        return Ok(Some(format!("see {f}")));
    }
    if krate.own {
        return Ok(None);
    }
    Err(format!(
        "{} declares no license (no `license` and no `license_file` in its manifest): \
         check its terms before shipping it, or drop the dependency",
        krate.label()
    ))
}

/// File names that are license texts (also inside font folders and the like).
fn is_license_name(name: &str) -> bool {
    const NOT_TEXT: &[&str] = &[
        "rs", "toml", "json", "ttf", "otf", "png", "c", "h", "py", "js", "yml", "yaml", "lock",
    ];
    let lower = name.to_ascii_lowercase();
    let starts = ["license", "licence", "copying", "unlicense", "notice", "ofl", "ufl"]
        .iter()
        .any(|p| lower.starts_with(p));
    let ext = Path::new(&lower).extension().map(|e| e.to_string_lossy().into_owned());
    (starts || lower.contains("license") || lower.contains("licence"))
        && !ext.is_some_and(|e| NOT_TEXT.contains(&e.as_str()))
}

/// Folders of a crate that hold code, not license files.
const CODE_DIRS: &[&str] = &["src", "tests", "examples", "benches", "target", ".git", ".github"];

/// The license files a crate's source ships, as (path in the crate, text).
fn license_texts(krate: &Crate, workspace_root: &Path) -> Vec<(String, String)> {
    let mut found = Vec::new();
    if let Some(f) = &krate.license_file {
        let p = krate.dir.join(f);
        if p.is_file() {
            found.push(p);
        }
    }
    // Files at most two folders below the crate's.
    let mut stack = vec![(krate.dir.clone(), 0)];
    while let Some((dir, depth)) = stack.pop() {
        let Ok(read) = std::fs::read_dir(&dir) else { continue };
        for entry in read.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            match entry.file_type() {
                Ok(t) if t.is_dir() && depth < 2 && !CODE_DIRS.contains(&name.as_str()) => {
                    stack.push((path, depth + 1));
                }
                Ok(t) if t.is_file() && is_license_name(&name) => found.push(path),
                _ => {}
            }
        }
    }
    found.sort();
    found.dedup();
    let mut texts: Vec<(String, String)> = found
        .iter()
        .filter_map(|p| {
            // Universal newlines, so a text with CRLF line ends is the same text.
            let raw = String::from_utf8_lossy(&std::fs::read(p).ok()?).replace("\r\n", "\n");
            let text = raw.replace('\r', "\n").trim().to_string();
            let rel = p.strip_prefix(&krate.dir).unwrap_or(p);
            let rel = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            Some((rel, text))
        })
        .collect();
    if texts.is_empty() && krate.dir.starts_with(workspace_root) {
        // A crate of the game's workspace: it is under the workspace's license files.
        for name in [
            "LICENSE-MIT",
            "LICENSE-APACHE",
            "LICENSE",
            "LICENSE.md",
            "LICENSE.txt",
            "COPYING",
        ] {
            if let Ok(text) = std::fs::read_to_string(workspace_root.join(name)) {
                texts.push((name.to_string(), text.replace("\r\n", "\n").trim().to_string()));
            }
        }
    }
    texts
}

/// The notices; returns (text, libraries listed, libraries without a
/// license file in their source, own crates left out for declaring no license).
pub fn render(
    title: &str,
    triple: &str,
    linked: &[(String, String)],
    crates: &BTreeMap<(String, String), Crate>,
    workspace_root: &Path,
) -> Result<(String, usize, Vec<String>), String> {
    let mut lines = Vec::new();
    let mut without_text = Vec::new();
    // Each text once, with the libraries it belongs to (first-seen order).
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    let mut listed = 0;
    for key in linked {
        let krate = crates
            .get(key)
            .ok_or_else(|| format!("cargo metadata does not know {} {}", key.0, key.1))?;
        let Some(expression) = declared_license(krate)? else {
            continue;
        };
        listed += 1;
        let label = krate.label();
        let authors = if krate.authors.is_empty() {
            "no authors listed".to_string()
        } else {
            krate.authors.join(", ")
        };
        lines.push(format!("  {label}: {expression} ({authors})"));
        let texts = license_texts(krate, workspace_root);
        if texts.is_empty() {
            without_text.push(format!("{label} ({expression})"));
        }
        for (_, text) in texts {
            match groups.iter_mut().find(|g| g.0 == text) {
                Some(g) => g.1.push(label.clone()),
                None => groups.push((text, vec![label.clone()])),
            }
        }
    }
    let heading = format!("Third-party notices for {title}");
    let mut capital = title.chars();
    let title_cap: String = capital
        .next()
        .map(|c| c.to_uppercase().chain(capital).collect())
        .unwrap_or_default();
    let mut out = vec![
        heading.clone(),
        "=".repeat(heading.chars().count()),
        String::new(),
        format!("{title_cap} is linked with the libraries below, built for {triple}."),
        "The first part lists each library with the license it declares; the".into(),
        "second part gives the license texts those libraries ship, each text once".into(),
        "with the libraries it belongs to.".into(),
        String::new(),
        format!("Libraries ({listed})"),
        "------------------".into(),
    ];
    out.extend(lines);
    out.push(String::new());
    if !without_text.is_empty() {
        out.extend([
            "These libraries ship no license file in their published source. Their".to_string(),
            "declared license and authors are listed above; the standard texts of".into(),
            "MIT, Apache-2.0 and the other SPDX licenses are at https://spdx.org/licenses/.".into(),
        ]);
        out.extend(without_text.iter().map(|w| format!("  {w}")));
        out.push(String::new());
    }
    out.extend(["License texts".to_string(), "-------------".into(), String::new()]);
    groups.sort_by(|a, b| a.1[0].cmp(&b.1[0]));
    for (text, labels) in groups {
        out.extend([
            "=".repeat(72),
            format!("Used by: {}", labels.join(", ")),
            "=".repeat(72),
            text,
            String::new(),
        ]);
    }
    Ok((out.join("\n"), listed, without_text))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn krate(name: &str, license: Option<&str>, dir: &Path, own: bool) -> Crate {
        Crate {
            name: name.into(),
            version: "1.0.0".into(),
            license: license.map(str::to_string),
            dir: dir.to_path_buf(),
            own,
            ..Crate::default()
        }
    }

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ponko-notices-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn tree_lines_become_sorted_unique_crates() {
        let tree = "game v0.1.0 (/x)\nserde v1.0.2\nglam v0.30.0 (*)\nserde v1.0.2\n\n[build-dependencies]\n";
        assert_eq!(
            tree_crates(tree),
            [
                ("game".into(), "0.1.0".into()),
                ("glam".into(), "0.30.0".into()),
                ("serde".into(), "1.0.2".into())
            ]
        );
    }

    #[test]
    fn a_library_that_declares_no_license_fails_by_name() {
        let dir = temp("nolicense");
        let crates = BTreeMap::from([(
            ("nolicense".into(), "1.0.0".into()),
            krate("nolicense", None, &dir, false),
        )]);
        let err = render(
            "G",
            "t",
            &[("nolicense".into(), "1.0.0".into())],
            &crates,
            Path::new("/ws"),
        )
        .unwrap_err();
        assert!(err.contains("nolicense 1.0.0 declares no license"), "{err}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_games_own_crates_without_a_license_are_left_out() {
        let dir = temp("own");
        let crates = BTreeMap::from([(("mine".into(), "1.0.0".into()), krate("mine", None, &dir, true))]);
        let (text, listed, _) =
            render("G", "t", &[("mine".into(), "1.0.0".into())], &crates, Path::new("/ws")).unwrap();
        assert_eq!(listed, 0);
        assert!(!text.contains("mine 1.0.0"), "{text}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn license_texts_are_found_and_printed_once_with_their_libraries() {
        let root = temp("texts");
        for (name, files) in [
            ("a", &[("LICENSE-MIT", "MIT text"), ("src/LICENSE", "not this one")][..]),
            (
                "b",
                &[
                    ("LICENSE.txt", "MIT text"),
                    ("fonts/OFL.txt", "font text"),
                    ("license.json", "no"),
                ][..],
            ),
            ("c", &[][..]),
        ] {
            for (file, text) in files {
                let p = root.join(name).join(file);
                std::fs::create_dir_all(p.parent().unwrap()).unwrap();
                std::fs::write(p, text).unwrap();
            }
            std::fs::create_dir_all(root.join(name)).unwrap();
        }
        let linked: Vec<(String, String)> = ["a", "b", "c"]
            .iter()
            .map(|n| (n.to_string(), "1.0.0".to_string()))
            .collect();
        let crates = linked
            .iter()
            .map(|k| (k.clone(), krate(&k.0, Some("MIT"), &root.join(&k.0), false)))
            .collect();
        let (text, listed, without) = render(
            "my game",
            "x86_64-unknown-linux-gnu",
            &linked,
            &crates,
            Path::new("/elsewhere"),
        )
        .unwrap();
        assert_eq!(listed, 3);
        assert_eq!(without, ["c 1.0.0 (MIT)"]);
        assert!(text.starts_with("Third-party notices for my game\n"));
        assert!(text.contains("My game is linked with the libraries below, built for x86_64-unknown-linux-gnu."));
        assert!(text.contains("  a 1.0.0: MIT (no authors listed)"));
        assert_eq!(text.matches("MIT text").count(), 1, "{text}");
        assert!(text.contains("Used by: a 1.0.0, b 1.0.0\n"));
        assert!(text.contains("font text") && !text.contains("not this one") && !text.contains("\nno\n"));
        let _ = std::fs::remove_dir_all(root);
    }
}
