use lifesim_phase0_spike::{TickConfig, TickTimings, World, decode_snapshot, encode_snapshot};
use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const DEFAULT_SEED: u64 = 0x5eed_cafe_f00d_beef;

fn main() {
    if let Err(error) = run() {
        eprintln!("phase0-bench: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("fixture") => run_fixture(parse_options(args.collect())?),
        Some("benchmark") => run_benchmark(parse_options(args.collect())?),
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage: phase0-bench fixture [--organisms N --ticks N --seed N]\n       phase0-bench benchmark --benchmark-id ID --output DIR [--organisms N --seed N --warmup N --samples N --ticks-per-sample N]".to_owned()
}

#[derive(Default)]
struct Options {
    organisms: Option<usize>,
    ticks: Option<u64>,
    seed: Option<u64>,
    warmup: Option<u64>,
    samples: Option<usize>,
    ticks_per_sample: Option<u64>,
    benchmark_id: Option<String>,
    output: Option<PathBuf>,
}

fn parse_options(args: Vec<String>) -> Result<Options, String> {
    let mut options = Options::default();
    let mut index = 0;
    while index < args.len() {
        let name = &args[index];
        let value = args.get(index + 1).ok_or_else(usage)?;
        match name.as_str() {
            "--organisms" => options.organisms = Some(parse_number(name, value)?),
            "--ticks" => options.ticks = Some(parse_number(name, value)?),
            "--seed" => options.seed = Some(parse_seed(value)?),
            "--warmup" => options.warmup = Some(parse_number(name, value)?),
            "--samples" => options.samples = Some(parse_number(name, value)?),
            "--ticks-per-sample" => options.ticks_per_sample = Some(parse_number(name, value)?),
            "--benchmark-id" => options.benchmark_id = Some(value.clone()),
            "--output" => options.output = Some(PathBuf::from(value)),
            _ => return Err(format!("unknown option {name}\n{}", usage())),
        }
        index += 2;
    }
    Ok(options)
}

fn parse_number<T>(name: &str, value: &str) -> Result<T, String>
where
    T: std::str::FromStr,
{
    value
        .parse()
        .map_err(|_| format!("invalid value for {name}: {value}"))
}

fn parse_seed(value: &str) -> Result<u64, String> {
    value
        .strip_prefix("0x")
        .map_or_else(|| value.parse(), |hex| u64::from_str_radix(hex, 16))
        .map_err(|_| format!("invalid seed: {value}"))
}

fn run_fixture(options: Options) -> Result<(), String> {
    let organisms = options.organisms.unwrap_or(500);
    let ticks = options.ticks.unwrap_or(500);
    let seed = options.seed.unwrap_or(DEFAULT_SEED);
    let config = TickConfig::new(organisms, seed).map_err(|error| error.to_string())?;
    let mut world = World::synthetic(config);
    for _ in 0..ticks {
        world.step();
    }
    let snapshot = encode_snapshot(&world);
    println!(
        "{{\"fixture_schema_version\":1,\"organisms\":{organisms},\"ticks\":{ticks},\"seed\":\"0x{seed:016x}\",\"config_hash\":\"0x{:016x}\",\"state_checksum\":\"0x{:016x}\",\"snapshot_crc32\":\"0x{:08x}\",\"snapshot_bytes\":{}}}",
        config.stable_hash(),
        world.state_checksum(),
        crc32_for_report(&snapshot),
        snapshot.len()
    );
    Ok(())
}

fn run_benchmark(options: Options) -> Result<(), String> {
    let organisms = options.organisms.unwrap_or(500);
    let seed = options.seed.unwrap_or(DEFAULT_SEED);
    let warmup = options.warmup.unwrap_or(100);
    let samples = options.samples.unwrap_or(50);
    let ticks_per_sample = options.ticks_per_sample.unwrap_or(10);
    if samples == 0 || ticks_per_sample == 0 {
        return Err("samples and ticks-per-sample must be positive".to_owned());
    }
    let benchmark_id = options
        .benchmark_id
        .ok_or_else(|| "--benchmark-id is required".to_owned())?;
    let output_dir = options
        .output
        .ok_or_else(|| "--output is required".to_owned())?;
    fs::create_dir_all(&output_dir).map_err(io_error)?;

    let config = TickConfig::new(organisms, seed).map_err(|error| error.to_string())?;
    let mut world = World::synthetic(config);
    for _ in 0..warmup {
        world.step();
    }

    let mut tick_samples = Vec::with_capacity(samples);
    let mut phase_samples = Vec::with_capacity(samples);
    for _ in 0..samples {
        let mut aggregate = TickTimings::default();
        for _ in 0..ticks_per_sample {
            let timing = world.step_profiled();
            aggregate.clock += timing.clock;
            aggregate.spatial_index += timing.spatial_index;
            aggregate.sense_and_controller += timing.sense_and_controller;
            aggregate.apply += timing.apply;
            aggregate.checksum += timing.checksum;
            aggregate.total += timing.total;
        }
        let divisor = ticks_per_sample as f64;
        tick_samples.push(aggregate.total.as_secs_f64() * 1_000_000.0 / divisor);
        phase_samples.push([
            aggregate.clock.as_secs_f64() * 1_000_000.0 / divisor,
            aggregate.spatial_index.as_secs_f64() * 1_000_000.0 / divisor,
            aggregate.sense_and_controller.as_secs_f64() * 1_000_000.0 / divisor,
            aggregate.apply.as_secs_f64() * 1_000_000.0 / divisor,
            aggregate.checksum.as_secs_f64() * 1_000_000.0 / divisor,
        ]);
    }

    let mut encode_samples = Vec::with_capacity(samples);
    let mut decode_samples = Vec::with_capacity(samples);
    let snapshot = encode_snapshot(&world);
    for _ in 0..samples {
        let started = Instant::now();
        let encoded = encode_snapshot(&world);
        encode_samples.push(started.elapsed().as_secs_f64() * 1_000_000.0);
        std::hint::black_box(&encoded);

        let started = Instant::now();
        let decoded = decode_snapshot(&snapshot).map_err(|error| error.to_string())?;
        decode_samples.push(started.elapsed().as_secs_f64() * 1_000_000.0);
        std::hint::black_box(decoded.state_checksum());
    }

    let raw_path = output_dir.join(format!("rust-{organisms}-raw.csv"));
    let mut raw = File::create(&raw_path).map_err(io_error)?;
    writeln!(raw, "sample,tick_us,clock_us,spatial_us,sense_controller_us,apply_us,checksum_us,snapshot_encode_us,snapshot_decode_us").map_err(io_error)?;
    for sample in 0..samples {
        writeln!(
            raw,
            "{sample},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3}",
            tick_samples[sample],
            phase_samples[sample][0],
            phase_samples[sample][1],
            phase_samples[sample][2],
            phase_samples[sample][3],
            phase_samples[sample][4],
            encode_samples[sample],
            decode_samples[sample]
        )
        .map_err(io_error)?;
    }

    let tick_stats = summarize(&tick_samples);
    let encode_stats = summarize(&encode_samples);
    let decode_stats = summarize(&decode_samples);
    let rss_bytes = current_rss_bytes();
    let summary_path = output_dir.join(format!("rust-{organisms}-summary.json"));
    let generated_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs();
    let summary = format!(
        concat!(
            "{{\n",
            "  \"benchmark_schema_version\": 1,\n",
            "  \"benchmark_id\": \"{}-rust-{}\",\n",
            "  \"generated_at_unix_seconds\": {},\n",
            "  \"revision\": \"{}\",\n",
            "  \"working_tree_dirty\": {},\n",
            "  \"toolchain\": \"{}\",\n",
            "  \"build_profile\": \"release-lto-thin\",\n",
            "  \"os\": \"{}\",\n",
            "  \"architecture\": \"{}\",\n",
            "  \"cpu\": \"{}\",\n",
            "  \"host_memory_bytes\": {},\n",
            "  \"scenario\": {{\"organisms\": {}, \"world_units\": 256, \"observers\": 0, \"tick_millis\": 100}},\n",
            "  \"config_hash\": \"0x{:016x}\",\n",
            "  \"seed\": \"0x{:016x}\",\n",
            "  \"method\": {{\"warmup_ticks\": {}, \"samples\": {}, \"ticks_per_sample\": {}, \"deterministic_mode\": \"strict-spike-v1\"}},\n",
            "  \"tick_microseconds\": {},\n",
            "  \"snapshot\": {{\"format_version\": 1, \"compression\": \"none\", \"bytes\": {}, \"encode_microseconds\": {}, \"decode_microseconds\": {}}},\n",
            "  \"rss_bytes_at_completion\": {},\n",
            "  \"final_tick\": {},\n",
            "  \"final_state_checksum\": \"0x{:016x}\",\n",
            "  \"raw_samples\": \"{}\",\n",
            "  \"limitations\": [\"local development host, not deployment VM\", \"RSS is sampled at completion, not peak\", \"snapshot payload is uncompressed\"]\n",
            "}}\n"
        ),
        json_escape(&benchmark_id),
        organisms,
        generated_at,
        json_escape(&git_revision()),
        git_dirty(),
        json_escape(
            &command_output("rustc", &["--version"]).unwrap_or_else(|| "unknown".to_owned())
        ),
        json_escape(&os_description()),
        env::consts::ARCH,
        json_escape(&sysctl("machdep.cpu.brand_string").unwrap_or_else(|| "unknown".to_owned())),
        sysctl("hw.memsize").unwrap_or_else(|| "0".to_owned()),
        organisms,
        config.stable_hash(),
        seed,
        warmup,
        samples,
        ticks_per_sample,
        stats_json(tick_stats),
        snapshot.len(),
        stats_json(encode_stats),
        stats_json(decode_stats),
        rss_bytes,
        world.tick_number(),
        world.state_checksum(),
        json_escape(&raw_path.to_string_lossy())
    );
    fs::write(&summary_path, &summary).map_err(io_error)?;
    print!("{summary}");
    Ok(())
}

#[derive(Clone, Copy)]
struct Stats {
    p50: f64,
    p95: f64,
    p99: f64,
    min: f64,
    max: f64,
}

fn summarize(samples: &[f64]) -> Stats {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    Stats {
        p50: percentile(&sorted, 0.50),
        p95: percentile(&sorted, 0.95),
        p99: percentile(&sorted, 0.99),
        min: sorted[0],
        max: *sorted.last().expect("non-empty samples"),
    }
}

fn percentile(sorted: &[f64], percentile: f64) -> f64 {
    let index = ((sorted.len() - 1) as f64 * percentile).ceil() as usize;
    sorted[index]
}

fn stats_json(stats: Stats) -> String {
    format!(
        "{{\"p50\":{:.3},\"p95\":{:.3},\"p99\":{:.3},\"min\":{:.3},\"max\":{:.3}}}",
        stats.p50, stats.p95, stats.p99, stats.min, stats.max
    )
}

fn current_rss_bytes() -> u64 {
    let pid = std::process::id().to_string();
    command_output("ps", &["-o", "rss=", "-p", &pid])
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(0)
        .saturating_mul(1024)
}

fn git_revision() -> String {
    command_output("git", &["rev-parse", "HEAD"]).unwrap_or_else(|| "unborn-main".to_owned())
}

fn git_dirty() -> bool {
    Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .map(|output| !output.stdout.is_empty())
        .unwrap_or(true)
}

fn os_description() -> String {
    command_output("sw_vers", &["-productName"])
        .zip(command_output("sw_vers", &["-productVersion"]))
        .map(|(name, version)| format!("{name} {version}"))
        .unwrap_or_else(|| env::consts::OS.to_owned())
}

fn sysctl(name: &str) -> Option<String> {
    command_output("sysctl", &["-n", name])
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn io_error(error: io::Error) -> String {
    error.to_string()
}

fn crc32_for_report(bytes: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

#[allow(dead_code)]
fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    Ok(())
}
