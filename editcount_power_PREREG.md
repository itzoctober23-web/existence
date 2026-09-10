# Pre-registration: what the edit-count sweep CAN and CANNOT resolve

Written and committed **while the sweep is still on its edits=1 control arm**, so this is a
pre-registration rather than an explanation of a number I have already seen.

## The power arithmetic

`ALL_OPS` has 11 operators and `mutate_program_n` draws one uniformly per edit. A candidate carries
BOTH halves of the TT rung only if its draws include `ProbeRead` AND `StoreHere` — `ProbeRead` alone
gives Probe+Key+Field, `StoreHere` alone gives Store.

    edits=1 : P(both) = 0 BY CONSTRUCTION. One edit cannot draw two operators.
    edits=2 : P(both) = 2 * (1/11)^2 = 0.0165  ->  expected in n=40: 0.66
    edits=3 : P(both) = 0.0451                 ->  expected in n=40: 1.80

**Both expectations are below 1.** A `both_halves == 0` at n=40 is therefore entirely consistent with
the operators working perfectly — it would measure my sample size, not the search space.

    n for ~3 expected hits at edits=2 : 182
    n for ~3 expected hits at edits=3 :  67

And operator DRAW is only half the requirement: the two edits must also LAND correctly — probe before
recursing, store after. Placement is not modelled above, so these are **upper bounds** on the hit
rate. The true rate can only be lower.

## What the sweep at n=40 DOES resolve

The `identical` / `cheaper` / `guard-ok` columns are ~50%, ~0% and ~0% rates. At n=40 those are
adequately powered, and they answer the question the sweep was actually launched for: **does raising
the edit count from 1 to 2 or 3 produce a behaviour-preserving candidate that is CHEAPER** — a PATH-1
acceptable child, which the loop would take with no games at all.

## Pre-registered readings

* `identical_cheaper > 0` at edits=2 or 3 → PATH 1 is reachable inside the loop's own budget, and the
  failure is one of SAMPLING, not structure. This is the result that would matter.
* `identical_cheaper == 0` across all three → the valley holds even when both halves can be installed
  in one step, and the barrier is placement rather than draw.
* **`both_halves == 0` at n=40 → NOT INTERPRETABLE.** Expected count is 0.66 and 1.80. I will report
  it as underpowered and, if the cheaper column is also zero, run n=182 at edits=2 before drawing any
  conclusion about whether the pair is constructible in practice.

## Why this is written down first

Five analyses tonight rested on a premise the repo had already retired, and the common thread was
reasoning that ran ahead of checking. A zero with an expected count of 0.66 is exactly the kind of
number that reads as decisive and is not. Committing the arithmetic before the result removes the
option of deciding afterwards which reading was intended.
