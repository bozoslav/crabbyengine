#!/usr/bin/env node
// Browser smoke test for a Ponko web build (see README.md).
//
//   node web/smoke/smoke.mjs --dir target/web/web_smoke --backend webgl2
//
// Serves the built folder over HTTP, opens it in headless Chromium and checks
// that the game starts, fetches its assets, keeps simulating, reacts to a
// click and draws a nonblank canvas. Exit codes:
//
//   0  passed (for a --case self-test: the expected failure was detected)
//   1  the game failed (a panic, a failed asset fetch, a stall, ...)
//   2  unsupported: the browser could not start, or lacks the backend asked for
//   3  bad arguments or a missing build
//
// Every run writes report.json (browser, backend, adapter, timings, console)
// and screenshots to --out.

import { createServer } from "node:http";
import { mkdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import { extname, join, normalize, resolve, sep } from "node:path";
import { chromium } from "playwright-core";

// The canvas web/index.html gives the engine (its CANVAS_ID in src/web.rs).
const GAME_CANVAS = "canvas#crabby";

const EXIT = { pass: 0, app: 1, unsupported: 2, usage: 3 };
const BACKENDS = ["auto", "webgpu", "webgl2"];
// Self-tests: each makes the fixture fail in one way that must be caught.
const CASES = {
  ok: null,
  "missing-asset": "asset",
  panic: "panic",
  stall: "stall",
};
// Ticks the simulation must advance in the 3 s progress window (step 3).
const MIN_PROGRESS_TICKS = 60;

function usage(message) {
  console.error(`smoke: ${message}`);
  console.error(
    "usage: smoke.mjs --dir <built site> [--backend auto|webgpu|webgl2] [--case ok|missing-asset|panic|stall]\n" +
      "                 [--out <artifacts dir>] [--timeout <startup seconds>] [--allow-unsupported]\n" +
      "                 [--browser-arg <flag>]...",
  );
  process.exit(EXIT.usage);
}

function parseArgs(argv) {
  const args = { backend: "auto", case: "ok", timeout: 90, allowUnsupported: false, browserArgs: [] };
  for (let i = 0; i < argv.length; i++) {
    const flag = argv[i];
    const value = () => argv[++i] ?? usage(`${flag} needs a value`);
    switch (flag) {
      case "--dir": args.dir = value(); break;
      case "--backend": args.backend = value(); break;
      case "--case": args.case = value(); break;
      case "--out": args.out = value(); break;
      case "--timeout": args.timeout = Number(value()); break;
      case "--allow-unsupported": args.allowUnsupported = true; break;
      case "--browser-arg": args.browserArgs.push(value()); break;
      default: usage(`unknown argument ${flag}`);
    }
  }
  if (!args.dir) usage("--dir is required");
  if (!BACKENDS.includes(args.backend)) usage(`--backend must be one of ${BACKENDS.join(", ")}`);
  if (!(args.case in CASES)) usage(`--case must be one of ${Object.keys(CASES).join(", ")}`);
  if (!(args.timeout > 0)) usage("--timeout must be a positive number of seconds");
  args.out ??= join("target", "web-smoke", `${args.backend}-${args.case}`);
  return args;
}

const MIME = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript",
  ".mjs": "text/javascript",
  ".wasm": "application/wasm",
  ".json": "application/json",
  ".png": "image/png",
  ".wgsl": "text/plain",
};

/** An empty page on the same (secure) origin: WebGPU isn't exposed on about:blank. */
const PROBE_PATH = "/__smoke_probe.html";

/** A static file server for `root` on a free localhost port. */
async function serve(root) {
  const server = createServer(async (req, res) => {
    const path = decodeURIComponent(new URL(req.url, "http://x").pathname);
    if (path === PROBE_PATH) {
      res.writeHead(200, { "content-type": MIME[".html"] }).end("<!doctype html><title>probe</title>");
      return;
    }
    const file = normalize(join(root, path.endsWith("/") ? path + "index.html" : path));
    if (file !== root && !file.startsWith(root + sep)) {
      res.writeHead(403).end();
      return;
    }
    try {
      const body = await readFile(file);
      res.writeHead(200, { "content-type": MIME[extname(file)] ?? "application/octet-stream" }).end(body);
    } catch {
      res.writeHead(404).end("not found");
    }
  });
  await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
  return server;
}

const sleep = (ms) => new Promise((ok) => setTimeout(ok, ms));

/** What the browser offers, probed on a blank page before the game loads. */
async function probe(context, url) {
  const page = await context.newPage();
  try {
    await page.goto(new URL(PROBE_PATH, url).href);
    return await page.evaluate(async () => {
      const result = { webgpu: null, webgl2: null };
      if (navigator.gpu) {
        try {
          const adapter = await navigator.gpu.requestAdapter();
          if (adapter) {
            const info = adapter.info ?? {};
            result.webgpu = {
              vendor: info.vendor ?? "",
              architecture: info.architecture ?? "",
              description: info.description ?? "",
              fallback: Boolean(adapter.isFallbackAdapter ?? info.isFallbackAdapter),
            };
            // Can this browser show a WebGPU canvas at all? Clear one to
            // red without the engine and copy it back. (Some headless
            // setups have an adapter but no canvas presentation.)
            const device = await adapter.requestDevice();
            const errors = [];
            device.addEventListener("uncapturederror", (e) => errors.push(e.error.message));
            const canvas = document.createElement("canvas");
            canvas.width = canvas.height = 16;
            document.body.append(canvas);
            const context = canvas.getContext("webgpu");
            context.configure({ device, format: navigator.gpu.getPreferredCanvasFormat(), alphaMode: "opaque" });
            const encoder = device.createCommandEncoder();
            encoder
              .beginRenderPass({
                colorAttachments: [{ view: context.getCurrentTexture().createView(), loadOp: "clear", storeOp: "store", clearValue: [1, 0, 0, 1] }],
              })
              .end();
            device.queue.submit([encoder.finish()]);
            await device.queue.onSubmittedWorkDone();
            await new Promise((ok) => requestAnimationFrame(() => setTimeout(ok, 0)));
            const copy = document.createElement("canvas").getContext("2d", { willReadFrequently: true });
            copy.canvas.width = copy.canvas.height = 16;
            copy.drawImage(canvas, 0, 0);
            const [r, g, b] = copy.getImageData(8, 8, 1, 1).data;
            result.webgpu.canvas = r > 200 && g < 50 && b < 50 && errors.length === 0;
            if (!result.webgpu.canvas) result.webgpu.canvasProblem = errors[0] ?? `cleared red, read back rgb(${r}, ${g}, ${b})`;
            device.destroy();
          }
        } catch (e) {
          result.webgpuError = String(e);
          if (result.webgpu) {
            result.webgpu.canvas = false;
            result.webgpu.canvasProblem = `probing a WebGPU canvas threw: ${e}`;
          }
        }
      }
      const gl = document.createElement("canvas").getContext("webgl2");
      if (gl) {
        const debug = gl.getExtension("WEBGL_debug_renderer_info");
        result.webgl2 = {
          vendor: gl.getParameter(debug ? debug.UNMASKED_VENDOR_WEBGL : gl.VENDOR),
          renderer: gl.getParameter(debug ? debug.UNMASKED_RENDERER_WEBGL : gl.RENDERER),
        };
      }
      return result;
    });
  } finally {
    await page.close();
  }
}

/**
 * Counts colors and pixels unlike the most common one, inside the browser:
 * of a PNG screenshot, or (without one) of the game canvas copied with
 * drawImage, which also sees WebGPU canvases that headless screenshots miss.
 */
async function canvasStats(page, png) {
  return page.evaluate(async ([base64, selector]) => {
    let image;
    if (base64) {
      image = new Image();
      image.src = `data:image/png;base64,${base64}`;
      await image.decode();
    } else {
      image = document.querySelector(selector);
      // Copy right after the game's next frame, while it's current.
      await new Promise((ok) => requestAnimationFrame(() => setTimeout(ok, 0)));
    }
    const canvas = document.createElement("canvas");
    canvas.width = image.width;
    canvas.height = image.height;
    const ctx = canvas.getContext("2d");
    ctx.drawImage(image, 0, 0);
    const { data } = ctx.getImageData(0, 0, image.width, image.height);
    const counts = new Map();
    for (let i = 0; i < data.length; i += 4) {
      const key = (data[i] << 16) | (data[i + 1] << 8) | data[i + 2];
      counts.set(key, (counts.get(key) ?? 0) + 1);
    }
    const pixels = data.length / 4;
    const top = Math.max(...counts.values());
    return { width: image.width, height: image.height, colors: counts.size, foreground: (pixels - top) / pixels };
  }, [png ? png.toString("base64") : null, GAME_CANVAS]);
}

/** The fixture's `<html data-smoke>` state, or null before its first frame. */
async function smokeState(page) {
  return page.evaluate(() => {
    const raw = document.documentElement.getAttribute("data-smoke");
    const status = document.getElementById("status");
    return {
      state: raw ? JSON.parse(raw) : null,
      status: status && status.className === "error" ? status.textContent : null,
    };
  });
}

/** Which context the game's canvas got: a second getContext of another kind returns null. */
async function canvasBackend(page) {
  return page.evaluate((selector) => {
    const canvas = document.querySelector(selector);
    if (!canvas) return "unknown";
    if (canvas.getContext("webgl2")) return "webgl2";
    if (navigator.gpu && canvas.getContext("webgpu")) return "webgpu";
    return "unknown";
  }, GAME_CANVAS);
}

class Failure extends Error {
  constructor(kind, message) {
    super(message);
    this.kind = kind;
  }
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const root = resolve(args.dir);
  try {
    await stat(join(root, "index.html"));
  } catch {
    usage(`${root}/index.html not found: build it first, e.g. web/build.sh web_smoke`);
  }
  await mkdir(args.out, { recursive: true });
  // Don't leave an earlier run's artifacts beside this one's.
  for (const name of ["report.json", "console.log", "canvas.png", "page.png"]) {
    await rm(join(args.out, name), { force: true });
  }

  const started = Date.now();
  const elapsed = () => (Date.now() - started) / 1000;
  const report = {
    case: args.case,
    requestedBackend: args.backend,
    platform: `${process.platform}-${process.arch}`,
    startupTimeoutSeconds: args.timeout,
    console: [],
    pageErrors: [],
    requests: [],
  };
  const log = (line) => console.log(`[${elapsed().toFixed(1)}s] ${line}`);
  let outcome;

  const finish = async (status, kind, message) => {
    report.outcome = { status, kind, message };
    report.elapsedSeconds = elapsed();
    await writeFile(join(args.out, "report.json"), JSON.stringify(report, null, 2) + "\n");
    await writeFile(
      join(args.out, "console.log"),
      report.console.map((m) => `[${m.type}] ${m.text}`).join("\n") + "\n",
    );
    outcome = status;
  };

  const server = await serve(root);
  const url = `http://127.0.0.1:${server.address().port}/`;
  let browser;
  try {
    // Software fallbacks so a GPU-less CI runner can still draw; Linux
    // needs Vulkan turned on for WebGPU.
    const browserArgs = ["--enable-unsafe-swiftshader", "--enable-unsafe-webgpu", ...args.browserArgs];
    if (process.platform === "linux") browserArgs.push("--enable-features=Vulkan");
    report.browserArgs = browserArgs;
    try {
      browser = await chromium.launch({ headless: true, args: browserArgs });
    } catch (e) {
      throw new Failure("unsupported", `could not launch headless Chromium: ${e.message.split("\n")[0]}`);
    }
    report.browser = `chromium ${browser.version()}`;
    log(`${report.browser}, ${report.platform}`);

    // Software GL contexts occasionally fail to start; ask again before
    // calling a backend unsupported.
    const wanted = (c) =>
      args.backend === "webgl2" ? c.webgl2 : args.backend === "webgpu" ? c.webgpu?.canvas : c.webgl2 || c.webgpu;
    for (let attempt = 1; ; attempt++) {
      const probeContext = await browser.newContext();
      report.capabilities = await probe(probeContext, url);
      await probeContext.close();
      report.probeAttempts = attempt;
      if (wanted(report.capabilities) || attempt === 3) break;
      log(`probe ${attempt} lacked the backend: ${JSON.stringify(report.capabilities)}; retrying`);
      await sleep(1000);
    }
    log(`WebGPU adapter: ${JSON.stringify(report.capabilities.webgpu)}; WebGL2: ${JSON.stringify(report.capabilities.webgl2)}`);
    if (args.backend === "webgpu" && !report.capabilities.webgpu) {
      throw new Failure("unsupported", "this browser has no WebGPU adapter here");
    }
    if (args.backend === "webgpu" && !report.capabilities.webgpu.canvas) {
      throw new Failure("unsupported", `this browser can't show a WebGPU canvas here: ${report.capabilities.webgpu.canvasProblem}`);
    }
    if (args.backend !== "webgpu" && !report.capabilities.webgl2 && !report.capabilities.webgpu) {
      throw new Failure("unsupported", "this browser has neither WebGPU nor WebGL2 here");
    }
    if (args.backend === "webgl2" && !report.capabilities.webgl2) {
      throw new Failure("unsupported", "this browser has no WebGL2 here");
    }

    const context = await browser.newContext({ viewport: { width: 800, height: 600 } });
    if (args.backend === "webgl2") {
      // Hide WebGPU so the engine takes its WebGL2 fallback.
      await context.addInitScript(() => {
        Object.defineProperty(Navigator.prototype, "gpu", { get: () => undefined, configurable: true });
      });
    }
    if (args.case === "missing-asset") {
      await context.route("**/assets/crab.png", (route) => route.fulfill({ status: 404, body: "gone" }));
    }
    const page = await context.newPage();
    page.on("console", (m) => report.console.push({ type: m.type(), text: m.text(), at: elapsed() }));
    page.on("pageerror", (e) => report.pageErrors.push({ message: String(e.stack ?? e), at: elapsed() }));
    page.on("response", (r) => {
      const path = new URL(r.url()).pathname;
      report.requests.push({ path, status: r.status() });
    });
    page.on("requestfailed", (r) =>
      report.requests.push({ path: new URL(r.url()).pathname, status: "failed", error: r.failure()?.errorText }),
    );

    const query = args.case === "panic" || args.case === "stall" ? `?smoke_case=${args.case}` : "";
    log(`loading ${url}${query}`);
    await page.goto(url + query);

    // An app failure shows as a panic in the console, an uncaught page
    // error, the page's error banner or a failed asset request.
    const appFailure = (status) => {
      const badAsset = report.requests.find((r) => r.path.includes("/assets/") && r.status !== 200);
      if (badAsset) return new Failure("asset", `asset request ${badAsset.path} answered ${badAsset.status}`);
      const panic = report.console.find((m) => m.text.includes("panicked at"));
      if (panic) return new Failure("panic", panic.text.split("\n").slice(0, 3).join(" | "));
      if (report.pageErrors.length) return new Failure("page-error", report.pageErrors[0].message.split("\n")[0]);
      if (status) return new Failure("page-error", status);
      return null;
    };

    // 1. Startup: the first frame publishes the state.
    const deadline = Date.now() + args.timeout * 1000;
    let snapshot;
    for (;;) {
      snapshot = await smokeState(page);
      const failure = appFailure(snapshot.status);
      if (failure) throw failure;
      if (snapshot.state && snapshot.state.ticks > 0) break;
      if (Date.now() > deadline) {
        throw new Failure("startup-timeout", `no running frame within ${args.timeout}s (state: ${JSON.stringify(snapshot.state)})`);
      }
      await sleep(100);
    }
    report.startupSeconds = elapsed();
    report.backend = await canvasBackend(page);
    log(`started after ${report.startupSeconds.toFixed(1)}s on ${report.backend}: ${JSON.stringify(snapshot.state)}`);
    if (args.backend !== "auto" && report.backend !== args.backend) {
      throw new Failure("backend", `asked for ${args.backend} but the canvas uses ${report.backend}`);
    }

    // 2. Assets: the index and the texture were fetched and decoded.
    for (const path of ["/assets/index.json", "/assets/crab.png"]) {
      if (!report.requests.some((r) => r.path === path && r.status === 200)) {
        throw new Failure("asset", `${path} was never fetched`);
      }
    }
    if (JSON.stringify(snapshot.state.asset) !== "[32,32]") {
      throw new Failure("asset", `crab.png decoded as ${JSON.stringify(snapshot.state.asset)}, not 32x32`);
    }
    // Assets from the game's in-memory asset source load by name in the
    // browser too, and are never fetched.
    if (JSON.stringify(snapshot.state.source) !== "[4,2]") {
      throw new Failure("asset", `the asset source's image decoded as ${JSON.stringify(snapshot.state.source)}, not 4x2`);
    }
    if (snapshot.state.stream !== true) {
      throw new Failure("asset", "streamed music from the asset source did not load");
    }
    const fetchedSource = report.requests.find((r) => r.path.includes("/assets/embedded/"));
    if (fetchedSource) {
      throw new Failure("asset", `${fetchedSource.path} was fetched, but it comes from an asset source`);
    }

    // 3. Progress: the fixed-step simulation keeps ticking. The bound is
    //    coarse, not a frame-rate assertion: 60 Hz for 3 s gives ~180 ticks,
    //    and the bound must stay above STALL_AFTER in examples/web_smoke.rs,
    //    since a `stall` run seen early (at tick 1) still advances that far.
    const before = snapshot.state;
    await sleep(3000);
    snapshot = await smokeState(page);
    let failure = appFailure(snapshot.status);
    if (failure) throw failure;
    report.progress = { before, after: snapshot.state };
    log(`after 3s: ${JSON.stringify(snapshot.state)}`);
    if (snapshot.state.frames <= before.frames) {
      throw new Failure("stall", `frames stopped at ${before.frames}`);
    }
    if (snapshot.state.ticks < before.ticks + MIN_PROGRESS_TICKS) {
      throw new Failure("stall", `the simulation stalled: ${before.ticks} -> ${snapshot.state.ticks} ticks in 3s`);
    }

    // 4. Input: a click on the canvas reaches the game.
    const box = await page.locator(GAME_CANVAS).boundingBox();
    await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    const clickDeadline = Date.now() + 10_000;
    for (;;) {
      snapshot = await smokeState(page);
      if (snapshot.state.clicks >= 1) break;
      if (Date.now() > clickDeadline) throw new Failure("input", "the game never saw the click");
      await sleep(100);
    }
    log(`click seen: ${JSON.stringify(snapshot.state)}`);

    // 5. Rendering: the canvas shows the background and the sprites.
    const png = await page.locator(GAME_CANVAS).screenshot();
    await writeFile(join(args.out, "canvas.png"), png);
    const blank = (stats) => stats.colors < 3 || stats.foreground < 0.001;
    report.canvas = { screenshot: await canvasStats(page, png) };
    if (blank(report.canvas.screenshot)) {
      report.canvas.readback = await canvasStats(page, null);
    }
    log(`canvas: ${JSON.stringify(report.canvas)}`);
    if (Object.values(report.canvas).every(blank)) {
      if (report.backend === "webgpu" && report.capabilities.webgpu?.canvas !== true) {
        throw new Failure("unsupported", `blank WebGPU canvas, but a plain WebGPU canvas fails here too: ${report.capabilities.webgpu.canvasProblem}`);
      }
      throw new Failure("blank", `the canvas looks blank: ${JSON.stringify(report.canvas)}`);
    }
    failure = appFailure(null);
    if (failure) throw failure;
    report.finalState = snapshot.state;

    if (CASES[args.case]) {
      await finish(EXIT.app, "missed", `--case ${args.case} should have failed with "${CASES[args.case]}" but passed`);
    } else {
      await finish(EXIT.pass, "pass", `passed on ${report.backend}`);
    }
  } catch (e) {
    const kind = e instanceof Failure ? e.kind : "harness";
    const message = e instanceof Failure ? e.message : String(e.stack ?? e);
    if (browser?.isConnected()) {
      for (const page of browser.contexts().flatMap((c) => c.pages())) {
        await page.screenshot({ path: join(args.out, "page.png") }).catch(() => {});
      }
    }
    if (kind === "unsupported") {
      await finish(args.allowUnsupported ? EXIT.pass : EXIT.unsupported, kind, message);
    } else if (CASES[args.case]) {
      const expected = CASES[args.case] === kind;
      await finish(expected ? EXIT.pass : EXIT.app, expected ? "expected-failure" : kind,
        expected ? `detected the expected ${kind} failure: ${message}` : message);
    } else {
      await finish(EXIT.app, kind, message);
    }
  } finally {
    await browser?.close().catch(() => {});
    server.close();
  }
  const { kind } = report.outcome;
  const message = report.outcome.message.replace(/\s*\n\s*/g, " ").trim();
  const label = kind === "unsupported" ? "UNSUPPORTED" : outcome === EXIT.pass ? "PASS" : "FAIL";
  console.log(`${label} (${kind}): ${message} [${report.elapsedSeconds.toFixed(1)}s, report in ${args.out}]`);
  process.exit(outcome);
}

main();
