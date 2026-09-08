fn main() {
    use grammar::mutate::{mutate_program_n, Rng, ALL_OPS};
    use grammar::reference;
    let champ = reference::bare_alpha_beta();
    let mut drawn = std::collections::BTreeMap::new();
    let mut placed = std::collections::BTreeMap::new();
    let mut none = 0;
    for g in 1..=17u64 {
        for i in 0..24u64 {
            let mut r = Rng::new((g << 24) ^ i ^ 0xBEEF);
            // what the raw draw picks, before placement
            let mut probe = Rng::new((g << 24) ^ i ^ 0xBEEF);
            let op = ALL_OPS[probe.below(ALL_OPS.len())];
            *drawn.entry(format!("{op:?}")).or_insert(0) += 1;
            match mutate_program_n(&champ, &mut r, 1) {
                Some((_, ops)) => { *placed.entry(format!("{:?}", ops[0])).or_insert(0) += 1; }
                None => none += 1,
            }
        }
    }
    println!("DRAWN (before placement):");
    for (k, v) in &drawn { println!("   {k:<14} {v}"); }
    println!("PLACED (what the ledger sees):  none={none}");
    for (k, v) in &placed { println!("   {k:<14} {v}"); }
}
