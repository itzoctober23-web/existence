#!/bin/bash
# Does the loop's own 1-3 edit budget jump the conjunctive valley?
#
# THE QUESTION. `Op::ProbeRead` emits Field(Probe(Key(Var "p")), f) in ONE edit; `Op::StoreHere`
# emits a Store. So hash reuse is TWO well-placed edits. The valley says each half alone is WORSE
# (probe 0.991x, store 0.997x) and only the pair pays (1.024x) -- so every 1-edit measurement taken
# so far could only ever see the valley floor, which is exactly what it saw: 0 of 100
# behaviour-preserving single edits were cheaper.
#
# CONTROL FIRST, and it is a real one: edits=1 must reproduce the recorded n=200 shape
# (~50% identical, 0 cheaper, 0 guard-ok). If it does not, the edit-count argument changed something
# it should not have and the edits=2 numbers are void.
#
# PRE-REGISTERED READING:
#   * both_halves > 0 at edits=2 and 0 at edits=1  -> the budget CAN install the pair; the question
#     moves to whether those children are correct and cheaper.
#   * identical_cheaper > 0 at edits=2             -> PATH 1 is reachable in two edits and the loop
#     should already be finding it, which would make the failure one of SAMPLING, not structure.
#   * both_halves == 0 at edits=2                  -> two random edits essentially never co-locate
#     a probe and a store correctly, and the barrier is combinatorial placement rather than the
#     valley depth. That is a different fix (targeted operators) from a different problem.
set -u
cd /home/maswabe/existence || exit 1
B=./target/release/examples/evolve
for e in 1 2 3; do
  echo "########## edits=$e ##########"
  nice -n 19 taskset -c 0-11 "$B" stepdiff 40 3 "$e"
  echo
done
