# (surrogate, VERIFY) pairs, DEDUPLICATED by (run-seed, lineage, gen) and STRATIFIED by guard.
#
# CORRECTION 2026-09-09: the first version pooled n=7 that contained duplicates -- arms with
# identical config replay the SAME trajectory, so seed-1 gen-3 appeared in three logs and seed-2
# gen-1 in two -- and it mixed a relaxed-guard arm (tolerance 7 / floor 16) in with the standard
# ones (tolerance 4 / floor 19). Duplicated rows inflate apparent n; mixed conditions are not one
# population. Both errors are removed here and the Spearman is withdrawn as uninterpretable at n=3.
MAIN_SEED, MCTS_SEED = 0.002490, 0.001406
std = [  # guard tolerance 4, MAIN floor 19
  ("MAIN", 2, 2, 0.002817, 0.430, 0.030),
  ("MAIN", 1, 3, 0.002924, 0.422, 0.027),
  ("MAIN", 1, 4, 0.003045, 0.430, 0.030),
  ("MCTS", 2, 1, 0.001506, 0.505, 0.019),
  ("MCTS", 2, 2, 0.001811, 0.492, 0.018),
  ("MCTS", 1, 4, 0.001848, 0.490, 0.018),
]
relaxed = [("MAIN", 2, 2, 0.004618, 0.258, 0.041)]  # guard tolerance 7, MAIN floor 16
def show(rows, title):
    print(f"\n  {title}")
    print("    lineage seed gen  surrogate   gain     VERIFY          verdict")
    for lin,sd,gn,s,v,ci in rows:
        seed = MAIN_SEED if lin=="MAIN" else MCTS_SEED
        verdict = "RESOLVED WORSE" if v+ci < 0.5 else "not resolved"
        print(f"    {lin:<5} {sd:>4} {gn:>4}  {s:.6f} {100*(s/seed-1):+6.1f}%  {v:.3f}+/-{ci:.3f}  {verdict}")
show(std, "STANDARD GUARD (tolerance 4, floor 19) -- n=6 unique")
show(relaxed, "RELAXED GUARD (tolerance 7, floor 16) -- n=1, DIFFERENT CONDITION")
for lin in ("MAIN","MCTS"):
    rows=[r for r in std if r[0]==lin]
    worse=sum(1 for r in rows if r[4]+r[5] < 0.5)
    print(f"\n  {lin} under the standard guard: {worse}/{len(rows)} resolved WORSE")
print("\n  Spearman WITHDRAWN: n=3 per lineage after dedup is not interpretable.")
