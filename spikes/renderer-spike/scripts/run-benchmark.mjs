import { chromium } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";

const options = parseOptions(process.argv.slice(2));
const browser = await chromium.launch({
  channel: options.channel,
  headless: true,
  args: ["--enable-unsafe-webgpu", "--use-angle=metal"],
});

try {
  const scenarios = [
    { name: "desktop-webgl-500", backend: "webgl", count: 500, width: 1280, height: 720 },
    { name: "desktop-webgl-2000", backend: "webgl", count: 2000, width: 1280, height: 720 },
    { name: "mobile-webgl-500", backend: "webgl", count: 500, width: 390, height: 844 },
    { name: "mobile-webgl-2000", backend: "webgl", count: 2000, width: 390, height: 844 },
    { name: "desktop-webgpu-2000", backend: "webgpu", count: 2000, width: 1280, height: 720 },
    { name: "mobile-webgpu-2000", backend: "webgpu", count: 2000, width: 390, height: 844 },
  ];
  const results = [];
  await mkdir(options.output, { recursive: true });

  for (const scenario of scenarios) {
    const page = await browser.newPage({ viewport: { width: scenario.width, height: scenario.height } });
    const url = new URL(options.url);
    url.searchParams.set("backend", scenario.backend);
    url.searchParams.set("count", String(scenario.count));
    url.searchParams.set("warmup", String(options.warmup));
    url.searchParams.set("frames", String(options.frames));
    await page.goto(url.toString(), { waitUntil: "networkidle" });
    await page.waitForFunction(
      () => window.__PHASE0_RESULT__ !== undefined || window.__PHASE0_ERROR__ !== undefined,
      undefined,
      { timeout: 60_000 },
    );
    const failure = await page.evaluate(() => window.__PHASE0_ERROR__);
    if (failure !== undefined) {
      throw new Error(`${scenario.name}: ${failure}`);
    }
    const result = await page.evaluate(() => window.__PHASE0_RESULT__);
    await page.screenshot({ path: path.join(options.output, `${scenario.name}.png`) });
    results.push({ scenario: scenario.name, ...result });
    await page.close();
  }

  const record = {
    benchmark_schema_version: 1,
    benchmark_id: `${options.benchmarkId}-renderer`,
    generated_at: new Date().toISOString(),
    browser_channel: options.channel,
    browser_version: browser.version(),
    headless: true,
    launch_args: ["--enable-unsafe-webgpu", "--use-angle=metal"],
    method: { warmup_frames: options.warmup, sampled_frames: options.frames },
    results,
    limitations: [
      "Local development host, not deployment VM or physical mobile hardware.",
      "Browser viewport emulation does not establish mobile GPU performance.",
      "WebGPU availability and adapter behavior can differ in non-headless target browsers.",
    ],
  };
  const outputPath = path.join(options.output, "renderer-results.json");
  await writeFile(outputPath, `${JSON.stringify(record, null, 2)}\n`, "utf8");
  process.stdout.write(`${outputPath}\n`);
} finally {
  await browser.close();
}

function parseOptions(args) {
  const values = new Map();
  for (let index = 0; index < args.length; index += 2) {
    const key = args[index];
    const value = args[index + 1];
    if (key === undefined || value === undefined || !key.startsWith("--")) {
      throw new Error("expected --name value arguments");
    }
    values.set(key.slice(2), value);
  }
  const output = values.get("output");
  const benchmarkId = values.get("benchmark-id");
  if (output === undefined || benchmarkId === undefined) {
    throw new Error("--output and --benchmark-id are required");
  }
  return {
    output,
    benchmarkId,
    url: values.get("url") ?? "http://127.0.0.1:4173/",
    channel: values.get("channel") ?? "chrome",
    warmup: boundedInteger(values.get("warmup") ?? "30", 0, 600),
    frames: boundedInteger(values.get("frames") ?? "120", 10, 2_000),
  };
}

function boundedInteger(value, minimum, maximum) {
  const parsed = Number.parseInt(value, 10);
  if (!Number.isSafeInteger(parsed) || parsed < minimum || parsed > maximum) {
    throw new Error(`integer must be between ${minimum} and ${maximum}`);
  }
  return parsed;
}

