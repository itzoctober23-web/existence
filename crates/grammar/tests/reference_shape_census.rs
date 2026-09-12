//! A descriptive fact I kept inferring from filters instead of measuring: how many functions, and
//! of what sizes, does each reference program actually have?
//!
//! Both AddFn tests tonight filtered on `funcs.len()` without ever reporting the distribution, so
//! "only one 1-function program exists" was a by-product of a filter rather than a stated fact.
use grammar::reference;

#[test]
fn reference_program_shapes() {
    println!("\n{:<46} {:>5} {:>22} {:>7}", "program", "funcs", "sizes", "total");
    for (name, p) in reference::all() {
        let sizes: Vec<usize> = p.funcs.iter().map(|f| f.body.size()).collect();
        let tot: usize = sizes.iter().sum();
        println!("{:<46} {:>5} {:>22} {:>7}", name, p.funcs.len(), format!("{sizes:?}"), tot);
    }
}
