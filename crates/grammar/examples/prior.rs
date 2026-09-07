//! Report the MEASURED node counts of the reference programs — the numbers GRAMMAR 6 states
//! its prior in. Run: cargo run --release --example prior -p grammar
use grammar::reference;

fn main() {
    let progs = reference::all();
    let seed = progs
        .iter()
        .find(|(n, _)| n.starts_with("bare alpha-beta"))
        .map(|(_, p)| p.size())
        .unwrap();
    println!("  MEASURED node counts (parser, not estimates)\n");
    println!("  {:<32} {:>6}  {:>10}", "program", "nodes", "vs seed");
    for (name, p) in &progs {
        let n = p.size();
        let d = n as i64 - seed as i64;
        println!("  {:<32} {:>6}  {:>+10}", name, n, d);
    }
    println!("\n  main seed (bare alpha-beta) = {seed} nodes");
    let mcts = progs.iter().find(|(n, _)| n.contains("MCTS")).unwrap().1.size();
    let pn = progs.iter().find(|(n, _)| n.contains("proof-number")).unwrap().1.size();
    let d1 = progs.iter().find(|(n, _)| n.starts_with("depth-one")).unwrap().1.size();
    println!("  declared prior: MCTS is {:+} nodes from the seed, PN {:+}", mcts as i64 - seed as i64, pn as i64 - seed as i64);
    println!("  purity distance: depth-one -> bare AB = {} nodes", seed as i64 - d1 as i64);
}
