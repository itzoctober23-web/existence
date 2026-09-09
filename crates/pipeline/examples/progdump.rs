//! Snapshot a reference program to `.sexp`. The writer half of `sexp.rs`, as a tool.
//!
//! Useful on its own (a reference program becomes a file you can diff a champion against), and it is
//! what makes the end-to-end control possible: dump a program, load it back through the FILE path,
//! and match it against the in-memory original. Structural equality is already unit-tested; this
//! exercises the integration -- encoding, trailing newline, argument plumbing -- that a unit test on
//! `from_str(to_string(p))` never touches.
use grammar::{reference, sexp};

fn main() {
    let mut a = std::env::args().skip(1);
    let (want, out) = match (a.next(), a.next()) {
        (Some(w), Some(o)) => (w, o),
        _ => { eprintln!("usage: progdump <name-substring> <out.sexp>");
               eprintln!("  names: {:?}", reference::all().iter().map(|(n,_)| *n).collect::<Vec<_>>());
               std::process::exit(2); }
    };
    let all = reference::all();
    let hit: Vec<_> = all.iter().filter(|(n, _)| n.contains(&want)).collect();
    if hit.len() != 1 {
        eprintln!("{want:?} matched {} programs: {:?}", hit.len(),
                  hit.iter().map(|(n,_)| *n).collect::<Vec<_>>());
        std::process::exit(2);
    }
    let (name, p) = hit[0];
    let text = sexp::to_string(p);
    std::fs::write(&out, &text).unwrap_or_else(|e| panic!("cannot write {out}: {e}"));
    let nodes: usize = p.funcs.iter().map(|f| f.body.size()).sum();
    println!("wrote {out}: {name} ({nodes} nodes, {} bytes)", text.len());
}
