#!/usr/bin/env node
// Builds the web gallery: every example in examples.json, built for the web
// with web/build.sh, a page per example and an index page, as one static
// site (see README.md).
//
//   node web/gallery/build.mjs                      # -> target/web/gallery/
//   node web/gallery/build.mjs --ref v0.2.0         # source links at the tag
//   node web/gallery/build.mjs --only battle,3d     # some examples
//   node web/gallery/build.mjs --pages-only         # rewrite the pages, no cargo
//   python3 -m http.server -d target/web/gallery 8000
//
// Needs what web/build.sh needs (the wasm32-unknown-unknown target and
// wasm-bindgen-cli at the version in Cargo.lock).

import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { REPO, parseManifest, renderIndex, renderPage, requiredFeatures, versionOf } from "./lib.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, "..", "..");

const USAGE = "usage: build.mjs [--out <dir>] [--ref <tag or branch>] [--only <example,...>] [--pages-only]";

function parseArgs(argv) {
  const args = { out: null, ref: "main", only: null, pagesOnly: false };
  for (let i = 0; i < argv.length; i++) {
    const value = () => argv[++i] ?? die(`${argv[i - 1]} needs a value`);
    switch (argv[i]) {
      case "--out": args.out = value(); break;
      case "--ref": args.ref = value(); break;
      case "--only": args.only = value().split(",").filter(Boolean); break;
      case "--pages-only": args.pagesOnly = true; break;
      case "-h": case "--help": console.log(USAGE); process.exit(0); break;
      default: die(`unknown argument ${argv[i]}`);
    }
  }
  return args;
}

function die(message) {
  console.error(`gallery: ${message}\n${USAGE}`);
  process.exit(2);
}

/** examples/<name>.rs or the folder examples/<name>/ (with main.rs). */
function sourceOf(example) {
  if (existsSync(join(ROOT, "examples", `${example}.rs`))) return `examples/${example}.rs`;
  if (existsSync(join(ROOT, "examples", example, "main.rs"))) return `examples/${example}/`;
  return null;
}

function targetDir() {
  const meta = spawnSync("cargo", ["metadata", "--format-version", "1", "--no-deps"], { cwd: ROOT, encoding: "utf8" });
  if (meta.status !== 0) die(`cargo metadata failed: ${meta.stderr}`);
  return JSON.parse(meta.stdout).target_directory;
}

const args = parseArgs(process.argv.slice(2));
let entries = parseManifest(readFileSync(join(HERE, "examples.json"), "utf8"));
const required = requiredFeatures(readFileSync(join(ROOT, "Cargo.toml"), "utf8"));
for (const entry of entries) {
  entry.source = sourceOf(entry.example) ?? die(`examples.json names ${entry.example}, but examples/ has no such example`);
  const missing = (required[entry.example] ?? []).filter((f) => !entry.features.includes(f));
  if (missing.length) die(`${entry.example} needs the features ${missing.join(", ")} (Cargo.toml); add them to its "features"`);
}
if (args.only) {
  const unknown = args.only.filter((name) => !entries.some((e) => e.example === name));
  if (unknown.length) die(`--only: not in examples.json: ${unknown.join(", ")}`);
  entries = entries.filter((e) => args.only.includes(e.example));
}

const out = resolve(args.out ?? join(targetDir(), "web", "gallery"));
const site = { ref: args.ref, version: versionOf(args.ref), repo: REPO };
mkdirSync(out, { recursive: true });

const started = Date.now();
for (const entry of entries) {
  const dir = join(out, entry.example);
  if (!args.pagesOnly) {
    const buildArgs = [join(ROOT, "web", "build.sh")];
    if (entry.features.length) buildArgs.push("--features", entry.features.join(","));
    buildArgs.push(entry.example, dir);
    console.log(`gallery: web/build.sh ${buildArgs.slice(1).join(" ")}`);
    // From the repository root: build.sh copies ./assets next to the game.
    const built = spawnSync("bash", buildArgs, { cwd: ROOT, stdio: "inherit" });
    if (built.status !== 0) die(`building ${entry.example} failed`);
  } else if (!existsSync(join(dir, "game.js"))) {
    console.warn(`gallery: ${entry.example} has no build in ${dir} (--pages-only writes its page anyway)`);
  }
  mkdirSync(dir, { recursive: true });
  // The gallery page replaces web/index.html's bare page.
  writeFileSync(join(dir, "index.html"), renderPage(entry, site));
}

writeFileSync(join(out, "index.html"), renderIndex(entries, site));
copyFileSync(join(HERE, "gallery.css"), join(out, "gallery.css"));
copyFileSync(join(HERE, "play.js"), join(out, "play.js"));
// What was built, for web/smoke/gallery.mjs.
writeFileSync(join(out, "gallery.json"), JSON.stringify({ ref: site.ref, version: site.version, examples: entries }, null, 2) + "\n");
// GitHub Pages: serve the files as they are (no Jekyll).
writeFileSync(join(out, ".nojekyll"), "");
console.log(`gallery: ${entries.length} example(s) in ${out} after ${((Date.now() - started) / 1000).toFixed(0)}s; try it: python3 -m http.server -d ${out} 8000`);
