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

## The aggregation audit

`results/294-aggregation-audit.json` draws 15 resolved claims from the same
kind of simulated world, 200 times. The median absolute gap between the
claim-pooled hit rate and the unweighted mean of per-class hit rates is
0.05. Which unit is averaged can move a headline hit rate. This is a
simulation; no real claim set was aggregated.

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

## The convention audit

`results/448-censor-convention.json` scores 400 simulated claims per class
under two slip conventions: overdue never-happened claims kept as lower
bounds, or dropped. The pre-registered hypothesis, a capability-class gap of
at least 0.5 months, was not supported: the capability class has 0 censored
claims of 67 resolved, so the gap is 0.0 months. The deployment class gap is
1.412731 months (5 censored of 67) and the other class gap is 1.494867
months (3 censored of 66). The convention matters where overdue
never-happened claims sit, which is a property of the world and not of the
rule. This is a simulation, one finite world, and says nothing about any
real claim set.

## The censored lead audit

`results/452-lead-censored.json` scores 400 simulated claims made 450 days
before the as-of date 2025-09-01, so long-lead due dates fall after it and
those claims can stay open. The pre-registered hypothesis, a long-minus-short
resolved share of at most -0.2, was supported: the share is -0.638783. Short
lead claims resolve at 1.0 (137 claims, 0 open) and long-lead claims at
0.361217 (263 claims, 168 open). Median slip is -0.558522 months (short) and
-2.135524 months (long), the long figure computed on the claims that have
resolved only. A lead-time comparison on a young claim set is therefore
mostly a comparison of how much of each bin is still open. This is a
simulation, one finite world, and says nothing about any real claim set.

## The Kaplan-Meier slip audit

`results/455-km-slip.json` takes the 400-claim simulated world with claims
made 450 days before the as-of date. For long-lead claims the naive median
slip over claims that happened is -3.449692 months, against 1.281314 months
from a Kaplan-Meier median that treats claims not yet happened as
right-censored. Naive minus censoring-aware is -4.731006 months. Dropping
open claims biases the slip early. This is a simulation, one finite world,
and says nothing about any real claim set.

## The class-mix audit

`results/456-class-mix.json` resamples the 400-claim simulated world with
replacement, 200 draws of 400 per share, so the capability share is 0.1, 0.5
or 0.9. The median over draws of the pooled median slip is 0.459959 months,
-0.164271 months and -0.542094 months, a range of 1.002053 months. Which
classes are in a sample moves the pooled slip. This is a simulation, one
finite world, and says nothing about any real claim set.

## The pace-snapshot audit

`results/458-pace-snapshots.json` scores one fixed simulated 400-claim world
at two as-of dates. The share of claims resolved rises from 0.2375 on
2025-03-01 to 0.58 on 2025-09-01, and the naive median slip over occurred
claims moves from -3.613963 to -2.628337 months, a shift of 0.985626 months,
although the world did not change. A self-graded pace figure can move between
snapshots only because more claims resolved. This is a simulation and scores
no real claim.

## The group-gap audit

`results/464-group-gap.json` plants a 4.000000-month slip gap between a fast
group and a slow group in 400 simulated claims, drops the claims still open at
the as-of date, and measures the gap on the rest. The measured gap is
3.531828 months, an absolute recovery error of 0.468172 months, from 112
resolved fast claims and 150 resolved slow claims. This is a simulation, one
seed; it says nothing about any real source.

## The ceiling audit

`results/471-ceiling-doubling.json` fits a doubling time to a simulated horizon series whose true doubling time is 6 months, clipped at suite ceilings. With the cap at 0.25 of the top horizon the fit overstates the doubling time by 2.172805 months; the uncapped fit is off by -0.00149 months. The bias at the lowest cap exceeds the uncapped bias by 2.174295 months. This is a simulation of one series and says nothing about any measured horizon.

## The suite-refresh audit

`results/473-suite-refresh.json` fits a doubling time to a simulated horizon series whose true doubling time is 6 months, with a one-time level step of 1 log2 unit at month 18 of a suite refresh. The naive fit is off by -1.174877 months; a fit with a step term is off by -0.048344 months. This is a simulation of one series with a known refresh month and says nothing about any measured horizon.

## The revision audit

`results/472-revision-audit.json` scores 400 resolved simulated claims for a prior forecast, an informative revision and a noise-only revision. The informative revision's Brier skill against the coin is 0.230045 above the prior's, the noise-only revision's is -0.047939 relative to the prior's, and an always-0.5 responder scores 0.000000. This is a simulation; no real forecast or revision is scored.

## The pace-ratio audit

`results/474-pace-ratio-conventions.json` scores 40 simulated quantitative claims with a true pace ratio of 0.65, 6 of them graded against a wrong curve. Mean-of-ratios gives 0.76247, ratio-of-sums 0.739932 and median-of-ratios 0.542227, a spread of 0.220243; with no mis-graded claims the spread is 0.197523. The spread is mostly the convention, not the mis-grading. This is a simulation of one claim set and says nothing about any real grading.

## The fit-points audit

`results/475-fit-points.json` fits a doubling time to the last n of 37 monthly points in 500 simulated series with a known true doubling time of 6 months, and projects the date for a fixed level at month 60. The median absolute date error is 1.126816 months at n=6 and 0.219417 months at n=20, a ratio of 5.135492, and 1.92094 months at n=4. Coverage of the 95% interval for the fitted level is 0.948 at n=6 and 0.948 at n=20, so the interval stays near nominal while the error grows. This is a simulation with the correct model form; it says nothing about any real benchmark.

## The snapshot-count audit

`results/476-snapshot-count.json` counts confirmed claims in one fixed
simulated world of 400 claims at four as-of dates six months apart. The
strict-rule count spreads by 144 across dates; adding 90 days of slack moves
the count by at most 36 at any one date, a ratio of 4.0. The snapshot date
moved the count more than the counting rule. This is a simulation; no real
tracker is scored.

## The window audit

`results/481-window-spread.json` simulates 500 frontiers with one constant
doubling time of 210 days and triangular noise of half-width 0.3 in log2
horizon, then fits doubling time from four start windows (months 0, 12, 24
and 36). The median range across windows is 3.12176 days, and the share of
draws with a range of at least 20 days is 0.0. The pre-registered hypothesis
of at least 20 days is refuted for this setup. This is a simulation with
correct exponential form; no real measured horizon is scored.

## The label-noise audit

`results/482-label-noise.json` simulates 500 suites of 200 tasks with a true
50% horizon of 60 minutes and a true slope of 1 per natural-log unit of
length, then refits a logistic on length labels carrying log-normal noise.
At sigma 1.0 the median log ratio of fitted to true 80% horizon is -0.32816
and the median fitted slope is 0.773652; the 50% horizon log ratio is
0.064492. At sigma 0 the 80% log ratio is 0.024324. The pre-registered
hypothesis is supported for this setup: noisy labels understate the 80%
horizon far more than the 50% one. This is a simulation with correct logistic
form; no real benchmark or measured horizon is scored.

## The difficulty-spread audit

`results/483-difficulty-spread.json` simulates 500 suites of 200 tasks whose
log-odds of success carry a per-task offset of spread tau around a log-linear
function of length (true 50% horizon 60 minutes, slope 1 per natural-log unit).
It compares a two-parameter logistic fit with the true marginal horizons. At
tau 2 the median absolute log ratio of fitted to marginal 80% horizon is
0.277716, below the pre-registered 0.3, and the 50% horizon figure is
0.192547. At tau 0 the 80% figure is 0.192455, above the 0.1 the hypothesis
allowed, so the finite-sample error alone exceeds that bound. The
pre-registered hypothesis is not supported for this setup. This is a
simulation with Gaussian offsets; no real benchmark or measured horizon is
scored.

## The tasks-per-model audit

`results/485-tasks-per-model.json` simulates 8 models on a frontier with a true
doubling time of 7 months, 200 replicates per setting, and gives each model
10, 30, 100 or 200 tasks. The share of replicates with a wrong-sign or
undefined slope is 0.04 at 10 tasks and 0.0 at 200 tasks, so the pre-registered
threshold of 0.05 at 10 tasks was not reached and the hypothesis is refuted.
The median absolute log error of the fitted doubling time is 0.273536 at 10
tasks and 0.040427 at 200. This is a simulation with a correct logistic form
and says nothing about any real benchmark.

## The cluster-coverage audit

`results/486-cluster-coverage.json` draws 20 families of 5 tasks with a shared difficulty offset, 300 replicates and 200 bootstrap resamples per interval. At offset spread 1.0 the 95% task-level bootstrap interval covers the true 50% horizon in 0.876667 of replicates and the family-level interval in 0.92; at spread 0 they cover in 0.94 and 0.92. The pre-registered expectation of at most 0.85 for the task-level interval was not met. This is a simulation with one suite size; no real benchmark was scored.

## The task-influence audit

`results/487-task-influence.json` draws 200 simulated suites of 40 tasks for a model whose true 50% horizon is 500 minutes, with the longest task at 1000 minutes, and refits dropping each task in turn. With 2 tasks above the horizon the median largest relative shift in the fitted horizon is 0.424353; with 20 tasks above it, 0.152267. Dropping one task moves a horizon estimate much more when few long tasks anchor it. This is a simulation with a correct logistic form; no real benchmark was scored.

## The baseline-censoring audit

`results/488-baseline-censoring.json` draws 200 simulated suites of 100 tasks and 8 models on a frontier with a true doubling time of 7 months, takes each task's length from successful baselines only, and lets baseliner failure rise with task length. The median fitted-to-true doubling-time ratio is 0.970912 at the highest failure rate against 0.978885 with no failures, inside the pre-registered 0.95 to 1.05 band, so the hypothesis is refuted for this failure model. This is a simulation and scores no real baseline.

## The flagged-runs audit

`results/490-flagged-runs.json` simulates 8 models and 100 tasks, 200 suites per setting, and flags a share of the newest model's successes as illegitimate. Scored as counted, as failure, or dropped, the median fitted 50% horizon of that model differs across rules by a max/min ratio of 1.0 at a flagged share of 0 and 53.97484 at 0.5 (counted 254.555547 minutes, failure 4.716189, dropped 130.102285). This is a simulation with a correct logistic form; no real run or flag is scored.
