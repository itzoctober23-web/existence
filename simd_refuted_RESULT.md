# `target-cpu=native` buys NOTHING — the width-vs-clock result is not a SIMD artifact

**2026-09-10.** Tested because the width conclusion ("wider is better per node, worse per second, at
every width") would be an artifact if inference were running far below the machine's capability.

## The setup was exactly the suspicious one

* `crates/nnue` is **pure scalar Rust** — no `std::arch`, no `target_feature`, no SIMD types. The hot
  loops (`Acc::add`, `Acc::output`, `Net::eval`) are `iter_mut().zip()` over `f32`.
* There is **no `.cargo/config.toml`**, no `target-cpu` in `Cargo.toml`, and `RUSTFLAGS` is unset —
  so rustc targets **x86-64 baseline: SSE2, 4-wide f32, no FMA**.
* The CPU has **AVX2 and FMA** — 8-wide, fused.

On paper that is 2× vector width plus FMA left on the table, on the hottest function in the program.

## It makes no difference

Identical work by construction: `netmatch` at FIXED depth searches the same tree and evaluates the
same positions regardless of build flags. Scores matched exactly in both arms, which confirms it.

| width | baseline (SSE2) | native (AVX2+FMA) | delta |
|---|---|---|---|
| w16 (the shipped champion), 64 pairs @ d3 | 13119 ms | 13083 ms | **0.3%** |
| w512, 24 pairs @ d3 | 43199 ms | 42949 ms | **0.6%** |

Both arms returned the same score (0.445 and 0.562 respectively), so the two builds did the same
work and only wall-clock could differ. It did not.

## What this rules out, and what it does not

**RULED OUT:** that the width-vs-clock finding is a codegen artifact. Turning on the machine's full
vector width changes nothing at either width, so "wider is worse per second" is a real property of
this engine and not of its compiler flags. **Do not re-run the width arm expecting SIMD to flip it.**

**NOT ruled out — and this is the part still worth attacking:** that inference is slow for a
*representational* reason rather than an instruction-set one. Real NNUE engines quantise to int16/int8
accumulators; this one carries `f32` throughout. That is a memory-bandwidth and cache-footprint
difference, not an ISA difference, and it is exactly the kind of thing AVX2 cannot rescue — at w512
one accumulator is 2KB of f32 against 512B of int8, and the row-adds are bandwidth-bound.

The measurement that would settle THAT is a profile showing whether `Acc::add` is stalled on memory
rather than issue-limited. Not run here; recorded as the open question that survives this refutation.
