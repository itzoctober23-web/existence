//! CONTROL: can the trainer learn a target it MUST be able to learn?
//!
//! Three hypotheses about why self-play training fails (capacity, self-referential blend,
//! label sparsity) have all come back null. The remaining suspect is the trainer itself.
//!
//! Target = material balance, computed from the SAME input planes the net sees, so it is
//! exactly a linear function of the input. If the trainer cannot fit this, the trainer is
//! broken and every experiment above was measuring a broken optimiser.
//!
//! NOTE this target is a control fixture only. It never touches the engine: putting piece
//! values into training would be exactly the human knowledge the project forbids.
use board::{Color, PieceKind, Position, bitboard as bb};
use nnue::Net;
use pipeline::datagen::{self, Rng, Sample};
use pipeline::trainer::Trainer;

fn material(pos: &Position) -> f32 {
    const V: [f32; 6] = [1.0, 3.0, 3.0, 5.0, 9.0, 0.0];
    let mut m = 0.0;
    for k in PieceKind::ALL {
        m += V[k.idx()] * bb::count(pos.pieces[0][k.idx()]) as f32;
        m -= V[k.idx()] * bb::count(pos.pieces[1][k.idx()]) as f32;
    }
    (m / 10.0).tanh()
}

fn corr(net: &Net, data: &[Sample]) -> f64 {
    let mut s = Vec::new();
    let (mut xs, mut ys) = (Vec::new(), Vec::new());
    for d in data {
        let p = match Position::from_fen(&d.fen) { Ok(p) => p, Err(_) => continue };
        xs.push(net.eval(&p, &mut s) as f64);
        let mv = if p.stm == Color::White { material(&p) } else { -material(&p) };
        ys.push(mv as f64);
    }
    let n = xs.len() as f64;
    let (mx, my) = (xs.iter().sum::<f64>() / n, ys.iter().sum::<f64>() / n);
    let (mut num, mut dx, mut dy) = (0.0, 0.0, 0.0);
    for i in 0..xs.len() {
        let (a, b) = (xs[i] - mx, ys[i] - my);
        num += a * b; dx += a * a; dy += b * b;
    }
    if dx <= 0.0 || dy <= 0.0 { return 0.0; }
    num / (dx.sqrt() * dy.sqrt())
}

fn main() {
    let gen_net = Net::random(128, 20260907);
    let mut rng = Rng(3);
    let mut raw: Vec<Sample> = Vec::new();
    for _ in 0..150 { datagen::play_game(&gen_net, 2, &mut rng, 6, 160, &mut raw); }

    // relabel every position with the material target (white POV, as datagen's z is)
    let data: Vec<Sample> = raw.iter().filter_map(|s| {
        let p = Position::from_fen(&s.fen).ok()?;
        Some(Sample { fen: s.fen.clone(), z: material(&p), root: 0 })
    }).collect();
    let split = data.len() * 3 / 4;
    let (train, held) = data.split_at(split);
    println!("  {} train / {} held-out positions, target = material (linear in the inputs)",
        train.len(), held.len());

    let tr = Trainer::new(0.02, 0.0);
    let mut net = Net::random(32, 4242);
    println!("  epoch   train-loss   held-out corr(eval, material)");
    println!("  {:>5}   {:>10}   {:+.4}   <- untrained", 0, "-", corr(&net, held));
    for e in 1..=20 {
        let l = tr.epoch(&mut net, train, 55 + e as u64);
        if e % 4 == 0 || e == 1 {
            println!("  {:>5}   {:>10.5}   {:+.4}", e, l, corr(&net, held));
        }
    }
    let c = corr(&net, held);
    println!("\n  => {}", if c > 0.8 {
        "TRAINER WORKS: it fits a learnable target. The failure is in the SIGNAL, not the optimiser."
    } else {
        "TRAINER IS BROKEN: it cannot fit a target that is linear in its own inputs."
    });
}
