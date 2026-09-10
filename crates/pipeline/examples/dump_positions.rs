//! CONTROL 2, part 0: dump the positions the LOOP ITSELF trains on.
//!
//! The control asks whether the trainer works when given good labels. That requires holding the
//! POSITIONS fixed and varying only the LABEL -- otherwise "SF labels are better" and "SF positions
//! are better" are the same experiment and neither is answered.
//!
//! So the positions must come from `datagen::play_games` with the champion net at the loop's own
//! datagen depth, not from a random walk.
//!
//! ⚠ THE ORIGINAL REASON GIVEN HERE WAS WRONG, and is corrected rather than deleted. I first wrote
//! that random legal walks were rejected because their labels came back at a mover-relative mean of
//! +774 to +1098 centipawns -- "random play leaves whoever is to move winning by ~9 pawns, which
//! saturates `tanh(root/600)`". That was an explanation, not a measurement. Re-measured on the
//! actual smoke file: the raw mean was +933, of which **+750 came from five `mate_score=30000`
//! entries out of 200 rows**. The non-mate median was **+221cp** -- entirely trainable, nowhere near
//! saturation. The same artifact appears in the loop's own positions (+1034 raw, +22 non-mate
//! median), so the raw mean never distinguished the two sources at all.
//!
//! The real reason to use the loop's own positions is the one the control needs anyway: the arms
//! must differ in the LABEL and nothing else, so the position distribution has to be the loop's.
//! That reason is independent of any centipawn statistic.
//!
//! Writes one FEN per line, plus the loop's own `root` and `z` alongside, so the SF-labelled arm and
//! the self-play arm can be built from the SAME rows and differ in one column.

use nnue::Net;
use pipeline::datagen;

fn arg<T: std::str::FromStr>(name: &str, d: T) -> T {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(d)
}

fn main() {
    let net_path: String = std::env::args().nth(1).unwrap_or_else(|| "p1_champion.net".into());
    let games: usize = arg("--games", 400);
    let depth: u32 = arg("--depth", 1);
    let seed: u64 = arg("--seed", 20260910);
    let out: String = {
        let a: Vec<String> = std::env::args().collect();
        a.iter().position(|x| x == "--out").and_then(|i| a.get(i + 1)).cloned()
            .unwrap_or_else(|| "positions.tsv".into())
    };

    let net = match Net::load(&net_path) {
        Ok(n) => n,
        Err(e) => { eprintln!("cannot load {net_path}: {e}"); std::process::exit(2); }
    };
    eprintln!("dump_positions: {net_path} (w{}), {games} games at depth {depth}, seed {seed}",
              net.n_hidden);

    // Same call the loop makes, so the position distribution is the loop's.
    let (samples, decided) = datagen::play_games(&net, depth, seed, games, 4, 160, 2);
    eprintln!("  {} samples, {decided} decided games", samples.len());

    let mut s = String::new();
    for sm in &samples {
        s.push_str(&format!("{}\t{}\t{}\t{}\n", sm.fen, sm.root, sm.z, sm.plies_to_end));
    }
    match std::fs::write(&out, s) {
        Ok(()) => println!("wrote {} rows to {out}  (fen, loop_root_cp, z, plies_to_end)", samples.len()),
        Err(e) => { eprintln!("write failed: {e}"); std::process::exit(2); }
    }
}
