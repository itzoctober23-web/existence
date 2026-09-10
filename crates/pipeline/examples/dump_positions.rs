//! CONTROL 2, part 0: dump the positions the LOOP ITSELF trains on.
//!
//! The control asks whether the trainer works when given good labels. That requires holding the
//! POSITIONS fixed and varying only the LABEL -- otherwise "SF labels are better" and "SF positions
//! are better" are the same experiment and neither is answered.
//!
//! So the positions must come from `datagen::play_games` with the champion net at the loop's own
//! datagen depth, not from a random walk. A first attempt used random legal walks and the labels
//! came back at a mover-relative mean of +774 to +1098 centipawns -- random play leaves whoever is
//! to move winning by ~9 pawns, which saturates `tanh(root/600)` and would have trained against a
//! near-constant target. That is a property of the SOURCE, not of the labels, and it is exactly the
//! confound this dump removes.
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
