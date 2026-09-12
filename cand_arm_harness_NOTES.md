# Harness facts for the Candidate A arms, checked rather than assumed

**2026-09-12.** Three things about running these arms that are not obvious from the code, recorded
because each could have silently invalidated the comparison.

## 1. The old binary SILENTLY IGNORES `--datagen-budget-labels-only`

Arms A and B run `target/release/learn_cand`, copied **before** cell C was implemented. Cell C needs
`learn_cand2`. Verified by the binaries' own banners rather than by `strings`:

```
learn_cand2 : datagen TRUE node budget 5269/move ... [LABELS ONLY: moves come from fixed depth, cell C]
learn_cand  : datagen TRUE node budget 5269/move ...                     <- flag accepted, ignored
```

Launching cell C on the old binary would have produced **a second copy of arm B**, and the
decomposition would have been two identical arms reported as different ones. Unknown flags are not
rejected, so nothing would have complained.

*(An earlier `strings | grep -c 'budget-labels-only'` returned 0 for BOTH binaries and nearly sent me
the wrong way — the literal is split in the binary, and `labels-only` matches. A grep that finds
nothing is usually a broken pattern; the tool's own banner is the authority.)*

## 2. Binary equivalence, so a 3-way comparison across two binaries is legitimate

`learn_cand2` adds only the cell-C branch, guarded by `BUDGET_LABELS_ONLY != 0`. That should make it
behaviourally identical on the arm-A and arm-B paths, but "should" is not a measurement. Same seed,
same settings, 15 generations, compare the trained net byte-for-byte:

```
ARM-A path (fixed depth 3)   learn_cand 334c3545d248ab7c   learn_cand2 334c3545d248ab7c   IDENTICAL
ARM-B path (budget on)       learn_cand 1c1f5341fe00039c   learn_cand2 <pending>
```

The arm-A path is confirmed identical. **The arm-B path is still running and is reported as pending,
not as a pass.** Cell C does not launch until it is confirmed.

## 3. The trainer does NOT exit at `--gens N` — there is a post-loop CONTROL

After the last generation it plays a ~900-game control, "final champion vs the ORIGINAL random net",
at full CPU. A run that has logged `gen 2000` is therefore still *active* for several more minutes.

This is why `gate_candidate_a.sh` guards on **the systemd unit being inactive** as well as on the
generation count. A gate keyed to the generation count alone would have fired while both arms were
still running, and measured nets that were still being written.

Also noted: the two equivalence runs produced **byte-identical nets but different control results**
(0.991 ± 0.006 against 0.919 ± 0.018). The trained net — the arms' actual product — is identical, so
equivalence holds. The post-loop control is evidently not seeded deterministically, or is
load-sensitive. Recorded because a reader comparing those two lines would otherwise conclude the
binaries differ. It is a diagnostic, and nothing here relies on it.
