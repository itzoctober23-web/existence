//! Round-robin a set of nets and rate each against the FIELD, not against one opponent.
//!
//! WHY THIS EXISTS. Non-transitivity here is measured, not feared: at a matched depth 2,
//!     ep_1 vs champion_long   direct        0.548 +/- 0.021   ep_1 stronger
//!     ep_1 / champion_long    via origin   -0.024 +/- 0.023   champion_long stronger
//! The two instruments disagree in SIGN on the same pair at the same depth. For the blend pair they
//! AGREE. So which single-opponent measurement to trust is pair-dependent, and "A beat B" is not a
//! basis for moving a shipped default.
//!
//! A round robin fixes the specific defect: a matchup edge against ONE opponent averages out over a
//! field, while genuine strength does not. This is what engine testing does as a matter of course
//! and what this project has been missing -- every comparison so far has been A-vs-B or A-vs-origin.
//!
//! TWO OUTPUTS, and the second is the point:
//!   * a rating per net -- mean pentanomial score against every other net in the pool;
//!   * a CYCLE COUNT -- how many triples have A>B>C>A. Cycles are non-transitivity made countable.
//!     A pool with many cycles has no consistent ordering, and any ranking printed from it, including
//!     this one, is partly an artefact of which opponents happened to be in the pool.
//!
//! The cycle count is reported whether it is convenient or not: if it is high, this tool's own
//! ranking is the thing it undermines.
//!
//! Depth is an argument and is printed, because this project judges strength at depth 4 while its
//! datagen runs at depth 2, and a rating computed at the wrong depth is a confident wrong answer.
use nnue::Net;
use pipeline::gate;

fn load(spec: &str) -> Net {
    if let Some(rest) = spec.strip_prefix("random:") {
        let mut it = rest.split(':');
        let w: usize = it.next().and_then(|x| x.parse().ok()).expect("random:<width>:<seed>");
        let sd: u64 = it.next().and_then(|x| x.parse().ok()).expect("random:<width>:<seed>");
        Net::random(w, sd)
    } else {
        Net::load(spec).unwrap_or_else(|e| panic!("{spec}: {e}"))
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 4 {
        eprintln!("usage: pool_rating <pairs> <depth> <seed> <net> <net> [net...]");
        std::process::exit(2);
    }
    // SHARDING. "s<i>/<n>" as the first argument runs only the matches with index % n == i, so a
    // pool can be spread across cores instead of played sequentially on one. Depth-4 matches cost
    // ~25x depth-2, and a 28-match pool takes ~3 hours on a single core; depth 4 is now the
    // standard, so every future pool pays that unless it can fan out.
    //
    // The MATCH LINES are the shareable artefact -- each shard prints "A vs B rate" and a merge
    // reads them back. Ratings are deliberately NOT computed per shard: a shard sees only part of
    // the field, and a rating over part of a field is exactly the single-opponent number this tool
    // exists to replace.
    let mut args = args;
    let mut shard = (0usize, 1usize);
    if args[0].starts_with('s') && args[0].contains('/') {
        let spec = args.remove(0);
        let (i, n) = spec[1..].split_once('/').expect("s<i>/<n>");
        shard = (i.parse().expect("shard index"), n.parse().expect("shard count"));
        assert!(shard.1 > 0 && shard.0 < shard.1, "shard index must be < count");
    }
    let pairs: usize = args[0].parse().expect("pairs");
    let depth: u32 = args[1].parse().expect("depth");
    let seed: u64 = args[2].parse().expect("seed");
    let names: Vec<String> = args[3..].to_vec();
    let nets: Vec<Net> = names.iter().map(|s| load(s)).collect();
    let n = nets.len();
    assert!(n >= 2, "need at least two nets");

    let std_note = if depth == 4 { "project standard for strength" } else { "NOT the strength standard (4)" };
    println!("pool_rating: {n} nets, {pairs} pairs/match, depth {depth} ({std_note}), seed {seed}");
    let total = n * (n - 1) / 2;
    if shard.1 > 1 {
        println!("  SHARD {}/{}: {} of {total} matches\n", shard.0, shard.1,
                 (0..total).filter(|k| k % shard.1 == shard.0).count());
    } else {
        println!("  {total} matches to play\n");
    }

    // score[i][j] = i's pentanomial rate against j. Filled symmetrically so the mean is over the
    // whole field rather than over "the ones that happened to be listed after me".
    let mut score = vec![vec![f64::NAN; n]; n];
    let mut midx = 0usize;
    for i in 0..n {
        for j in (i + 1)..n {
            let mine = midx % shard.1 == shard.0;
            midx += 1;
            if !mine { continue; }
            // Seed varies per PAIRING so different matchups do not all reuse one opening set; a
            // single shared set would let one lucky opening family bias every rating at once.
            let s = seed ^ ((i as u64) << 32) ^ (j as u64);
            let r = gate::match_nets(&nets[i], &nets[j], depth, pairs, s).pent_rate();
            score[i][j] = r;
            score[j][i] = 1.0 - r;
            println!("  {:>18} vs {:<18} {:.3}", names[i], names[j], r);
        }
    }

    if shard.1 > 1 {
        println!("\n  SHARD DONE. No rating printed: this shard saw only part of the field, and a");
        println!("  rating over part of a field is the single-opponent number this tool replaces.");
        println!("  Merge the match lines from all {} shards, then rate.", shard.1);
        return;
    }
    let mut rated: Vec<(f64, usize)> = (0..n).map(|i| {
        let v: Vec<f64> = (0..n).filter(|&j| j != i).map(|j| score[i][j]).collect();
        (v.iter().sum::<f64>() / v.len() as f64, i)
    }).collect();
    rated.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

    println!("\n  === RATING: mean score against the field ===");
    for (r, i) in &rated {
        println!("  {:>18}  {:.4}", names[*i], r);
    }

    // Cycles: A beats B beats C beats A. Counted over unordered triples.
    let beats = |a: usize, b: usize| score[a][b] > 0.5;
    let (mut cycles, mut triples) = (0usize, 0usize);
    for a in 0..n { for b in (a + 1)..n { for c in (b + 1)..n {
        triples += 1;
        let f = (beats(a, b), beats(b, c), beats(c, a));
        if f == (true, true, true) || f == (false, false, false) { cycles += 1; }
    }}}
    println!("\n  === NON-TRANSITIVITY: {cycles} cyclic triples out of {triples} ===");
    if cycles > 0 {
        println!("  A cycle means the pool has no consistent ordering, so the ranking above is");
        println!("  partly an artefact of WHICH nets are in the pool. Read it as a summary, not a");
        println!("  truth, and do not move a shipped default on a gap smaller than the cycling.");
    } else {
        println!("  No cycles: the pool orders consistently and the ranking is safe to read as one.");
    }
}
