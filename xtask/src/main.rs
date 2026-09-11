//! `cargo xtask watch` — a READ-ONLY window onto the loop.
//!
//! It reads `ledger*.jsonl`, the search-track logs, `STATE.md` and the ruler outputs, and writes
//! exactly two things, both inside `watch/`: `index.html` (regenerated every 60s) and
//! `events.log` (append-only). **It never writes a file the loop reads.** That is the whole safety
//! property: an instrument that can perturb its subject is not an instrument.
//!
//! No framework, no dependencies, no core pinning. Run at `nice 19` on whatever is free.
//!
//! Every number here already exists in a log the loop writes. Where one did not — per-member
//! `Avg`/`Sample`/`Field(count|sum)`, crossover proposed-vs-survived, the ledger's
//! `identity_string` and top-disagreement FEN — the WRITER was changed to emit it rather than this
//! file deriving it from something adjacent. A derived number drifts from its source silently.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const WATCH: &str = "watch";
const UNTRAINED_ELO: f64 = 954.0;
const P1_MILESTONE: f64 = 2000.0;
const SF_BASE: f64 = 1320.0; // ruler opponent; absolute = SF_BASE + relative

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("watch") => watch(&args[1..]),
        _ => {
            eprintln!("usage: cargo xtask watch [--port N] [--once]");
            std::process::exit(2);
        }
    }
}

fn watch(flags: &[String]) {
    let port = flag(flags, "--port").unwrap_or_else(|| "8799".into());
    let once = flags.iter().any(|f| f == "--once");
    fs::create_dir_all(WATCH).expect("create watch/");

    if !once {
        // Bound to 0.0.0.0 so it reaches the phone over Tailscale. Read-only static files.
        match Command::new("python3")
            .args(["-m", "http.server", &port, "--bind", "0.0.0.0", "--directory", WATCH])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(_) => println!("  serving {WATCH}/ on 0.0.0.0:{port}"),
            Err(e) => eprintln!("  WARN could not start server ({e}); still writing {WATCH}/"),
        }
    }

    loop {
        let s = collect();
        let html = render(&s);
        // temp+rename: a browser polling mid-write must not see a partial page.
        let tmp = format!("{WATCH}/.index.html.tmp");
        if fs::write(&tmp, html).is_ok() {
            let _ = fs::rename(&tmp, format!("{WATCH}/index.html"));
        }
        emit_events(&s);
        if once { break; }
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}

fn flag(flags: &[String], name: &str) -> Option<String> {
    flags.iter().position(|f| f == name).and_then(|i| flags.get(i + 1)).cloned()
}

// ---------------------------------------------------------------- sources

#[derive(Default)]
struct Snap {
    rungs: Vec<Rung>,
    speed: Vec<SpeedPoint>,
    gens: Vec<P2Gen>,
    decisions: Vec<Decision>,
    ledger: Vec<LedgerRow>,
    bounds: (Option<f64>, Option<f64>),
    state_tail: String,
}

struct Rung { generation: u64, elo: f64, ci: f64, label: String, champion: bool, mtime: std::time::SystemTime,
              /// Appearance order in `live_ruler.out`. THE ONLY RECENCY SIGNAL THERE IS: every rung
              /// takes its mtime from that one file, so mtime is identical across runs and cannot
              /// order them. `ruler_status.sh` picks the current run the same way -- "newest run BY
              /// MEASUREMENT ORDER, never alphabetical" -- by taking the last one mentioned.
              ord: usize }
struct SpeedPoint { commit: String, nps: u64, eval_ns: f64, depth_at_budget: u32, interp: Option<f64> }
struct P2Gen {
    generation: u64, lineage: String, pop: usize, spread: (f64, f64),
    tt: Vec<usize>, ttk: Vec<String>, x_prop: usize, x_surv: usize,
    /// Best member rate as a RATIO to the seed (`rates 0.99-1.04x`). The milestone needs it:
    /// the MCTS seed holds Probe and Store by construction, so "holds both" alone is not news.
    rate_hi: f64,
}
struct Decision { generation: u64, lineage: String, verdict: String, llr: Option<f64>, rate: f64, ci: f64, games: u32 }
struct LedgerRow { identity: String, what: String, verdict: String, fen: Option<String> }

fn collect() -> Snap {
    let mut s = Snap::default();
    s.rungs = read_rungs();
    maybe_measure_speed(&read_speed());
    s.speed = read_speed();
    let log = newest(|n| (n.starts_with("gate_") || n == "search_track.log") && n.ends_with(".log"));
    if let Some(p) = &log {
        let txt = fs::read_to_string(p).unwrap_or_default();
        s.gens = parse_gens(&txt);
        s.decisions = parse_decisions(&txt);
    }
    s.bounds = (env_f64("EXISTENCE_GATE_ELO0"), env_f64("EXISTENCE_GATE_ELO1"));
    s.ledger = read_ledger_tail(10);
    s.state_tail = fs::read_to_string("STATE.md").unwrap_or_default()
        .lines().rev().take(3).collect::<Vec<_>>().join(" ");
    s
}

fn env_f64(k: &str) -> Option<f64> { std::env::var(k).ok().and_then(|v| v.parse().ok()) }

fn files_matching(pred: impl Fn(&str) -> bool) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(".").into_iter().flatten().flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).map_or(false, |n| pred(n)))
        .collect();
    v.sort();
    v
}

/// Every ruler reading ever taken. The generation comes from the NET NAME (`gen1400`, `d3`, ...),
/// which is the only place it is recorded — the ruler prints the net it measured, not the
/// generation, so a rung whose name carries no number is listed but cannot be plotted.
fn read_rungs() -> Vec<Rung> {
    let mut out = Vec::new();
    for p in files_matching(|n| n.ends_with("_ruler.log") || n == "sf_ruler.log") {
        let txt = fs::read_to_string(&p).unwrap_or_default();
        let net = grab(&txt, "net      ").unwrap_or_else(|| p.display().to_string());
        // POOL EVERY READING IN THE FILE, do not take the first.
        //
        // This used to be `.find(...)`, i.e. the FIRST "Elo vs this opponent:" line and the rest
        // discarded. At prod4 gen 2052 that first line is +223 -> 1543, the most extreme of EIGHT
        // samples whose pool is 1466 +/- 20 (raw span 1424-1543). The page was therefore publishing
        // an outlier as the headline, and plotting one 120-game sample per rung invents movement:
        // eight measurements of one UNCHANGED net span 119 Elo purely because each is +/-60.
        // CHAMPIONS.md already records the ruler "produced a four-reading DECLINE while the net was
        // genuinely stronger" -- this is that failure at the presentation layer.
        //
        // Inverse-variance pooling; for near-equal sigmas this is the mean with SE = sigma/sqrt(n).
        // The individual samples are NOT lost -- they stay in the ruler logs, which are the ledger.
        let mut obs: Vec<(f64, f64)> = Vec::new();
        for line in txt.lines().filter(|l| l.contains("Elo vs this opponent:")) {
            let rest = line.split(':').nth(1).unwrap_or("").trim();
            let mut it = rest.split("+/-");
            let (Some(e), Some(c)) = (it.next(), it.next()) else { continue };
            let (Ok(v), Ok(s)) = (e.trim().parse::<f64>(), c.trim().parse::<f64>()) else { continue };
            if s > 0.0 { obs.push((v, s)); }
        }
        if obs.is_empty() { continue }
        let wsum: f64 = obs.iter().map(|(_, c)| 1.0 / (c * c)).sum();
        let elo: f64 = obs.iter().map(|(v, c)| v / (c * c)).sum::<f64>() / wsum;
        let ci: f64 = (1.0 / wsum).sqrt();
        let nsamp = obs.len();
        // `gen1400.net` carries its number; `dr_r1_d3.net` does not. For the latter the arm's own
        // log is the only record of how many generations it ran, so count them rather than
        // plotting the rung at 0 and flattening the trend line.
        let mut gnum = gen_from(&net);
        // A rung is a CHAMPION rung only if its own name carries the generation. Arms like
        // `dr_r1_d3.net` get their number recovered from their log below, but they are separate
        // experiments -- putting them on one trend line fits a slope across incomparable runs,
        // which is how this project has manufactured a fake curve before.
        let champion = gnum > 0 || net.contains("champion");
        if gnum == 0 {
            let stem = p.file_name().and_then(|n| n.to_str()).unwrap_or("")
                .trim_end_matches("_ruler.log").to_string();
            if let Ok(t) = fs::read_to_string(format!("{stem}.log")) {
                gnum = t.lines().filter(|l| l.starts_with("gen ")).count() as u64;
            }
        }
        let mtime = fs::metadata(&p).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
        let label = if nsamp > 1 { format!("{} [pooled n={nsamp}]", net.trim()) } else { net.trim().to_string() };
        // Here each FILE is one run, so the file's own mtime genuinely orders runs -- unlike
        // live_ruler.out below, where every rung shares one file's mtime and it cannot.
        // ONE SCALE FOR BOTH SOURCES. seconds*1e5 leaves room for the appearance index the
        // live_ruler.out path adds below. Mixing raw epoch seconds with a 0..N index -- which
        // the first version of this did -- makes every per-file rung outrank every live one and
        // selects a run with too few rungs to fit, which blanked the trend line entirely.
        let ord = mtime.duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as usize * 100_000).unwrap_or(0);
        out.push(Rung { generation: gnum, elo, ci, label, champion, mtime, ord });
    }
    // AND `live_ruler.out`, WHICH IS WHERE THE READINGS ACTUALLY ARE.
    //
    // The loop above reads `*_ruler.log`, one reading per file, generation recovered from the net
    // NAME. But the continuous ruler writes somewhere else entirely: `live_ruler.out`, one line per
    // measurement, with the run and generation INLINE:
    //
    //     03:20 prod4 gen 2052 Elo vs this opponent: +223 +/- 63
    //
    // Checked 2026-09-11: `prod4_ruler.log` holds ZERO readings while `live_ruler.out` holds all
    // eight for gen 2052. So the page was not merely showing an unpooled sample for that rung -- it
    // was not showing that rung at all, and every rung the live ruler has measured since was
    // invisible to it. Pooling the file it does read fixes nothing on its own.
    {
        let txt = fs::read_to_string("live_ruler.out").unwrap_or_default();
        let mut by: std::collections::BTreeMap<(String, u64), Vec<(f64, f64)>> = Default::default();
        let mut order: std::collections::HashMap<(String, u64), usize> = Default::default();
        for line in txt.lines() {
            let Some(pos) = line.find(" Elo vs this opponent:") else { continue };
            let head: Vec<&str> = line[..pos].split_whitespace().collect();
            // [hh:mm, run, "gen", N]
            if head.len() < 4 || head[2] != "gen" { continue }
            // `gen` is a reserved keyword in this edition -- name it gnum.
            let (run, Ok(gnum)) = (head[1].to_string(), head[3].parse::<u64>()) else { continue };
            let rest = line[pos..].split(':').nth(1).unwrap_or("").trim();
            let mut it = rest.split("+/-");
            let (Some(e), Some(c)) = (it.next(), it.next()) else { continue };
            let (Ok(v), Ok(sg)) = (e.trim().parse::<f64>(), c.trim().parse::<f64>()) else { continue };
            if sg <= 0.0 { continue }
            let k = (run, gnum);
            let n = order.len();
            order.entry(k.clone()).or_insert(n);
            by.entry(k).or_default().push((v, sg));
        }
        let mtime = fs::metadata("live_ruler.out").and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
        for ((run, gnum), obs) in by {
            let wsum: f64 = obs.iter().map(|(_, c)| 1.0 / (c * c)).sum();
            let elo: f64 = obs.iter().map(|(v, c)| v / (c * c)).sum::<f64>() / wsum;
            let ci: f64 = (1.0 / wsum).sqrt();
            let n = obs.len();
            let label = if n > 1 { format!("{run} gen{gnum} [pooled n={n}]") } else { format!("{run} gen{gnum}") };
            // Same scale as above: this file's mtime, plus the appearance index so runs inside
            // live_ruler.out order against each other by MEASUREMENT ORDER.
            let base = mtime.duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as usize * 100_000).unwrap_or(0);
            let ord = base + *order.get(&(run.clone(), gnum)).unwrap_or(&0);
            out.push(Rung { generation: gnum, elo, ci, label, champion: run.starts_with("prod"), mtime, ord });
        }
    }

    out.sort_by_key(|r| r.generation);
    out
}

/// First run of digits after "gen", else 0 (unplottable, still listed).
fn gen_from(name: &str) -> u64 {
    let b = name.as_bytes();
    for i in 0..b.len().saturating_sub(3) {
        if &b[i..i + 3] == b"gen" {
            let d: String = name[i + 3..].chars().take_while(|c| c.is_ascii_digit()).collect();
            if !d.is_empty() { return d.parse().unwrap_or(0); }
        }
    }
    0
}

fn grab(txt: &str, key: &str) -> Option<String> {
    txt.lines().find(|l| l.contains(key))
        .and_then(|l| l.split(key).nth(1))
        .map(|v| v.trim().to_string())
}

/// Speed series, one point per commit that changed the engine. Written by this tool into
/// `watch/speed.jsonl` — its OWN file, never one the loop reads.
/// Measure the current build ONCE PER ENGINE COMMIT and append to `watch/speed.jsonl`.
///
/// Keyed on the git hash of the engine-relevant sources, so a docs commit does not add a point and
/// a real change always does. Skips silently when the bench binaries are not built -- an absent
/// point is honest, an invented one is not.
fn maybe_measure_speed(existing: &[SpeedPoint]) {
    let head = Command::new("git")
        .args(["log", "-1", "--format=%h", "--", "crates/engine", "crates/nnue", "crates/pipeline/src/search.rs"])
        .output().ok().and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|v| v.trim().to_string()).unwrap_or_default();
    if head.is_empty() || existing.iter().any(|p| p.commit == head) { return; }

    let target = std::env::var("EXISTENCE_TARGET_DIR")
        .unwrap_or_else(|_| "target".into());
    let bench = format!("{target}/release/examples/search_bench");
    let engine = format!("{target}/release/engine");
    if !Path::new(&bench).exists() || !Path::new(&engine).exists() { return; }

    // nps at champion width, same harness every speed claim here uses.
    let out = Command::new(&bench).args(["4", "16"]).output().ok()
        .and_then(|o| String::from_utf8(o.stdout).ok()).unwrap_or_default();
    let nps = out.split_whitespace().rev().nth(1).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);

    // Depth at the standard budget: the number a speedup MOVES. Constant before the engine read
    // `go` params, which is why every speed result in this repo read 0 Elo.
    let probe = std::process::Command::new(&engine)
        .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped())
        .spawn().ok();
    let mut depth = 0u32;
    if let Some(mut ch) = probe {
        use std::io::Write;
        if let Some(si) = ch.stdin.as_mut() {
            let _ = si.write_all(b"uci\nposition startpos\ngo movetime 200\nquit\n");
        }
        if let Ok(o) = ch.wait_with_output() {
            let t = String::from_utf8_lossy(&o.stdout).to_string();
            depth = after(&t, "info depth ").and_then(|v| v.split_whitespace().next()?.parse().ok()).unwrap_or(0);
        }
    }
    let eval_ns = if nps > 0 { 1e9 / nps as f64 * 0.254 } else { 0.0 };
    use std::io::Write;
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true)
        .open(format!("{WATCH}/speed.jsonl")) {
        let _ = writeln!(f, "{{\"commit\":\"{head}\",\"nps\":{nps},\"eval_ns\":{eval_ns:.1},\"depth\":{depth}}}");
    }
}

fn read_speed() -> Vec<SpeedPoint> {
    let txt = fs::read_to_string(format!("{WATCH}/speed.jsonl")).unwrap_or_default();
    txt.lines().filter(|l| !l.trim().is_empty()).filter_map(|l| {
        Some(SpeedPoint {
            commit: json_str(l, "commit")?,
            nps: json_num(l, "nps")? as u64,
            eval_ns: json_num(l, "eval_ns")?,
            depth_at_budget: json_num(l, "depth")? as u32,
            interp: json_num(l, "interp_ratio"),
        })
    }).collect()
}

fn json_str(line: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\":\"");
    let i = line.find(&pat)? + pat.len();
    let j = line[i..].find('"')? + i;
    Some(line[i..j].to_string())
}

fn json_num(line: &str, key: &str) -> Option<f64> {
    let pat = format!("\"{key}\":");
    let i = line.find(&pat)? + pat.len();
    let rest = &line[i..];
    let j = rest.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == 'e'))
        .unwrap_or(rest.len());
    rest[..j].parse().ok()
}



/// `generation  12 MAIN ... pop 8 spread 0.003-0.004 tt[..] ttk[".."] dsl0 x3/1`
fn parse_gens(txt: &str) -> Vec<P2Gen> {
    let mut out = Vec::new();
    for l in txt.lines().filter(|l| l.trim_start().starts_with("gen ")) {
        let t = l.trim_start();
        let mut w = t.split_whitespace();
        w.next();
        let Some(generation) = w.next().and_then(|g| g.parse::<u64>().ok()) else { continue };
        let lineage = w.next().unwrap_or("?").to_string();
        let pop = after(t, "pop ").and_then(|v| v.split_whitespace().next()?.parse().ok()).unwrap_or(0);
        let spread = after(t, "spread ").map(|v| {
            let f = v.split_whitespace().next().unwrap_or("");
            let mut p = f.split('-');
            (p.next().unwrap_or("0").parse().unwrap_or(0.0), p.next().unwrap_or("0").parse().unwrap_or(0.0))
        }).unwrap_or((0.0, 0.0));
        let tt = bracket(t, "tt[").map(|s| s.split(',')
            .filter_map(|x| x.trim().parse().ok()).collect()).unwrap_or_default();
        let ttk = bracket(t, "ttk[").map(|s| s.split(',')
            .map(|x| x.trim().trim_matches('"').to_string()).collect()).unwrap_or_default();
        let (x_prop, x_surv) = after(t, " x").and_then(|v| {
            let f = v.split_whitespace().next()?;
            let mut p = f.split('/');
            Some((p.next()?.parse().ok()?, p.next()?.parse().ok()?))
        }).unwrap_or((0, 0));
        let rate_hi = after(t, "rates ").and_then(|v| {
            let f = v.split_whitespace().next()?;
            f.trim_end_matches('x').split('-').nth(1)?.parse().ok()
        }).unwrap_or(0.0);
        out.push(P2Gen { generation, lineage, pop, spread, tt, ttk, x_prop, x_surv, rate_hi });
    }
    out
}

fn after<'a>(s: &'a str, key: &str) -> Option<&'a str> { s.split(key).nth(1) }
fn bracket<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    let r = s.split(key).nth(1)?;
    Some(&r[..r.find(']')?])
}

/// `gen  11 MAIN  gate REJECT llr -3.00 0.440+/-0.067 (42 games W-D-L 1-35-6)`
fn parse_decisions(txt: &str) -> Vec<Decision> {
    let mut out = Vec::new();
    for l in txt.lines().filter(|l| l.contains("gate ACCEPT") || l.contains("gate REJECT")) {
        let t = l.trim_start();
        let mut w = t.split_whitespace();
        w.next();
        let Some(gnum) = w.next().and_then(|g| g.parse::<u64>().ok()) else { continue };
        let lineage = w.next().unwrap_or("?").to_string();
        let verdict = if t.contains("ACCEPT") { "ACCEPT" } else { "REJECT" }.to_string();
        let llr = after(t, "llr ").and_then(|v| v.split_whitespace().next()?.parse().ok());
        let (rate, ci) = after(t, "+/-").map(|c| {
            let ci: f64 = c.split_whitespace().next().unwrap_or("0").parse().unwrap_or(0.0);
            let r = t.split("+/-").next().unwrap_or("")
                .split_whitespace().last().unwrap_or("0").parse().unwrap_or(0.0);
            (r, ci)
        }).unwrap_or((0.0, 0.0));
        let games = after(t, "(").and_then(|v| v.split_whitespace().next()?.parse().ok()).unwrap_or(0);
        out.push(Decision { generation: gnum, lineage, verdict, llr, rate, ci, games });
    }
    out
}

fn read_ledger_tail(n: usize) -> Vec<LedgerRow> {
    let Some(p) = newest(|f| f.starts_with("ledger") && f.ends_with(".jsonl")) else { return vec![] };
    let txt = fs::read_to_string(p).unwrap_or_default();
    let lines: Vec<&str> = txt.lines().filter(|l| !l.trim().is_empty()).collect();
    lines.iter().rev().take(n).map(|l| LedgerRow {
        identity: json_str(l, "identity_string").unwrap_or_else(|| "(no identity_string)".into()),
        what: json_str(l, "what").unwrap_or_default(),
        verdict: json_str(l, "verdict").unwrap_or_default(),
        fen: json_str(l, "fen_or_board_hash"),
    }).collect()
}

fn newest(pred: impl Fn(&str) -> bool) -> Option<PathBuf> {
    let mut c: Vec<(std::time::SystemTime, PathBuf)> = files_matching(pred).into_iter()
        .filter_map(|p| fs::metadata(&p).and_then(|m| m.modified()).ok().map(|t| (t, p)))
        .collect();
    c.sort();
    c.pop().map(|(_, p)| p)
}

// ---------------------------------------------------------------- render

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Elo per 100 generations over the plotted rungs, by least squares. `None` when fewer than two
/// rungs carry a generation number — a slope through one point is not a trend.
/// Weighted least squares of absolute Elo on generation — slope in Elo per 1000 generations WITH
/// ITS STANDARD ERROR. Returns `(slope, se)`.
///
/// WHY IT CHANGED, 2026-09-11. This was an UNWEIGHTED OLS slope with no error bar, and
/// `emit_events` fired `RULER_TREND_POSITIVE` on `t > 0.0` — ANY positive number. It announced
/// "+0.4 Elo/100gens" as a milestone while `STATE.md` described the same run as FLAT
/// (`slope +2.2 +/- 2.2 /1000  z +1.00`). Both were right about their own arithmetic: a slope one
/// SE from zero is not a trend, and an event with no significance test cries wolf every day it is
/// run.
///
/// It now mirrors `ruler_trend.py::wls`, which is the INSTRUMENT OF RECORD for this quantity and
/// whose header states the rule plainly: *"A trend is only claimed at |z| > 2."* Three divergences
/// are closed at once — this ignored each rung's `ci` entirely, used no significance test, and
/// reported per 100 generations where the instrument reports per 1000, so the two numbers could not
/// even be compared by eye.
///
/// X IS CENTRED FIRST, and that is not style. `ruler_trend.py` records that the textbook
/// `den = S*Sxx - Sx^2` form cancels away its significant digits once generation numbers reach
/// ~18,000: it produced a negative denominator and a `sqrt` crash, and the same cancellation one
/// digit smaller would have returned a plausible WRONG slope in silence.
///
/// The degenerate-design guard is carried across for the same reason. Repeat samples of one frozen
/// net share a generation, leaving `Sxx` a floating-point crumb rather than an exact zero, which
/// once reported slope +354.7 with an SE of 1.2e17 — a well-formatted row carrying no information.
fn trend(rungs: &[Rung]) -> Option<(f64, f64)> {
    // ONE RUN ONLY. This is the half of the bug that mattered, and adding a significance test
    // WITHOUT it made the page worse rather than better: `champion` is `run.starts_with("prod")`,
    // which is true of eight different training runs, so the fit pooled 151 rungs spanning prod1
    // through prodk1056 at levels from 1302 to 1518. Generation restarts at 1 for every run, so a
    // later run sitting 200 Elo higher contributes a between-run LEVEL difference that the fit can
    // only express as a slope. Pooled and weighted, that read "+4.0 +/- 0.5, z +8.2, RISING" for a
    // champion `ruler_trend.py` calls "+0.8 +/- 0.7, z +1.06, FLAT" -- a confident wrong answer,
    // which is worse than the bare unweighted number it replaced.
    //
    // `ruler_trend.py` never had this problem because it groups by run before fitting. So does this
    // now: take the CURRENT run, by the newest reading and then the furthest generation, and fit
    // only its rungs. Runs are never pooled.
    let run_of = |r: &Rung| r.label.split_whitespace().next().unwrap_or("").to_string();
    // BY APPEARANCE ORDER, NOT BY GENERATION NUMBER. The first version of this used
    // `(mtime, generation)`, and every rung takes its mtime from the same live_ruler.out, so it
    // always fell through to the generation NUMBER -- which picks whichever run ran LONGEST, not the
    // one running now. Measured 2026-09-11: after the 16:56 trainer cycle the page kept reporting
    // prodk1056 (ended at 54,394 generations) while prodk1658 was live at 11,530, and it would have
    // gone on doing so until the new run out-counted the old one. ruler_status.sh has always done
    // this correctly -- "newest run BY MEASUREMENT ORDER" -- by taking the last run named in the file.
    let cur = rungs.iter()
        .filter(|r| r.generation > 0 && r.champion && r.ci > 0.0)
        .max_by_key(|r| r.ord)
        .map(&run_of)?;
    // `ci > 0` is required, not assumed: a zero sigma is an infinite weight and would silently
    // dominate the fit. A rung without a real interval is not evidence about a slope.
    let p: Vec<(f64, f64, f64)> = rungs.iter()
        .filter(|r| r.generation > 0 && r.champion && r.ci > 0.0 && run_of(r) == cur)
        .map(|r| (r.generation as f64, r.elo, r.ci)).collect();
    // Three, not two: two points fit a line exactly and the slope's SE is meaningless.
    if p.len() < 3 { return None; }

    let w: Vec<f64> = p.iter().map(|q| 1.0 / (q.2 * q.2)).collect();
    let sw: f64 = w.iter().sum();
    if !sw.is_finite() || sw <= 0.0 { return None; }
    let xbar: f64 = p.iter().zip(&w).map(|(q, wi)| wi * q.0).sum::<f64>() / sw;
    let ybar: f64 = p.iter().zip(&w).map(|(q, wi)| wi * q.1).sum::<f64>() / sw;
    let sxx: f64 = p.iter().zip(&w).map(|(q, wi)| wi * (q.0 - xbar).powi(2)).sum();
    let sxy: f64 = p.iter().zip(&w).map(|(q, wi)| wi * (q.0 - xbar) * (q.1 - ybar)).sum();

    // Span as max-minus-min rather than last-minus-first: the python takes the ends of a list it
    // assumes is ordered, and `s.rungs` here is not guaranteed to be.
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for q in &p { lo = lo.min(q.0); hi = hi.max(q.0); }
    let span = hi - lo;

    let se = (1.0 / sxx).sqrt() * 1000.0;
    // se_guard: an SE larger than a whole run's worth of Elo means the design, not the data, is
    // doing the talking.
    if sxx <= 0.0 || span <= 0.0 || !se.is_finite() || se > 1000.0 { return None; }
    Some((sxy / sxx * 1000.0, se))
}

/// The verdict the slope supports, on the instrument of record's rule: a trend is claimed only at
/// |z| > 2, i.e. the interval `slope +/- 2*se` must clear zero.
fn trend_verdict(b: f64, se: f64) -> &'static str {
    if b - 2.0 * se > 0.0 { "RISING" } else if b + 2.0 * se < 0.0 { "FALLING" } else { "FLAT" }
}

/// Page chrome. `refresh content=60` matches the regeneration cadence, so the phone stays current
/// without any client-side code.
const HEAD: &str = "<!doctype html><meta charset=utf-8><title>Existence watch</title>\
<meta http-equiv=refresh content=60><meta name=viewport content='width=device-width,initial-scale=1'>\
<style>body{background:#0b0e14;color:#c9d3e0;font:13px/1.5 ui-monospace,monospace;margin:0;padding:10px}\
h2{font-size:13px;color:#58a6ff;margin:18px 0 6px;border-bottom:1px solid #1e2530;padding-bottom:3px}\
table{border-collapse:collapse;width:100%;font-size:12px}td,th{padding:2px 6px;text-align:left;\
border-bottom:1px solid #161b22}th{color:#6b7688;font-weight:400}.big{font-size:20px;color:#3fb950}\
.mut{color:#6b7688}.hot{color:#f85149;font-weight:700}.warm{color:#e3b341}.ok{color:#3fb950}\
code{color:#d2a8ff;word-break:break-all}</style>";

fn render(s: &Snap) -> String {
    let mut h = String::with_capacity(16384);
    h.push_str(HEAD);

    // ---- 1. RULER
    h.push_str("<h2>1 &middot; RULER — absolute Elo vs Stockfish</h2>");
    if s.rungs.is_empty() {
        h.push_str("<p class=mut>no *_ruler.log yet</p>");
    } else {
        // Latest = most recently MEASURED, not highest generation (that reports the longest arm).
        let last = s.rungs.iter().max_by_key(|r| r.mtime).unwrap();
        let abs = SF_BASE + last.elo;
        let _ = write!(h, "<p><span class=big>{abs:.0}</span> absolute \
            <span class=mut>({:+.0} &plusmn; {:.0} vs SF-{})</span>", last.elo, last.ci, SF_BASE as u32);
        match trend(&s.rungs) {
            // The error bar and the verdict travel WITH the slope. A bare "+0.4" reads as progress
            // to anyone glancing at a phone, which is exactly how this page and STATE.md came to
            // describe the same flat run in opposite terms.
            Some((b, se)) => {
                let v = trend_verdict(b, se);
                let cls = if v == "RISING" { "ok" } else { "mut" };
                let _ = write!(h, " &nbsp; trend <b>{b:+.1}</b> &plusmn; {se:.1} Elo / 1000 gens \
                    <span class={cls}>({v}, z {:+.1})</span>", b / se);
            }
            None => h.push_str(" &nbsp; <span class=mut>trend: needs 3+ CHAMPION rungs with real spread</span>"),
        }
        h.push_str("</p>");
        h.push_str(&svg_ruler(&s.rungs));
        h.push_str("<table><tr><th>generation<th>net<th>Elo<th>&plusmn;CI<th>absolute</tr>");
        for r in &s.rungs {
            let _ = write!(h, "<tr><td>{}<td class=mut>{}<td class=mut>{}<td>{:+.0}<td class=mut>{:.0}<td>{:.0}</tr>",
                if r.generation > 0 { r.generation.to_string() } else { "—".into() },
                esc(&r.label), if r.champion { "champion" } else { "arm" },
                r.elo, r.ci, SF_BASE + r.elo);
        }
        h.push_str("</table>");
    }

    // ---- 2. SPEED
    h.push_str("<h2>2 &middot; SPEED — one point per engine commit</h2>");
    if s.speed.is_empty() {
        h.push_str("<p class=mut>watch/speed.jsonl is empty — no engine commit measured yet.</p>");
    } else {
        h.push_str("<table><tr><th>commit<th>nps<th>eval ns/node<th>depth @ budget<th>bench-interp</tr>");
        for p in &s.speed {
            let iv = p.interp.map_or("—".into(), |r| format!("{r:.3}x"));
            let _ = write!(h, "<tr><td class=mut>{}<td>{}<td>{:.0}<td>{}<td>{iv}</tr>",
                esc(&p.commit), p.nps, p.eval_ns, p.depth_at_budget);
        }
        h.push_str("</table>");
    }

    // ---- 3. P2 SEARCH TRACK
    h.push_str("<h2>3 &middot; P2 SEARCH TRACK</h2>");
    if s.gens.is_empty() {
        h.push_str("<p class=mut>no generation lines in the newest track log</p>");
    } else {
        h.push_str("<table><tr><th>generation<th>lineage<th>pop<th>rate spread<th>tt/member\
<th>Avg/Sample/Field<th>cross prop/surv</tr>");
        for g in s.gens.iter().rev().take(40) {
            let hash_reuse = g.ttk.iter().any(|k| k.contains('P') && k.contains('S'));
            let mcts_in_main = g.lineage == "MAIN"
                && g.ttk.iter().any(|k| k.contains('A') || k.contains('c'));
            let cls = if hash_reuse { " class=hot" } else if mcts_in_main { " class=warm" } else { "" };
            let _ = write!(h, "<tr{cls}><td>{}<td>{}<td>{}<td class=mut>{:.6}–{:.6}<td>{:?}<td>{}<td>{}/{}</tr>",
                g.generation, esc(&g.lineage), g.pop, g.spread.0, g.spread.1, g.tt,
                esc(&g.ttk.join(" ")), g.x_prop, g.x_surv);
        }
        h.push_str("</table><p class=mut>red = a member holds Probe AND Store (hash reuse assembled). \
amber = a MAIN member holds Avg or Field(count) (MCTS material in an alpha-beta program).</p>");
    }

    // ---- 4. GATE
    h.push_str("<h2>4 &middot; GATE</h2>");
    let n = s.decisions.len();
    let acc = s.decisions.iter().filter(|d| d.verdict == "ACCEPT").count();
    let _ = write!(h, "<p>accept rate <b>{}</b> / {n} &nbsp; bounds e0={} e1={}</p>", acc,
        s.bounds.0.map_or("—".into(), |v| format!("{v}")),
        s.bounds.1.map_or("—".into(), |v| format!("{v}")));
    if !s.decisions.is_empty() {
        h.push_str("<table><tr><th>generation<th>lineage<th>verdict<th>LLR<th>rate<th>&plusmn;<th>games</tr>");
        for d in s.decisions.iter().rev().take(20) {
            let cls = if d.verdict == "ACCEPT" { " class=ok" } else { "" };
            let _ = write!(h, "<tr{cls}><td>{}<td>{}<td>{}<td>{}<td>{:.3}<td class=mut>{:.3}<td>{}</tr>",
                d.generation, esc(&d.lineage), d.verdict,
                d.llr.map_or("—".into(), |v| format!("{v:+.2}")), d.rate, d.ci, d.games);
        }
        h.push_str("</table>");
    }

    // ---- 5. LEDGER TAIL
    h.push_str("<h2>5 &middot; LEDGER — last 10, as the engine writes them</h2>");
    for r in &s.ledger {
        let _ = write!(h, "<p><b>{}</b><br><span class=mut>{} — {}</span>{}</p>",
            esc(&r.identity), esc(&r.verdict), esc(&r.what),
            r.fen.as_ref().map_or(String::new(),
                |f| format!("<br><code>{}</code>", esc(f))));
    }
    let _ = write!(h, "<p class=mut>{}</p>", esc(&s.state_tail));
    h
}

/// Inline SVG scatter with error bars. No library: five panels do not justify a dependency.
fn svg_ruler(rungs: &[Rung]) -> String {
    let pts: Vec<&Rung> = rungs.iter().filter(|r| r.generation > 0 && r.champion).collect();
    if pts.is_empty() { return String::new(); }
    let (w, hh) = (720.0, 200.0);
    let gmax = pts.iter().map(|r| r.generation).max().unwrap_or(1).max(1) as f64;
    let lo = UNTRAINED_ELO.min(pts.iter().map(|r| SF_BASE + r.elo - r.ci).fold(f64::MAX, f64::min)) - 40.0;
    let hi = P1_MILESTONE.max(pts.iter().map(|r| SF_BASE + r.elo + r.ci).fold(f64::MIN, f64::max)) + 40.0;
    let x = |g: u64| 40.0 + (g as f64 / gmax) * (w - 60.0);
    let y = |e: f64| hh - 20.0 - (e - lo) / (hi - lo) * (hh - 40.0);
    let mut s = format!("<svg width='100%' viewBox='0 0 {w} {hh}' style='background:#0f141c'>");
    for (v, c, lbl) in [(UNTRAINED_ELO, "#6b7688", "untrained ~954"), (P1_MILESTONE, "#e3b341", "P1 2000")] {
        let _ = write!(s, "<line x1=40 x2={} y1={:.1} y2={:.1} stroke='{c}' stroke-dasharray='3,3'/>\
<text x=44 y={:.1} fill='{c}' font-size=9>{lbl}</text>", w - 20.0, y(v), y(v), y(v) - 3.0);
    }
    for r in &pts {
        let (px, e) = (x(r.generation), SF_BASE + r.elo);
        let _ = write!(s, "<line x1={px:.1} x2={px:.1} y1={:.1} y2={:.1} stroke='#3fb950' stroke-width=1/>\
<circle cx={px:.1} cy={:.1} r=3 fill='#3fb950'/>", y(e - r.ci), y(e + r.ci), y(e));
    }
    s.push_str("</svg>");
    s
}

// ---------------------------------------------------------------- events

/// The milestone list is FIXED. Nothing is added to it without saying so.
fn emit_events(s: &Snap) {
    let path = format!("{WATCH}/events.log");
    let seen: BTreeSet<String> = fs::read_to_string(&path).unwrap_or_default()
        .lines().filter_map(|l| l.split_whitespace().nth(2).map(str::to_string)).collect();
    let mut new: Vec<(String, String)> = Vec::new();
    let idx = s.ledger.len();

    for r in &s.rungs {
        // KEY ON THE MEASUREMENT, NOT ON WHEN IT WAS READ. For a live arm the generation is counted
        // from the trainer's log AT POLL TIME, so an unchanged ruler reading got a new key every 60
        // seconds and was re-emitted as a fresh milestone -- gen 2303 and 2339 both logged 1361,
        // gen 2519 and 2550 both logged 1285. The reading's own mtime identifies it uniquely.
        let stamp = r.mtime.duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs()).unwrap_or(0);
        let key = format!("RULER_RUNG:{}:{:.0}", stamp, r.elo);
        new.push((key, format!("RULER_RUNG            {} {:.0} {:.0}", r.generation, SF_BASE + r.elo, r.ci)));
        if SF_BASE + r.elo >= P1_MILESTONE {
            new.push((format!("P1:{}", r.generation), format!("P1_MILESTONE          gen {} at {:.0}", r.generation, SF_BASE + r.elo)));
        }
    }
    // SIGNIFICANCE, not sign. This fired on `t > 0.0`, so a slope one SE from zero was published as
    // a milestone while STATE.md called the same run FLAT. The rule is the instrument of record's:
    // claim a trend only at |z| > 2, i.e. slope - 2*se > 0. The message now carries the SE and z, so
    // the number cannot be quoted on its own.
    if let Some((b, se)) = trend(&s.rungs) {
        let n = s.rungs.iter().filter(|r| r.generation > 0 && r.champion).count();
        if n >= 3 && b - 2.0 * se > 0.0 {
            new.push((format!("TREND:{n}"),
                format!("RULER_TREND_POSITIVE  slope {b:+.1} +/- {se:.1} Elo/1000gens (z {:+.1}) over {n} rungs",
                        b / se)));
        }
    }
    for wnd in s.speed.windows(2) {
        if wnd[1].depth_at_budget == wnd[0].depth_at_budget + 1 {
            new.push((format!("DEPTH:{}", wnd[1].commit),
               format!("DEPTH_PLUS_ONE        {} -> depth {}", wnd[1].commit, wnd[1].depth_at_budget)));
        }
    }
    for g in &s.gens {
        // "FIRST ... graft passing the mate guard" -- keyed once, not once per generation.
        if g.lineage == "MAIN" && g.x_surv > 0 {
            new.push(("XSURV".into(),
               format!("CROSSOVER_SURVIVED    gen {} MAIN {}/{} survived the mate guard", g.generation, g.x_surv, g.x_prop)));
        }
    }
    let main: Vec<&P2Gen> = s.gens.iter().filter(|g| g.lineage == "MAIN").collect();
    // HASH REUSE must be ACQUIRED, not inherited, and MAIN is the only lineage where that means
    // anything. The first version fired on MCTS at gen 1 because the UCT seed ALREADY carries
    // probe+store by construction -- it reported the seed as a discovery. The condition is now a
    // false->true TRANSITION in MAIN: the previous generation's population held no member with
    // both halves and this one does, which is a parent that lacked one. `rate_hi >= 1.0` keeps the
    // original requirement that the assembling member is not worse than the seed.
    let has_pair = |g: &P2Gen| g.ttk.iter().any(|k| k.contains('P') && k.contains('S'));
    for w in main.windows(2) {
        if !has_pair(w[0]) && has_pair(w[1]) && w[1].rate_hi >= 1.0 {
            new.push(("HASH".into(), format!(
               "HASH_REUSE_ASSEMBLED  gen {} MAIN acquired probe+store (gen {} had none) ttk {} rate {:.3}x",
               w[1].generation, w[0].generation, w[1].ttk.join(" "), w[1].rate_hi)));
        }
    }
    // Must PERSIST: 5 consecutive MAIN gens, so one lucky candidate is not called a discovery.
    let mut run = 0usize;
    for g in &main {
        if g.ttk.iter().any(|k| k.contains('A') || k.contains('c')) { run += 1 } else { run = 0 }
        if run >= 5 {
            new.push(("MCTSMAT".into(),
               format!("MCTS_MATERIAL_IN_MAIN gen {} held Avg/Field(count) for {run} generations", g.generation)));
        }
    }
    for d in s.decisions.iter().filter(|d| d.verdict == "ACCEPT") {
        new.push((format!("ACC:{}:{}", d.lineage, d.generation),
           format!("GATE_ACCEPT_PROGRAM   gen {} {} rate {:.3}", d.generation, d.lineage, d.rate)));
    }
    for r in s.ledger.iter().filter(|r| r.verdict == "accept") {
        // ⚠ MEASURED INERT 2026-09-11, and kept with the measurement so the next reader does not
        // re-derive it. This filters a ledger row's `what` for program-kind names. NO LEDGER ROW
        // HAS EVER CONTAINED ONE: across all 37 ledger*.jsonl on disk -- 178,000+ rows -- the count
        // whose `what` mentions Avg/Probe/Store/Field/Sample is ZERO. `what` holds net-track
        // descriptions ("train 3 epochs on 233 samples, horizon 160, width 16"), because `ledger()`
        // reads the NEWEST ledger*.jsonl and that is the P1 trainer's. The event cannot fire, so
        // its silence has never meant anything and must not be read as "no hybrid was accepted".
        if ["Avg", "Field(count)", "Sample"].iter().any(|k| r.what.contains(k)) {
            new.push((format!("HYB:{}", r.identity), format!("HYBRID_ACCEPTED       {}", r.identity)));
        }
    }

    // ---- THE TWO UNC EVENTS -------------------------------------------------------------------
    // Keyed on the `ttk` tag's U, which `evolve ttk` positively controls: it asserts U is visible
    // on the uncertainty yardstick, does not leak into the P/S slots, and does not inflate the
    // pooled TT count. Deliberately NOT keyed on the ledger -- copying the pattern directly above
    // would have produced a second event that can never fire.
    //
    // MUST PERSIST, the same rule MCTS_MATERIAL_IN_MAIN uses: five consecutive MAIN generations, so
    // one lucky candidate carrying an unc read is not announced as a discovery. `unc` is reachable
    // in a SINGLE edit (ProbeRead's sixth source), so transient members carrying it are expected
    // and are not news.
    let mut urun = 0usize;
    for g in &main {
        if g.ttk.iter().any(|k| k.contains('U')) { urun += 1 } else { urun = 0 }
        if urun >= 5 {
            new.push(("UNCREAD".into(), format!(
                "UNC_READ_IN_MAIN      gen {} held an unc read for {urun} generations  ttk {}",
                g.generation, g.ttk.join(" "))));
        }
    }

    // AN ACCEPTED MAIN PROGRAM IN A GENERATION WHOSE POPULATION CARRIES THE READ.
    //
    // THIS IS AN APPROXIMATION AND THE WORDING MUST NOT OUTRUN IT. The available data cannot tie an
    // ACCEPT to the STRUCTURE of the program that won: a decision carries generation, lineage and
    // verdict, while `ttk` describes the POPULATION. So this fires when both hold in the same MAIN
    // generation, which is necessary and not sufficient -- the accepted member may carry no unc
    // read at all. The line says so itself, because a reader seeing "UNC_GATED_EXTENSION" in a log
    // six weeks from now will not come and check what it meant.
    for d in s.decisions.iter().filter(|d| d.verdict == "ACCEPT" && d.lineage == "MAIN") {
        if let Some(g) = main.iter().find(|g| g.generation == d.generation) {
            if g.ttk.iter().any(|k| k.contains('U')) {
                new.push((format!("UNCACC:{}", d.generation), format!(
                    "UNC_GATED_EXTENSION   gen {} MAIN ACCEPT, unc read present in the population \
                     (rate {:.3}) ttk {} -- POPULATION-level, not proof the accepted program reads it",
                    d.generation, d.rate, g.ttk.join(" "))));
            }
        }
    }
    if Path::new("WEEK_STOP").exists() {
        new.push(("WEEK".into(), "WEEK_STOP             day-7 stop condition fired".into()));
    }

    let mut add = String::new();
    for (key, line) in new {
        if seen.contains(&key) { continue; }
        let stamp = Command::new("date").arg("+%Y-%m-%d %H:%M:%S").output().ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string()).unwrap_or_default();
        let full = format!("{stamp} {key} {line} [ledger:{idx}]");
        println!("{full}");
        add.push_str(&full);
        add.push('\n');
    }
    if !add.is_empty() {
        use std::io::Write;
        if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&path) {
            let _ = f.write_all(add.as_bytes());
        }
    }
}
