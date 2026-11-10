// End-to-end checks of the runner that need no browser and no build:
// `node --test web/smoke` (needs `npm ci --prefix web/smoke` for playwright-core).
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { after, test } from "node:test";
import { fileURLToPath } from "node:url";
import { summaryMarkdown, summaryRow } from "./lib.mjs";

const SMOKE = fileURLToPath(new URL("./smoke.mjs", import.meta.url));
const SUMMARY = fileURLToPath(new URL("./summary.mjs", import.meta.url));
const work = mkdtempSync(join(tmpdir(), "ponko-smoke-test-"));
after(() => rmSync(work, { recursive: true, force: true }));

/** A site folder that is never served to a browser: the browser can't start. */
const site = join(work, "site");
mkdirSync(site);
writeFileSync(join(site, "index.html"), "<!doctype html>");

function run(script, args, env = {}) {
  return spawnSync(process.execPath, [script, ...args], {
    encoding: "utf8",
    // No browsers installed under this path: every launch fails the same way.
    // CI is cleared: these runs check the not-installed case a laptop sees.
    env: { ...process.env, CI: "", PLAYWRIGHT_BROWSERS_PATH: join(work, "no-browsers"), ...env },
  });
}

for (const browser of ["chromium", "firefox", "webkit"]) {
  test(`${browser} that isn't installed is unsupported with a reason, not a pass`, () => {
    const out = join(work, `out-${browser}`);
    const result = run(SMOKE, ["--dir", site, "--browser", browser, "--backend", "webgpu", "--out", out]);
    assert.equal(result.status, 2, result.stdout + result.stderr);
    const report = JSON.parse(readFileSync(join(out, "report.json"), "utf8"));
    assert.equal(report.outcome.kind, "unsupported");
    assert.match(report.outcome.message, new RegExp(`could not launch headless ${browser}`));
    assert.equal(report.requestedBrowser, browser);
  });
}

test("--allow-unsupported exits 0 but the report still says unsupported", () => {
  const out = join(work, "out-allowed");
  const result = run(SMOKE, ["--dir", site, "--browser", "webkit", "--backend", "webgpu", "--allow-unsupported", "--out", out]);
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stdout, /^UNSUPPORTED \(unsupported\)/m);
  const report = JSON.parse(readFileSync(join(out, "report.json"), "utf8"));
  assert.equal(report.outcome.kind, "unsupported");
  assert.notEqual(report.outcome.message, "");
});

test("a missing build is a usage error", () => {
  const result = run(SMOKE, ["--dir", join(work, "nothing-here")]);
  assert.equal(result.status, 3);
  assert.match(result.stderr, /index\.html not found/);
});

test("the summary table names unsupported runs and failures", () => {
  const rows = [
    summaryRow({ fixture: "web_smoke", browser: "chromium 1", requestedBackend: "webgl2", case: "ok", backend: "webgl2", engine: { summary: { adapter: { backend: "gl", device_type: "cpu", name: "SwiftShader" } } }, outcome: { status: 0, kind: "pass", message: "passed" } }),
    summaryRow({ fixture: "web_smoke", browser: "webkit 2", requestedBackend: "webgpu", outcome: { status: 0, kind: "unsupported", message: "headless webkit has no WebGPU" } }),
    summaryRow({ fixture: "web_smoke", browser: "firefox 3", requestedBackend: "webgl2", outcome: { status: 1, kind: "stall", message: "stalled" } }),
    summaryRow({ fixture: "x" }),
  ];
  assert.deepEqual(rows.map((r) => r.result), ["pass", "UNSUPPORTED", "FAIL", "FAIL"]);
  const table = summaryMarkdown(rows);
  assert.match(table, /gl cpu SwiftShader/);
  assert.match(table, /UNSUPPORTED \| headless webkit has no WebGPU/);
});

test("summary.mjs fails when the folder holds no report", () => {
  const empty = join(work, "empty");
  mkdirSync(empty);
  assert.equal(run(SUMMARY, [empty]).status, 1);
  const out = join(work, "out-chromium");
  const ok = run(SUMMARY, [out]);
  assert.equal(ok.status, 0, ok.stderr);
  assert.match(ok.stdout, /UNSUPPORTED/);
});

test("under CI a browser that isn't installed fails instead of passing as unsupported", () => {
  const out = join(work, "out-ci");
  const result = run(SMOKE, ["--dir", site, "--browser", "webkit", "--backend", "webgpu", "--allow-unsupported", "--out", out], { CI: "true" });
  assert.equal(result.status, 1, result.stdout + result.stderr);
  const report = JSON.parse(readFileSync(join(out, "report.json"), "utf8"));
  assert.equal(report.outcome.kind, "launch");
});
