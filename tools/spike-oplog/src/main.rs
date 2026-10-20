//! A3 — op-log replay cost spike.
//!
//! The question (plan §2, unit A3): the design note assumes replay is cheap
//! because the entity count is small. That is the wrong variable. **Replay cost
//! grows with operation count, not entity count.** This measures the curve.
//!
//! Throwaway. Not wired into the crate, not imported by `src/`, its own
//! workspace and lockfile.
//!
//! Three layers are timed separately, because which one the kill criterion
//! applies to is a real question and not a detail:
//!
//! * `apply` — sort by (clock, pack id, position) and fold, over packs already
//!   in memory. The irreducible part.
//! * `parse+apply` — deserialise every pack from bytes, then the above. The
//!   honest cost of one entity read in a process that has not seen the packs
//!   before, which is every `jjj` invocation (one-shot CLI) and every MCP tool
//!   call (design note §5 caveat 4 requires re-reading from disk).
//! * `read+parse+apply` — plus the filesystem reads.
//!
//! Against a baseline of the current state-based read: split `---` frontmatter,
//! parse it with `serde_norway`, take the body. Same content, same parser as
//! `src/storage/`.
//!
//! Usage (release only — a debug build measures the debug allocator):
//!
//! ```text
//! cargo run --release -- --out /tmp/spike --body 1200
//! ```

mod model;
mod synth;

use model::{LoadedPack, StoredPack, WireFormat};
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const SEED: u64 = 0x5377_6172_6d43_6f72; // "SwarmCor"

/// Per-layer time budget used to pick an iteration count. Enough that p99 over
/// the resulting sample is not one scheduler hiccup.
const TARGET: Duration = Duration::from_millis(2500);
const ITERS_MIN: usize = 200;
const ITERS_MAX: usize = 40_000;

struct Sample {
    iters: usize,
    p50_ns: u128,
    p99_ns: u128,
}

/// Calibrate an iteration count against [`TARGET`], then measure. Warmup is a
/// tenth of the run, discarded, so the first-touch page faults and the
/// allocator's growth do not land in the sample.
fn measure(mut f: impl FnMut()) -> Sample {
    let probe = Instant::now();
    for _ in 0..5 {
        f();
    }
    let per = (probe.elapsed() / 5).as_nanos().max(1);
    let iters = ((TARGET.as_nanos() / per) as usize).clamp(ITERS_MIN, ITERS_MAX);

    for _ in 0..(iters / 10).max(5) {
        f();
    }
    let mut times = Vec::with_capacity(iters);
    for _ in 0..iters {
        let t = Instant::now();
        f();
        times.push(t.elapsed().as_nanos());
    }
    times.sort_unstable();
    Sample {
        iters,
        p50_ns: pct(&times, 0.50),
        p99_ns: pct(&times, 0.99),
    }
}

fn pct(sorted: &[u128], p: f64) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let i = (((sorted.len() - 1) as f64) * p).round() as usize;
    sorted[i]
}

fn ms(ns: u128) -> f64 {
    ns as f64 / 1_000_000.0
}

fn fmt_ms(ns: u128) -> String {
    let v = ms(ns);
    if v < 1.0 {
        format!("{v:.4}")
    } else {
        format!("{v:.3}")
    }
}

struct Point {
    ops: usize,
    packs: usize,
    bytes: usize,
    state_bytes: usize,
    apply: Sample,
    parse_apply: Sample,
    read_parse_apply: Sample,
    state_read: Sample,
    state_read_disk: Sample,
}

fn write_packs(dir: &Path, packs: &[StoredPack]) -> Vec<PathBuf> {
    std::fs::create_dir_all(dir).expect("mkdir");
    packs
        .iter()
        .map(|p| {
            let path = dir.join(format!("{}.json", p.id));
            std::fs::write(&path, &p.bytes).expect("write pack");
            path
        })
        .collect()
}

fn run_point(ops: usize, body_bytes: usize, fmt: WireFormat, out: &Path) -> Point {
    let packs = synth::synth_entity(SEED, ops, body_bytes, fmt);
    let bytes: usize = packs.iter().map(|p| p.bytes.len()).sum();

    let dir = out.join(format!("{}-ops-{}b-{}", ops, body_bytes, fmt.name()));
    let paths = write_packs(&dir, &packs);

    // Parsed once, for the apply-only layer.
    let loaded: Vec<LoadedPack> = packs.iter().map(|p| model::parse_pack(p, fmt)).collect();
    let state = model::replay(&loaded);

    // The state-based equivalent of the same content.
    let state_bytes_vec = synth::state_file(&state, "019c8cc2-8c89-7551-b704-6bba1d67dcd4");
    let state_path = dir.join("state.md");
    std::fs::write(&state_path, &state_bytes_vec).expect("write state file");

    let apply = measure(|| {
        black_box(model::replay(black_box(&loaded)));
    });
    let parse_apply = measure(|| {
        let l: Vec<LoadedPack> = packs.iter().map(|p| model::parse_pack(p, fmt)).collect();
        black_box(model::replay(black_box(&l)));
    });
    let read_parse_apply = measure(|| {
        let stored: Vec<StoredPack> = paths
            .iter()
            .map(|p| StoredPack {
                id: p.file_stem().unwrap().to_string_lossy().into_owned(),
                bytes: std::fs::read(p).expect("read pack"),
            })
            .collect();
        let l: Vec<LoadedPack> = stored.iter().map(|p| model::parse_pack(p, fmt)).collect();
        black_box(model::replay(black_box(&l)));
    });
    let state_read = measure(|| {
        black_box(model::read_state_file(black_box(&state_bytes_vec)));
    });
    let state_read_disk = measure(|| {
        let b = std::fs::read(&state_path).expect("read state file");
        black_box(model::read_state_file(&b));
    });

    Point {
        ops,
        packs: packs.len(),
        bytes,
        state_bytes: state_bytes_vec.len(),
        apply,
        parse_apply,
        read_parse_apply,
        state_read,
        state_read_disk,
    }
}

fn human(bytes: usize) -> String {
    if bytes >= 1 << 20 {
        format!("{:.2} MiB", bytes as f64 / (1u64 << 20) as f64)
    } else if bytes >= 1 << 10 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn main() {
    let mut out = PathBuf::from("target/spike-out");
    let mut body_sizes: Vec<usize> = vec![1200];
    let mut op_counts: Vec<usize> = vec![10, 100, 1_000, 10_000];
    let mut formats: Vec<WireFormat> = vec![WireFormat::Nested, WireFormat::Flat];

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let next = |i: usize| args.get(i + 1).cloned().unwrap_or_default();
        match args[i].as_str() {
            "--out" => {
                out = PathBuf::from(next(i));
                i += 1;
            }
            "--body" => {
                body_sizes = next(i)
                    .split(',')
                    .map(|s| s.trim().parse().expect("--body wants byte counts"))
                    .collect();
                i += 1;
            }
            "--ops" => {
                op_counts = next(i)
                    .split(',')
                    .map(|s| s.trim().parse().expect("--ops wants counts"))
                    .collect();
                i += 1;
            }
            "--format" => {
                formats = next(i)
                    .split(',')
                    .map(|s| WireFormat::parse(s.trim()).expect("--format: nested|flat"))
                    .collect();
                i += 1;
            }
            "--help" | "-h" => {
                println!(
                    "spike-oplog — A3 op-log replay cost\n\n\
                     --ops    <n,n,...>  operations per entity (default 10,100,1000,10000)\n\
                     --body   <n,n,...>  mean description bytes per body edit (default 1200)\n\
                     --format <f,f,...>  wire format: nested, flat (default both)\n\
                     --out    <dir>      where packs are written (default target/spike-out)\n\n\
                     Build with --release; a debug build measures the debug allocator."
                );
                return;
            }
            other => {
                eprintln!("unknown argument: {other} (try --help)");
                std::process::exit(2);
            }
        }
        i += 1;
    }

    println!("# A3 — op-log replay cost\n");
    println!("Seed `{SEED:#x}`. Op mix per 100: {d} body, {t} title, {s} status, {p} priority, {at}+{rt} tag churn, {a} assignee, {pa} parent.",
        d = synth::MIX.description, t = synth::MIX.title, s = synth::MIX.status,
        p = synth::MIX.priority, at = synth::MIX.add_tag, rt = synth::MIX.remove_tag,
        a = synth::MIX.assignee, pa = synth::MIX.parent);
    println!(
        "Packs hold {:?} ops (mean {:.2}); {:.0}% of packs are concurrent with their predecessor.",
        synth::OPS_PER_PACK,
        synth::OPS_PER_PACK.iter().sum::<usize>() as f64 / synth::OPS_PER_PACK.len() as f64,
        synth::CONCURRENT_SHARE * 100.0
    );
    println!("Times in milliseconds. Output under `{}`.\n", out.display());

    for &body_bytes in &body_sizes {
        for &fmt in &formats {
            println!(
                "## body mean {body_bytes} B, wire format `{}`\n",
                fmt.name()
            );
            println!("| ops | packs | on disk | apply p50 | apply p99 | parse+apply p50 | parse+apply p99 | read+parse+apply p50 | read+parse+apply p99 | iters (a/pa/rpa) |");
            println!("|---:|---:|---:|---:|---:|---:|---:|---:|---:|:---|");
            let mut points = Vec::new();
            for &ops in &op_counts {
                let pt = run_point(ops, body_bytes, fmt, &out);
                println!(
                    "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {}/{}/{} |",
                    pt.ops,
                    pt.packs,
                    human(pt.bytes),
                    fmt_ms(pt.apply.p50_ns),
                    fmt_ms(pt.apply.p99_ns),
                    fmt_ms(pt.parse_apply.p50_ns),
                    fmt_ms(pt.parse_apply.p99_ns),
                    fmt_ms(pt.read_parse_apply.p50_ns),
                    fmt_ms(pt.read_parse_apply.p99_ns),
                    pt.apply.iters,
                    pt.parse_apply.iters,
                    pt.read_parse_apply.iters,
                );
                points.push(pt);
            }
            println!();
            println!("| ops | state file | state read p50 | state read p99 | +disk p50 | +disk p99 | oplog/state (parse+apply p50) | bytes ratio |");
            println!("|---:|---:|---:|---:|---:|---:|---:|---:|");
            for pt in &points {
                println!(
                    "| {} | {} | {} | {} | {} | {} | {:.0}x | {:.0}x |",
                    pt.ops,
                    human(pt.state_bytes),
                    fmt_ms(pt.state_read.p50_ns),
                    fmt_ms(pt.state_read.p99_ns),
                    fmt_ms(pt.state_read_disk.p50_ns),
                    fmt_ms(pt.state_read_disk.p99_ns),
                    pt.parse_apply.p50_ns as f64 / pt.state_read.p50_ns.max(1) as f64,
                    pt.bytes as f64 / pt.state_bytes.max(1) as f64,
                );
            }
            println!();

            // The kill criterion, evaluated rather than left to the reader.
            if let Some(pt) = points.iter().find(|p| p.ops == 1_000) {
                let verdict = |label: &str, ns: u128| {
                    println!(
                        "- kill criterion ({label}, 1,000 ops): {} ms vs 50 ms — **{}**",
                        fmt_ms(ns),
                        if ms(ns) > 50.0 { "FIRED" } else { "not fired" }
                    );
                };
                verdict("apply p99", pt.apply.p99_ns);
                verdict("parse+apply p99", pt.parse_apply.p99_ns);
                verdict("read+parse+apply p99", pt.read_parse_apply.p99_ns);
                println!();
            }
        }
    }
}
