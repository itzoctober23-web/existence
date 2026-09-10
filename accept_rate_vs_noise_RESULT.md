# Accepts are NOT noise: 8.87% observed against a 2.50% false-positive floor (z = +29.1)

**2026-09-10.** The ancestor control shows no measurable gain over 400-generation windows
(0.474 ± 0.031, three readings all at or below 0.5). The natural explanation is that the gate's
accepts are largely noise, so the champion random-walks. This tests that directly, from logs that
already existed.

## The test, which costs nothing

The gate accepts on `resolved_up`, i.e. **`rate − ci95 > 0.5`** — a one-sided 97.5% test. So if
every candidate were EXACTLY equal to the champion, the gate would still accept on **2.50% of
generations, by construction**. That number is not an estimate; it falls out of the decision rule.

The observed accept rate is therefore directly diagnostic, and it is already in the log.

## The measurement

| | |
|---|---|
| generations | **5,095** |
| accepts | **452** |
| observed rate | **8.87%** |
| null floor (all candidates equal) | **2.50%** |
| z | **+29.1** |

**At most 28% of accepts are explainable as false positives**, so at least ~72% are candidates that
genuinely resolved up at the gate's depth. The random-walk explanation for the plateau, in its
strong form — "accepts are noise" — is **refuted**.

## What it does NOT resolve, and this is the live gap

The gate measures at the **datagen depth (1)**. "Genuinely resolved up" therefore means *stronger at
depth 1*. The ancestor control measures at **depth 4**. So this result is fully compatible with the
plateau if depth-1 gains do not accumulate into depth-4 strength — and `reject_audit` already
established the neighbouring fact that REJECTED candidates sit at exactly champion strength at
depth 4 (0.4982 [0.4847, 0.5117], n=34).

The accept audit now banking samples is the direct test of that gap: it plays accepted candidates
against the champions they beat, **at depth 4**.

## A second pattern, stated as a pattern and not a finding

Accept rate by run, oldest to newest:

```
12.44%   16.46%   10.95%   8.97%   6.20%   2.04%   6.47%
```

It declines as the champion matures, and run 8 (2.04%) sits at the noise floor — but that run is
only 147 generations and today's runs changed configuration mid-stream (gate depth default, ancestor
pairs, sampling flags). **So the decline is confounded with configuration and must not be read as
"the loop is running out of improvements" on this evidence.** What is solid is the pooled figure
against the floor; the trend needs runs held at one configuration.

## Why this was worth doing before the audit finished

The audit needs ~516 pairs at depth 4 and about an hour of E-core time. This test needed a log
already on disk and a number implied by the decision rule. It rules out the strongest version of the
plateau hypothesis immediately, and it sharpens what the audit has to answer: not "are accepts
real?" but "do real depth-1 accepts transfer to depth 4?".
