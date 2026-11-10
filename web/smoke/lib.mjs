// The parts of the browser smoke runner that need no browser: arguments,
// fixture configuration, and the decisions made from what the browser and the
// engine report. smoke.mjs drives the browser; lib.test.mjs tests this file
// (`node --test web/smoke`).

import { join } from "node:path";

export const EXIT = { pass: 0, app: 1, unsupported: 2, usage: 3 };
export const BACKENDS = ["auto", "webgpu", "webgl2"];
export const BROWSERS = ["chromium", "firefox", "webkit"];
// Self-tests: each makes the fixture fail in one way that must be caught.
export const CASES = {
  ok: null,
  "missing-asset": "asset",
  panic: "panic",
  stall: "stall",
};
// Ticks the simulation must advance in the 3 s progress window. A coarse
// bound, not a frame-rate assertion: 60 Hz for 3 s gives ~180 ticks, and it
// must stay above STALL_AFTER in examples/common/web_smoke.rs, since a `stall`
// run seen early (at tick 1) still advances that far.
export const MIN_PROGRESS_TICKS = 60;

export class UsageError extends Error {}

export const USAGE =
  "usage: smoke.mjs (--dir <built site> | --fixture <name> [--build]) [--browser chromium|firefox|webkit]\n" +
  "                 [--backend auto|webgpu|webgl2] [--case ok|missing-asset|panic|stall]\n" +
  "                 [--out <artifacts dir>] [--timeout <startup seconds>] [--allow-unsupported]\n" +
  "                 [--asset <file under assets/>] [--asset-size <W>x<H>|none]\n" +
  "                 [--press <Key>=<field>:<value>]... [--min <field>=<number>]...\n" +
  "                 [--engine-gpu required|optional] [--no-trace] [--chrome-trace]\n" +
  "                 [--chromium-mode shell|new] [--browser-arg <flag>]...";

/** `Enter=screen:Playing`: press the key, then the state's `screen` must become "Playing". */
export function parsePress(spec) {
  const match = /^([A-Za-z0-9]+)=([A-Za-z_]+):(.+)$/.exec(spec);
  if (!match) throw new UsageError(`--press wants <Key>=<field>:<value>, not "${spec}"`);
  return { key: match[1], field: match[2], value: match[3] };
}

/** `triangles=100`: the state's numeric `triangles` must be at least 100. */
export function parseMin(spec) {
  const match = /^([A-Za-z_]+)=(\d+(?:\.\d+)?)$/.exec(spec);
  if (!match) throw new UsageError(`--min wants <field>=<number>, not "${spec}"`);
  return { field: match[1], min: Number(match[2]) };
}

const FIXTURE_KEYS = ["example", "features", "asset", "assetSize", "assetSource", "presses", "min", "engineGpu"];

/** Parses fixtures.json: what each example needs from the runner. */
export function parseFixtures(text) {
  const fixtures = JSON.parse(text);
  for (const [name, fixture] of Object.entries(fixtures)) {
    for (const key of Object.keys(fixture)) {
      if (!FIXTURE_KEYS.includes(key)) throw new UsageError(`fixtures.json: ${name} has an unknown field "${key}"`);
    }
    if (typeof fixture.example !== "string") throw new UsageError(`fixtures.json: ${name} needs an "example"`);
    if (fixture.presses) fixture.presses.forEach(parsePress);
  }
  return fixtures;
}

export function parseArgs(argv, fixtures = {}) {
  const args = {
    browser: "chromium",
    backend: "auto",
    case: "ok",
    timeout: 90,
    allowUnsupported: false,
    browserArgs: [],
    chromiumMode: "shell",
    trace: true,
    chromeTrace: false,
    build: false,
    presses: [],
    mins: [],
  };
  const given = {};
  for (let i = 0; i < argv.length; i++) {
    const flag = argv[i];
    const value = () => argv[++i] ?? fail(`${flag} needs a value`);
    switch (flag) {
      case "--dir": args.dir = value(); break;
      case "--fixture": args.fixture = value(); break;
      case "--build": args.build = true; break;
      case "--browser": args.browser = value(); break;
      case "--backend": args.backend = value(); break;
      case "--case": args.case = value(); break;
      case "--out": args.out = value(); break;
      case "--timeout": args.timeout = Number(value()); break;
      case "--allow-unsupported": args.allowUnsupported = true; break;
      case "--browser-arg": args.browserArgs.push(value()); break;
      case "--chromium-mode": args.chromiumMode = value(); break;
      case "--no-trace": args.trace = false; break;
      case "--chrome-trace": args.chromeTrace = true; break;
      case "--asset": given.asset = value(); break;
      case "--asset-size": given.assetSize = value(); break;
      case "--asset-source": given.assetSource = value(); break;
      case "--engine-gpu": given.engineGpu = value(); break;
      case "--press": (given.presses ??= []).push(parsePress(value())); break;
      case "--min": (given.mins ??= []).push(parseMin(value())); break;
      default: fail(`unknown argument ${flag}`);
    }
  }
  // A fixture supplies defaults; flags on the command line win.
  const fixture = args.fixture ? fixtures[args.fixture] : undefined;
  if (args.fixture && !fixture) {
    fail(`--fixture must be one of ${Object.keys(fixtures).join(", ")}`);
  }
  args.example = fixture?.example;
  args.features = fixture?.features ?? [];
  args.asset = given.asset ?? fixture?.asset ?? "crab.png";
  args.assetSize = given.assetSize ?? (fixture ? fixture.assetSize ?? "none" : "32x32");
  args.assetSource = given.assetSource ?? (fixture ? fixture.assetSource ?? "none" : "4x2");
  args.presses = given.presses ?? (fixture?.presses ?? []).map(parsePress);
  args.mins = given.mins ?? Object.entries(fixture?.min ?? {}).map(([field, min]) => ({ field, min }));
  args.engineGpu = given.engineGpu ?? (fixture?.engineGpu ? "required" : "optional");
  args.dir ??= args.example ? join("target", "web", args.example) : undefined;

  if (!args.dir) fail("--dir or --fixture is required");
  if (args.build && !args.fixture) fail("--build needs --fixture (which example to build, and how)");
  if (!BACKENDS.includes(args.backend)) fail(`--backend must be one of ${BACKENDS.join(", ")}`);
  if (!BROWSERS.includes(args.browser)) fail(`--browser must be one of ${BROWSERS.join(", ")}`);
  if (!["shell", "new"].includes(args.chromiumMode)) fail("--chromium-mode must be shell or new");
  if (!(args.case in CASES)) fail(`--case must be one of ${Object.keys(CASES).join(", ")}`);
  if (!(args.timeout > 0)) fail("--timeout must be a positive number of seconds");
  if (args.assetSize !== "none" && !/^\d+x\d+$/.test(args.assetSize)) fail("--asset-size must look like 32x32, or none");
  if (args.assetSource !== "none" && !/^\d+x\d+$/.test(args.assetSource)) fail("--asset-source must look like 4x2, or none");
  if (!/^[\w./-]+$/.test(args.asset)) fail("--asset must be a plain file name under assets/");
  if (!["required", "optional"].includes(args.engineGpu)) fail("--engine-gpu must be required or optional");
  const browserLabel = args.browser === "chromium" && args.chromiumMode === "new" ? "chromium-new" : args.browser;
  args.out ??= join("target", "web-smoke", [args.fixture ?? "site", browserLabel, args.backend, args.case].join("-"));
  return args;
}

function fail(message) {
  throw new UsageError(message);
}

/** The Playwright launch options for a browser: software rendering where there is no GPU, WebGPU switched on. */
export function launchConfig(browser, platform, args = {}) {
  const extra = args.browserArgs ?? [];
  if (browser === "chromium") {
    // Software fallbacks so a GPU-less CI runner can still draw; Linux
    // needs Vulkan turned on for WebGPU.
    const flags = ["--enable-unsafe-swiftshader", "--enable-unsafe-webgpu"];
    if (platform === "linux") flags.push("--enable-features=Vulkan");
    // The full browser in its new headless mode, not the headless shell:
    // it has the compositor a WebGPU canvas is presented through.
    const full = args.chromiumMode === "new";
    return { options: { headless: true, args: [...flags, ...extra], ...(full ? { channel: "chromium" } : {}) } };
  }
  if (browser === "firefox") {
    return {
      options: {
        headless: true,
        args: extra,
        // WebGPU is behind a pref where it ships at all; WebGL2 falls back
        // to software where the machine has no GPU.
        firefoxUserPrefs: {
          "dom.webgpu.enabled": true,
          "gfx.webgpu.ignore-blocklist": true,
          "webgl.force-enabled": true,
          "webgl.enable-webgl2": true,
        },
      },
    };
  }
  return { options: { headless: true, args: extra } };
}

/** What the engine's backend name (adapter.backend, from Ctx::gpu_capabilities) means for the page's canvas. */
export function canvasBackendOf(engineBackend) {
  if (engineBackend === "gl") return "webgl2";
  if (engineBackend === "webgpu") return "webgpu";
  return null;
}

/**
 * Compares what the engine says it opened with the context the canvas got.
 * Returns an error message, or null when they agree. With `required` the
 * engine's report must exist: a fixture that doesn't publish one is a failure
 * rather than a silent skip.
 */
export function checkEngineGpu(gpu, canvasBackend, required) {
  if (!gpu) {
    return required ? "the engine published no GPU report (data-smoke-gpu): Ctx::gpu_capabilities was never available" : null;
  }
  const adapter = gpu.adapter ?? {};
  const engine = canvasBackendOf(adapter.backend);
  if (!engine) return `the engine reports backend "${adapter.backend}", which is not a browser backend`;
  if (canvasBackend !== "unknown" && engine !== canvasBackend) {
    return `the engine says ${adapter.backend} (${engine}) but the canvas has a ${canvasBackend} context`;
  }
  return null;
}

/** An error message when the state's numeric fields miss a `--min`, else null. */
export function checkMinimums(state, mins) {
  for (const { field, min } of mins) {
    const got = state[field];
    if (typeof got !== "number" || !(got >= min)) {
      return `${field} is ${JSON.stringify(got)}, below the required ${min}`;
    }
  }
  return null;
}

/**
 * Why this browser can't run the requested backend, or null when it can.
 * `caps` is what the page probe found, without the engine: a backend
 * counts as available only if a plain canvas of that kind really draws.
 */
export function unsupportedReason(caps, backend, browser = "chromium") {
  const name = browser === "chromium" ? "this browser" : `headless ${browser}`;
  if (backend === "webgpu") {
    if (caps.webgpu === null && caps.webgpuError) return `${name} threw while asking for a WebGPU adapter: ${caps.webgpuError}`;
    if (!caps.webgpu) {
      return caps.hasNavigatorGpu === false
        ? `${name} has no WebGPU (navigator.gpu is missing)`
        : `${name} has no WebGPU adapter here`;
    }
    if (!caps.webgpu.canvas) return `${name} can't show a WebGPU canvas here: ${caps.webgpu.canvasProblem}`;
    return null;
  }
  if (backend === "webgl2" && !caps.webgl2) return `${name} has no WebGL2 here`;
  if (backend === "auto" && !caps.webgl2 && !caps.webgpu) return `${name} has neither WebGPU nor WebGL2 here`;
  return null;
}

/** Whether the probe found what `backend` needs (used to decide on asking again). */
export function backendAvailable(caps, backend) {
  return unsupportedReason(caps, backend) === null;
}

/**
 * One line per run of a folder of reports: what was asked for, what the page
 * and the engine reported, and how it ended. "Unsupported" runs say why, so a
 * WebGPU step that couldn't run can never read as a pass.
 */
export function summaryRow(report) {
  const outcome = report.outcome ?? { status: -1, kind: "no outcome", message: "the run wrote no outcome" };
  const adapter = report.engine?.summary?.adapter;
  return {
    fixture: report.fixture ?? "-",
    browser: report.browser ?? report.requestedBrowser ?? "-",
    requested: report.requestedBackend ?? "-",
    case: report.case ?? "ok",
    used: report.backend ?? "-",
    adapter: adapter ? `${adapter.backend} ${adapter.device_type} ${adapter.name}`.trim() : "-",
    result: outcome.kind === "unsupported" ? "UNSUPPORTED" : outcome.status === 0 ? "pass" : "FAIL",
    kind: outcome.kind,
    message: String(outcome.message ?? "").replace(/\s*\n\s*/g, " ").slice(0, 200),
  };
}

export function summaryMarkdown(rows) {
  const cell = (text) => String(text).replace(/\|/g, "\\|");
  const lines = [
    "| Fixture | Browser | Asked | Ran on | Adapter (from the engine) | Result | Detail |",
    "|---|---|---|---|---|---|---|",
    ...rows.map((r) =>
      `| ${[r.fixture + (r.case === "ok" ? "" : ` (${r.case})`), r.browser, r.requested, r.used, r.adapter, r.result, r.message].map(cell).join(" | ")} |`,
    ),
  ];
  return lines.join("\n") + "\n";
}

/**
 * What a browser that won't launch means. One that isn't installed is
 * "unsupported" on a developer's machine, where nobody installed it. Under CI
 * the job installs every browser it runs, so a missing one is a broken install
 * and a failure: otherwise every step for it would pass as unsupported.
 */
export function launchFailureKind(message, env = {}) {
  const notInstalled = /Executable doesn't exist/.test(message);
  const ci = env.CI !== undefined && env.CI !== "" && env.CI !== "false" && env.CI !== "0";
  return notInstalled && !ci ? "unsupported" : "launch";
}
