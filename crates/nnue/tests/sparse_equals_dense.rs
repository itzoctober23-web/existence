//! The sparse gather must agree with the dense sweep EXACTLY. The sparse path is a 48x
//! speedup; a silent disagreement would make every measurement taken after it worthless.
use board::Position;
use nnue::{N_INPUTS, Net};

/// Dense reference: the arithmetic the sparse path replaced.
fn dense(net: &Net, pos: &Position) -> f32 {
    let mut x = vec![0.0f32; N_INPUTS];
    Net::features_dense(pos, &mut x);
    let mut acc = 0.0f32;
    for h in 0..net.n_hidden {
        let mut s = net.b1[h];
        for (i, xi) in x.iter().enumerate() {
            if *xi != 0.0 {
                s += net.w1[i * net.n_hidden + h] * xi;
            }
        }
        if s > 0.0 {
            acc += s * net.w2[h];
        }
    }
    acc + net.b2
}

#[test]
fn sparse_matches_dense_over_random_games() {
    let net = Net::random(64, 0xABCDEF);
    let mut scratch = Vec::new();
    let mut rng: u64 = 0x1234_5678_9ABC_DEF0;
    let mut checked = 0;

    for _ in 0..40 {
        let mut pos = Position::startpos();
        for _ in 0..60 {
            let sparse = net.eval(&pos, &mut scratch) as f32;
            let d = dense(&net, &pos);
            let white_pov = d * net.scale;
            let expect = if pos.stm == board::Color::White { white_pov } else { -white_pov };
            let expect = expect.clamp(-30_000.0, 30_000.0) as i32 as f32;
            assert!(
                (sparse - expect).abs() <= 1.0,
                "sparse {sparse} vs dense {expect} at fen {}",
                pos.to_fen()
            );
            checked += 1;

            let list = pos.legal_moves();
            if list.is_empty() { break; }
            rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
            let m = list.as_slice()[(rng % list.len() as u64) as usize];
            pos.make_move(m);
        }
    }
    assert!(checked > 1000, "only {checked} positions checked");
    println!("sparse == dense on {checked} positions");
}
