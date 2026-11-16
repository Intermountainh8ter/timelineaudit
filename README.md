# timelineaudit

A pre-registered ledger of one thing that can be checked about the claim
that artificial general intelligence, and the end of the world as we know
it, is due around 2027: **the dated predictions those forecasts make along
the way, scored against what happened.**

Whether a scenario's ending is right cannot be known before it arrives, and
this project does not claim to know. But a scenario that reaches its crisis
in 2027 also says what should be true in 2025 and 2026. Those statements
have dates, and the dates pass. timelineaudit records each one, with its
source and the rule for deciding it, and scores the record. Sources that
predict slow progress are scored by the same rules.

## What it measures

| measure | question | score | null value |
|---|---|---|---|
| hit rate | Of the claims whose date has passed, how many happened on time? | share of resolved claims | none: depends on how hard the claims were |
| timing slip | How far from the stated date did events land? | median months, observed minus stated | 0 |
| Brier skill | Were the stated probabilities better than saying 0.5 to everything? | 1 - Brier / 0.25 | 0 |

Each is reported for all claims, per source, per class of claim
(capability, deployment, economy, governance, safety) and for short-lead
and long-lead claims apart. The hypotheses, the rule for extracting claims
without cherry-picking, and the limits on what may be concluded are fixed in
[docs/PREREGISTRATION.md](docs/PREREGISTRATION.md).

## What it does not say

No result here says when AGI will arrive, or that it will not, or how
dangerous it would be. A good record on near claims does not make a far
claim true, and a poor one does not make it false.

## The degenerate-forecaster audit

Each measure is first run against forecasters that know nothing. If one of
them can pass a measure, the measure is not measuring foresight.

`results/000-baselines.json` holds that audit: 400 simulated claims, one
simulated world, five forecasters. The forecaster that states each event's
true chance has a Brier skill of 0.290669. Saying yes to everything scores
-1.06, saying no to everything -0.94, and a coin 0.0. All four share a hit
rate of 0.485, which is why the hit rate is never read as skill on its own.
A forecaster that puts every date 90 days too early shows a median slip of
3.334702 months against 0.377823, and a hit rate of 0.245. This is a
simulation; no real claim has been scored.

## The selection audit

`results/271-selection-audit.json` draws 15 claims from the same kind of
simulated world, 200 times, by three rules. Against the full set, a random
draw moves the median slip by 1.839836 months (median over draws), a
hit-favouring pick by 3.252567 months and a miss-favouring pick by 8.443532
months. Which claims are included can move a score as much as the thing
scored. This is a simulation; no real grading was examined.

## The power audit

`results/274-power-brier.json` draws claim sets of 20, 50 and 100 from the
same kind of simulated world, 200 times each, for the forecaster that states
each event's true chance. The 1000-resample interval on Brier skill against
the coin has a lower end above zero in 0.405 of draws at 20 claims, 0.75 at 50
and 0.975 at 100. Draws overlap, so they are not independent. This is a
simulation; it sets a rough minimum claim count for the Brier measure and
says nothing about any real claim set.

## The null audit

`results/275-brier-null.json` repeats the power audit for a forecaster whose
stated probabilities are random draws independent of the outcome. The
interval on Brier skill has a lower end above zero in 0.0 of draws at 20, 50
and 100 claims, with median skill of -0.429369, -0.46325 and -0.441922. The
power rates above are therefore not produced by the interval method alone.
This is a simulation, one finite world, and says nothing about any real claim
set.

## First real claims

`results/278-real-claims-first.json` scores the claims entered from the five
dated sections of the AI 2027 scenario (Mid 2025 to Late 2026), scoring date
2026-11-16. Of 32 listed items, 3 were checkable by the pre-registered rule
and 29 were counted not-checkable. Of the 3 claims, 1 is resolved and 2 are
not yet due. The resolved claim (a public model trained with 1e27 FLOP by the
end of 2025; the largest Epoch AI estimate by then is 5.0e26 FLOP) did not
happen on time, so the hit rate is 0.0 on 1 resolved claim. Fewer than 5
claims resolved, so under the pre-registered stop rule the result is
inconclusive and the hypothesis is not scored. It says nothing about when AGI
will arrive.

## Status

The scorer, the claim format and the audit exist. The claim table holds 3
claims from one source.
Claims are added one pre-registered section at a time, each with its wording
and resolution rule in `preregistration_additions.md` before its outcome is
looked up.

## Use

```
cargo test --locked
cargo run --locked --release -- baselines --seed 7 --n 400 --out results/<name>.json
cargo run --locked --release -- selection-audit --seed 7 --n 400 --pick 15 --draws 200 --out results/<name>.json
cargo run --locked --release -- power-brier --seed 7 --n 400 --sizes 20,50,100 --draws 200 --boot 1000 --out results/<name>.json
cargo run --locked --release -- check --claims claims/claims.tsv --as-of <YYYY-MM-DD>
cargo run --locked --release -- score --claims claims/claims.tsv --as-of <YYYY-MM-DD> --out results/<name>.json
```

The crate has no dependencies and builds offline. The scoring date is always
given on the command line. Results files are never overwritten.

## Licence

MIT.

## The date-reading audit

`results/287-slack-sensitivity.json` scores the same 400 simulated claims with the stated date and with the stated date plus 30, 90 and 180 days. The hit rate moves from 0.485 to 0.5175, 0.575 and 0.66, and the number of miss verdicts that become hits is 13, 36 and 70. Which reading of a date is used can move a score as much as the thing scored. This is a simulation; no real claim has been scored under a window.
