//! Train the net on self-play outcomes. Hand-written backprop for the 782 -> H -> 1 net.
//!
//! Target = the game outcome from the MOVER's point of view, blended with the engine's own
//! root search score. Both are self-referential (MASTER_PLAN Given: "game outcome; agreement
//! with own deeper search"). Nothing human enters.

use board::{Color, Position};
use nnue::Net;

use crate::datagen::Sample;

pub struct Trainer {
    pub lr: f32,
    /// Weight on the engine's own search score vs the raw game outcome. The outcome is a very
    /// noisy label for an early position; the root score is lower-variance and self-derived.
    pub blend: f32,
}

impl Trainer {
    pub fn new(lr: f32, blend: f32) -> Self {
        Trainer { lr, blend }
    }

    /// One epoch of SGD. Returns mean squared error before the update.
    pub fn epoch(&self, net: &mut Net, data: &[Sample], rng_seed: u64) -> f32 {
        let mut order: Vec<usize> = (0..data.len()).collect();
        let mut r = rng_seed | 1;
        for i in (1..order.len()).rev() {
            r ^= r << 13; r ^= r >> 7; r ^= r << 17;
            order.swap(i, (r % (i as u64 + 1)) as usize);
        }

        let h = net.n_hidden;
        let mut idx: Vec<u16> = Vec::with_capacity(40);
        let mut acc = vec![0f32; h];
        let mut total = 0f32;

        for &i in &order {
            let s = &data[i];
            let pos = match Position::from_fen(&s.fen) {
                Ok(p) => p,
                Err(_) => continue,
            };
            Net::active(&pos, &mut idx);

            // forward
            acc.copy_from_slice(&net.b1);
            for &f in &idx {
                let row = &net.w1[f as usize * h..(f as usize + 1) * h];
                for (a, w) in acc.iter_mut().zip(row) { *a += *w; }
            }
            let mut out = net.b2;
            for k in 0..h {
                if acc[k] > 0.0 { out += acc[k] * net.w2[k]; }
            }
            let pred = out.tanh();

            // FRAME: the network's raw output is WHITE-POV -- Net::eval computes a white-POV
            // value and applies the mover flip only at the very end. So the target here must be
            // white-POV too. Training this output toward a MOVER-relative target asks the same
            // output to be +m and -m for the same material, which is contradictory, and the
            // cheapest solution is to predict the mean. That bug made the trainer unable to fit
            // even material -- a target linear in its own inputs (examples/trainer_control.rs).
            let z_white = s.z;
            // s.root is mover-relative (it comes from the search), so bring it to white-POV.
            let root_white = if pos.stm == Color::White { s.root } else { -s.root };
            let root = (root_white as f32 / net.scale).tanh();
            let target = (1.0 - self.blend) * z_white + self.blend * root;

            let err = pred - target;
            total += err * err;

            // backward
            let d_out = 2.0 * err * (1.0 - pred * pred);
            net.b2 -= self.lr * d_out;
            for k in 0..h {
                if acc[k] > 0.0 {
                    let d_w2 = d_out * acc[k];
                    let d_acc = d_out * net.w2[k];
                    net.w2[k] -= self.lr * d_w2;
                    net.b1[k] -= self.lr * d_acc;
                    for &f in &idx {
                        net.w1[f as usize * h + k] -= self.lr * d_acc;
                    }
                }
            }
        }
        total / data.len().max(1) as f32
    }
}
