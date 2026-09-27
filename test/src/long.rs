//! `long`: overnight waves (stormcentral `docs/test-standard.md`, "Overnight
//! soaks"). stormview's workload is reading and writing the contract, so a
//! wave is readers at the pod's capacity: half poll the node's feeds and
//! read every snapshot with this commit's types (the node's daemons serving
//! under load), half write and read a synthetic feed sized from the pod's
//! memory (the contract's own cost). Waves vary in size, one to three times
//! the base, and each is measured against the first wave of its size:
//! latency (p50/p95), errors, and what is left behind in this process —
//! resident memory and open descriptors — after the wave drains. A wave that
//! is slower than its first, or residue that grows, fails the `trend` line
//! even when every read passed.

use crate::contract::example;
use crate::env::Env;
use crate::feeds::{self, Feed};
use crate::report::{Outcome, Report};
use crate::suites::probe_all;
use serde_json::{json, Map, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use stormview::{ComponentSummary, Health, Relation};

/// CPUs this pod may use: its cgroup quota, else the machine's.
fn cpus() -> usize {
    let quota = std::fs::read_to_string("/sys/fs/cgroup/cpu.max").ok().and_then(|s| {
        let mut it = s.split_whitespace();
        let q: f64 = it.next()?.parse().ok()?; // "max" doesn't parse: no quota
        let p: f64 = it.next()?.parse().ok()?;
        Some((q / p).ceil() as usize)
    });
    quota.unwrap_or_else(|| std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)).max(1)
}

/// Memory this pod may use: its cgroup limit, else what the machine has
/// available.
fn memory() -> u64 {
    let limit = std::fs::read_to_string("/sys/fs/cgroup/memory.max").ok().and_then(|s| s.trim().parse::<u64>().ok());
    limit.unwrap_or_else(|| proc_kib("/proc/meminfo", "MemAvailable:").unwrap_or(1 << 20) * 1024)
}

fn proc_kib(file: &str, key: &str) -> Option<u64> {
    std::fs::read_to_string(file).ok()?.lines().find(|l| l.starts_with(key))?.split_whitespace().nth(1)?.parse().ok()
}

fn rss_kib() -> u64 {
    proc_kib("/proc/self/status", "VmRSS:").unwrap_or(0)
}

fn fds() -> usize {
    std::fs::read_dir("/proc/self/fd").map(|d| d.count()).unwrap_or(0)
}

/// A feed of `n` summaries shaped like a real one: a system, and components
/// that belong to it, with metrics, actions and a has_many edge each.
pub fn synthetic(n: usize) -> Vec<ComponentSummary> {
    let mut sys = example();
    sys.id = "system".into();
    sys.kind = "system".into();
    sys.relations = vec![Relation::has_many("members", (1..n).map(|i| format!("c:{i}")).collect())];
    let mut out = vec![sys];
    for i in 1..n {
        let mut c = example();
        c.id = format!("c:{i}");
        c.label = format!("component {i}");
        c.health = [Health::Ok, Health::Warn, Health::Idle][i % 3];
        c.relations = vec![Relation::belongs_to("system", "system")];
        c.link = Some(format!("#/c/{i}"));
        out.push(c);
    }
    out
}

fn percentile(v: &mut [u64], p: f64) -> u64 {
    if v.is_empty() {
        return 0;
    }
    v.sort_unstable();
    v[((v.len() - 1) as f64 * p).round() as usize]
}

#[derive(Default)]
struct Tally {
    feed_us: Vec<u64>,
    synth_us: Vec<u64>,
    errors: u64,
    first_error: Option<String>,
    counts: std::collections::BTreeMap<String, (usize, usize)>,
}

struct Wave {
    n: usize,
    factor: usize,
    feed_p95: u64,
    synth_p95: u64,
    rss: u64,
    fds: usize,
}

fn run_wave(feeds: &[Feed], workers: usize, synth: &str, until: Instant) -> Tally {
    let tally = Mutex::new(Tally::default());
    let stop = AtomicBool::new(false);
    std::thread::scope(|s| {
        for w in 0..workers {
            let (tally, stop) = (&tally, &stop);
            s.spawn(move || {
                let mut local = Tally::default();
                let mut i = w;
                while Instant::now() < until && !stop.load(Ordering::Relaxed) {
                    i += workers;
                    if !feeds.is_empty() && w % 2 == 0 {
                        let f = &feeds[(i / workers) % feeds.len()];
                        match feeds::read_timed(&f.addr, Duration::from_secs(10)) {
                            Ok((us, n)) => {
                                local.feed_us.push(us);
                                let e = local.counts.entry(f.name.clone()).or_insert((n, n));
                                *e = (e.0.min(n), e.1.max(n));
                            }
                            Err(e) => {
                                local.errors += 1;
                                local.first_error.get_or_insert(format!("{}: {e}", f.name));
                                std::thread::sleep(Duration::from_millis(200));
                            }
                        }
                    } else {
                        let t = Instant::now();
                        let r = serde_json::from_str::<Vec<ComponentSummary>>(synth)
                            .map_err(|e| e.to_string())
                            .and_then(|f| serde_json::to_string(&f).map_err(|e| e.to_string()));
                        match r {
                            Ok(back) if back == synth => local.synth_us.push(t.elapsed().as_micros() as u64),
                            Ok(_) => {
                                local.errors += 1;
                                local.first_error.get_or_insert("the synthetic feed does not write back as it was read".into());
                                stop.store(true, Ordering::Relaxed);
                            }
                            Err(e) => {
                                local.errors += 1;
                                local.first_error.get_or_insert(format!("synthetic feed: {e}"));
                                stop.store(true, Ordering::Relaxed);
                            }
                        }
                    }
                }
                let mut t = tally.lock().unwrap();
                t.feed_us.append(&mut local.feed_us);
                t.synth_us.append(&mut local.synth_us);
                t.errors += local.errors;
                if t.first_error.is_none() {
                    t.first_error = local.first_error;
                }
                for (k, (lo, hi)) in local.counts {
                    let e = t.counts.entry(k).or_insert((lo, hi));
                    *e = (e.0.min(lo), e.1.max(hi));
                }
            });
        }
    });
    tally.into_inner().unwrap()
}

pub fn run(env: &Env, r: &mut Report) {
    let feeds = probe_all(env, r);
    let (cpus, mem) = (cpus(), memory());
    let base = (cpus * 2).clamp(2, 32);
    // A quarter of the pod's memory across the largest wave's synthetic
    // readers, at ~4 KiB per summary held (text, parsed, written back).
    let per = (mem / 4 / (3 * base as u64) / 4096).clamp(200, 20_000) as usize;
    let synth = serde_json::to_string(&synthetic(per)).expect("synthetic feed serializes");
    let wave_len = (env.remaining() / 24).clamp(Duration::from_secs(30), Duration::from_secs(600));
    let mut waves: Vec<Wave> = Vec::new();
    while env.remaining() > wave_len + Duration::from_secs(30) {
        let n = waves.len() + 1;
        let factor = [1, 2, 3][(n - 1) % 3];
        let workers = base * factor;
        let t = Instant::now();
        let mut tally = run_wave(&feeds, workers, &synth, t + wave_len);
        let ms = t.elapsed().as_millis();
        // Drain: the wave's threads and sockets are gone; let the allocator
        // and the kernel settle before reading what is left.
        std::thread::sleep(Duration::from_secs(2));
        let w = Wave {
            n,
            factor,
            feed_p95: percentile(&mut tally.feed_us, 0.95),
            synth_p95: percentile(&mut tally.synth_us, 0.95),
            rss: rss_kib(),
            fds: fds(),
        };
        let mut extra = Map::new();
        extra.insert("wave".into(), json!(n));
        extra.insert("workers".into(), json!(workers));
        extra.insert("feed_reads".into(), json!(tally.feed_us.len()));
        extra.insert("feed_p50_us".into(), json!(percentile(&mut tally.feed_us, 0.5)));
        extra.insert("feed_p95_us".into(), json!(w.feed_p95));
        extra.insert("synthetic_summaries".into(), json!(per));
        extra.insert("synthetic_rounds".into(), json!(tally.synth_us.len()));
        extra.insert("synthetic_p50_us".into(), json!(percentile(&mut tally.synth_us, 0.5)));
        extra.insert("synthetic_p95_us".into(), json!(w.synth_p95));
        extra.insert("errors".into(), json!(tally.errors));
        extra.insert("rss_kib".into(), json!(w.rss));
        extra.insert("fds".into(), json!(w.fds));
        extra.insert(
            "feed_components".into(),
            Value::Object(tally.counts.iter().map(|(k, (lo, hi))| (k.clone(), json!([lo, hi]))).collect()),
        );
        let reads = tally.feed_us.len() + tally.synth_us.len();
        let outcome = match tally.first_error {
            Some(e) => Outcome::Fail(format!("{} error(s) in {reads} reads; first: {e}", tally.errors)),
            None => Outcome::Pass(format!("{workers} readers for {}s, {reads} reads", wave_len.as_secs())),
        };
        r.record(&format!("wave-{n}"), outcome, ms, Some(extra));
        waves.push(w);
    }
    r.run("trend", || trend(&waves));
}

/// Slowdown against the first wave of the same size; residue against the
/// first wave.
fn trend(waves: &[Wave]) -> Outcome {
    if waves.len() < 2 {
        return Outcome::Skip(format!("{} wave(s) in the window; a trend needs two", waves.len()));
    }
    let first = &waves[0];
    for w in &waves[1..] {
        if w.rss > first.rss + (first.rss / 2).max(16 * 1024) {
            return Outcome::Fail(format!("after wave {} this process holds {} KiB, after wave 1 {} KiB", w.n, w.rss, first.rss));
        }
        if w.fds > first.fds + 8 {
            return Outcome::Fail(format!("after wave {} {} descriptors are open, after wave 1 {}", w.n, w.fds, first.fds));
        }
        // The first wave of this size; a wave that is its own first has
        // nothing to be slower than.
        let base = waves.iter().find(|b| b.factor == w.factor).unwrap_or(w);
        if base.n == w.n {
            continue;
        }
        if w.feed_p95 > 2 * base.feed_p95 + 20_000 {
            return Outcome::Fail(format!(
                "wave {} reads the node's feeds slower: p95 {} µs against wave {}'s {} µs",
                w.n, w.feed_p95, base.n, base.feed_p95
            ));
        }
        if w.synth_p95 > 2 * base.synth_p95 + 5_000 {
            return Outcome::Fail(format!(
                "wave {} reads the synthetic feed slower: p95 {} µs against wave {}'s {} µs",
                w.n, w.synth_p95, base.n, base.synth_p95
            ));
        }
    }
    let last = waves.last().unwrap();
    Outcome::Pass(format!(
        "{} waves, none slower than the first of its size; {} KiB and {} descriptors after the last, {} KiB and {} after the first",
        waves.len(),
        last.rss,
        last.fds,
        first.rss,
        first.fds
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_synthetic_feed_is_well_formed_and_round_trips() {
        let f = synthetic(50);
        assert_eq!(feeds::integrity(&f), Vec::<String>::new());
        let s = serde_json::to_string(&f).unwrap();
        let back: Vec<ComponentSummary> = serde_json::from_str(&s).unwrap();
        assert_eq!(serde_json::to_string(&back).unwrap(), s);
    }

    fn wave(n: usize, factor: usize, feed_p95: u64, rss: u64) -> Wave {
        Wave { n, factor, feed_p95, synth_p95: 100, rss, fds: 5 }
    }

    #[test]
    fn trend_compares_like_sized_waves() {
        // Wave 2 is twice the size and slower: not a regression.
        let ok = [wave(1, 1, 1000, 10_000), wave(2, 2, 30_000, 10_000), wave(3, 3, 60_000, 10_000), wave(4, 1, 1500, 10_000)];
        assert!(matches!(trend(&ok), Outcome::Pass(_)));
        let slow = [wave(1, 1, 1000, 10_000), wave(2, 2, 1000, 10_000), wave(3, 3, 1000, 10_000), wave(4, 1, 90_000, 10_000)];
        assert!(matches!(trend(&slow), Outcome::Fail(d) if d.starts_with("wave 4")));
        let leak = [wave(1, 1, 1000, 10_000), wave(2, 2, 1000, 40_000)];
        assert!(matches!(trend(&leak), Outcome::Fail(d) if d.contains("KiB")));
        assert!(matches!(trend(&ok[..1]), Outcome::Skip(_)));
    }

    #[test]
    fn percentiles() {
        let mut v = vec![5, 1, 4, 2, 3];
        assert_eq!(percentile(&mut v, 0.5), 3);
        assert_eq!(percentile(&mut v, 0.95), 5);
        assert_eq!(percentile(&mut [], 0.5), 0);
    }
}
