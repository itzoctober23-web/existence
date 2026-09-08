//! Dump a reference program in the same Debug format evolve.rs saves, so an evolved program can be
//! diffed against the seed it came from. Without this, `evolved_gen13.prog` is 262 lines with no
//! baseline and "what did the search actually find" stays unanswerable.
use grammar::reference;
fn main() {
    let which = std::env::args().nth(1).unwrap_or_else(|| "seed".into());
    let p = match which.as_str() {
        "capture" => reference::capture_extension(),
        "reduction" => reference::table_reduction(),
        _ => reference::bare_alpha_beta(),
    };
    println!("// reference {which}, {} nodes", p.size());
    println!("{p:#?}");
}
