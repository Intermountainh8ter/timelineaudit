# Pre-registration additions

Dated sections are appended here before the work they describe is scored:
new sources with their extraction rule, every claim with its exact wording
and resolution rule, new hypotheses, and corrections. Nothing above a new
section is edited.

## 2026-11-16T16:22:21Z selection-audit proposal:b81d65846122ecd0fdc15e4288768bc4368c6e238f760081ecec206ab0cd4a52

Simulated; no real claim is used. One simulated world of 400 claims (seed 7, as-of 2026-01-01, the calibrated forecaster's claims from `synth_claims`). Three pick rules each draw 15 claims, 200 times: `random` (uniform without replacement, the extract-everything rule's sampling counterpart), `hit_favouring` (uniform without replacement among claims whose status is hit) and `miss_favouring` (the same among misses). For each draw the shift is the absolute difference between the picked subset's median slip (months) and the full-set median slip; the headline is the median shift over the 200 draws. Hit rate shift is reported the same way.

- Hypothesis: the hit-favouring median slip shift is at least 3.0 months (the smallest effect of interest) while the random-draw shift is below 3.0 months.
- Metric: `/selection/hit_favouring/median_abs_slip_shift_months`, with `/selection/random/median_abs_slip_shift_months` as the counterpart.
- Expected: metric at least 3.0 months, random below 3.0 months.
- Falsifier: hit-favouring shift under 3.0 months, or random shift of 3.0 months or more, at seed 7.
- Command: `cargo run --locked -q --release -- selection-audit --seed 7 --n 400 --pick 15 --draws 200 --out results/271-selection-audit.json`
- Limit: a statement about the simulated world only; it says nothing about any real grading.

## 2026-11-16T16:45:12Z power-brier proposal:cb761bbee0106b009e20670be9837ccc1db30bd65e1384c6da425474927fb069

Simulated; no real claim is used. One simulated world of 400 claims (seed 7, as-of 2026-01-01, the calibrated forecaster's claims from `synth_claims`, which states each event's true chance). For each size n in 20, 50, 100, the command draws n claims without replacement from the resolved claims, 200 times. For each draw it computes the 1000-resample percentile interval on Brier skill against the 0.5 coin (the interval used by `score`). `excludes_zero_rate` is the share of draws whose interval lower end is above zero.

- Hypothesis: in the simulated world, for the forecaster that states each event's true chance, the share of 200 simulated claim sets whose 1000-resample percentile interval on Brier skill vs the coin has a lower end above zero is at least 0.80 at 100 claims, and is lower at 20 claims than at 100 claims.
- Metric: `/power_brier/by_n/100/excludes_zero_rate`.
- Expected: metric increases with n and is at least 0.80 at n=100 (range 0.8 to 1.0).
- Falsifier: excludes_zero_rate at n=100 below 0.80, or the rate at n=20 not below the rate at n=100, at seed 7.
- Command: `cargo run --locked -q --release -- power-brier --seed 7 --n 400 --sizes 20,50,100 --draws 200 --boot 1000 --out results/274-power-brier.json`
- Limit: draws come from one finite world, so they overlap and are not independent; a statement about the simulated world only, and about the Brier measure only, not any real claim set.

## 2026-11-16T16:50:19Z brier-null proposal:cb64c5c42a9bcdd0bd9676efc7c37e0d0f86fea053e0a4dee60c7d2b8b7fcf56

Simulated; no real claim is used. The world is the one used by power-brier (400 claims, seed 7, as-of 2026-01-01, outcomes from `synth_claims`), but each claim's stated probability is replaced by a uniform random draw on [0, 1] independent of the outcome. For each size n in 20, 50, 100 the command draws n resolved claims without replacement, 200 times, and computes the 1000-resample percentile interval on Brier skill against the 0.5 coin (the interval used by `score`). `excludes_zero_rate` is the share of draws whose interval lower end is above zero.

- Hypothesis: for a forecaster whose stated probabilities are random and independent of each event's outcome, the share of 200 simulated claim sets whose interval lower end is above zero is at most 0.05 at each of 20, 50 and 100 claims, so the power-brier rates are not an artefact of the interval method.
- Metric: `/brier_null/by_n/100/excludes_zero_rate`.
- Expected: at or below 0.05 at n=100 (and at 20 and 50).
- Falsifier: excludes_zero_rate above 0.05 at any of the three sizes at seed 7.
- Command: `cargo run --locked -q --release -- brier-null --seed 7 --n 400 --sizes 20,50,100 --draws 200 --boot 1000 --out results/275-brier-null.json`
- Limit: draws come from one finite world and overlap; a statement about the simulated world and the Brier measure only, not any real claim set.

## 2026-11-16T17:05:59Z real-claims-first proposal:62a639b0be78e2cdd52f3523073dc288dc3a3727d9218b44732f5abe4f8eb364

Real input, first use. Source: the AI 2027 scenario (https://ai-2027.com, published 2025-04-03 as read from the page; the made_on date used for every claim). Sections named before being read for claims: the five dated sections of the scenario's narrative, "Mid 2025: Stumbling Agents", "Late 2025: The World's Most Expensive AI", "Early 2026: Coding Automation", "Mid 2026: China Wakes Up" and "Late 2026: AI Takes Some Jobs". Per the extraction rule in docs/PREREGISTRATION.md every sentence that asserts something will have happened by a date is entered; sentences that are narrated in the scenario's own past tense but state events in the 2025 to 2026 window count as dated claims, and a sentence with no resolution rule counts in not-checkable and is listed. Dates are read by the rule in the pre-registration (mid = 06-30, late = 12-31, early = 03-31). Resolution rules are written in a second dated section before any outcome is looked up. Scoring date 2026-11-16.

- Hypothesis: of the resolved dated claims extracted from these sections, the hit rate (share that happened on or before the stated date) is at most 0.5 at as-of 2026-11-16.
- Metric: `/groups/overall/hit_rate`.
- Expected: hit rate at most 0.5 (range 0.0 to 0.5); the count of resolved claims is reported beside it.
- Falsifier: a hit rate above 0.5 on the extracted resolved claims. If fewer than 5 claims resolve the result is inconclusive and the hypothesis is not scored either way.
- Command: `cargo run --locked -q --release -- score --claims claims/claims.tsv --as-of 2026-11-16 --seed 7 --resamples 2000 --out results/278-real-claims-first.json`
- Limit: a statement about the extracted dated claims of one source; no result says when AGI will arrive. Exploratory except where H2/H3 apply.

## 2026-11-16T17:06:52Z real-claims-first-rules proposal:62a639b0be78e2cdd52f3523073dc288dc3a3727d9218b44732f5abe4f8eb364

Rules written after reading the five named sections and before any outcome was looked up. made_on is 2025-04-03 for every claim. Extraction applies the pre-registered rule: a sentence enters the table if it asserts a dated, publicly checkable event that was not already true on 2025-04-03; every other asserting sentence is counted not-checkable (its item number in the numbered list read from the page is given).

Entered claims:
- `ai2027-flop-1e27` (capability, due 2025-12-31, item 10: "OpenBrain's latest public model—Agent-0—was trained with 10^27 FLOP."). Rule: a publicly released model whose training compute is estimated at 1e27 FLOP or more in the Epoch AI notable-models data, first documented on or before 2025-12-31.
- `ai2027-market-30pct` (economy, due 2026-12-31, item 28: "The stock market has gone up 30% in 2026, led by OpenBrain, Nvidia…"). Rule: the S&P 500 price return for calendar 2026 is 30% or more at the 2026-12-31 close. Not due at as-of 2026-11-16, so outcome `-`.
- `ai2027-dc-protest` (governance, due 2026-12-31, item 30: "…there is a 10,000 person anti-AI protest in DC."). Rule: a news report of an anti-AI protest in Washington DC with an attendance of 10,000 or more, dated on or before 2026-12-31.

Not checkable (counted, not scored): items 1, 3, 5 (already true on made_on or rejected by the parser as not a forecast: Operator, deep-research agents and a 200-dollar tier existed before 2025-04-03), 2, 4, 6, 7, 8, 9, 11 to 27 and 29, 31, 32 (descriptive, relative or about the scenario's fictional companies, or China's compute shares with no public measure; item 12 and 15 have no cheap public rule). Total not checkable: 29 of 32 listed items.

## 2026-11-16T17:07:14Z real-claims-first-lookup proposal:62a639b0be78e2cdd52f3523073dc288dc3a3727d9218b44732f5abe4f8eb364

Outcome lookups, rules unchanged. `ai2027-flop-1e27`: Epoch AI notable-models CSV (https://epoch.ai/data/notable_ai_models.csv, 1094 rows, latest publication date 2026-11-10): the largest estimate published on or before 2025-12-31 is 5.0e26 FLOP (Grok 4, 2025-07-09); the only entry at 9.4e26 (GPT-6 Astra, 2026-09-03) is later and below 1e27. No qualifying model: outcome `-`, overdue, counted as a miss. `ai2027-market-30pct` and `ai2027-dc-protest` are due 2026-12-31, after the as-of date, so outcome `-` (open); the protest lookup was one failed web search and no report was found, so it is not evidence either way. Expected consequence under the pre-registered stop rule: fewer than 5 resolved claims, result inconclusive.

## 2026-11-16T18:54:46Z slack-sensitivity proposal:0d1870a505249f636217fc4ab6a27ffb0a0e6985eb33c9e7c25f095a30ecca85

Motivation: the authors of the scenario say 2027 was a mode and not a median, so a dated claim can be scored under more than one reading of its date. This section fixes a SIMULATED check of how much the reading matters, before any real claim is scored under a window.

- Hypothesis: on the 400-claim simulated world (seed 7, calibrated forecaster, as-of 2026-01-01), widening the on-time window from the stated date to stated date plus 90 days turns between 20 and 40 miss verdicts into hits.
- Metric: `/sensitivity/by_slack_days/90/miss_to_hit_count`.
- Expected: at least 20 and at most 40 (about 30 expected from a delay uniform on 1 to 600 days).
- Falsifier: the count at 90 days slack below 20 or above 40 at seed 7, or the count not non-decreasing across slack 0, 30, 90, 180.
- Command: `cargo run --locked -q --release -- slack-sensitivity --seed 7 --n 400 --slacks 0,30,90,180 --as-of 2026-01-01 --out results/287-slack-sensitivity.json`
- Limit: a statement about one simulated world and the date-reading rule only; no real claim is scored.

## 2026-11-16T19:39:38Z aggregation-audit proposal:cfa866f28e0909f60388bef844657272574a62dde3e260b8558b4c91523dd5fd

Motivation: a headline hit rate can be pooled over claims or averaged over classes, and the two differ when classes have uneven counts. This section fixes a SIMULATED check of how far apart they sit on small draws, before any real claim set is aggregated both ways.

- Hypothesis: on the 400-claim simulated world (seed 7, calibrated forecaster, as-of 2026-01-01), for 200 draws of 15 resolved claims without replacement, the median over draws of the absolute difference between the pooled hit rate and the unweighted mean of per-class hit rates (classes present in the draw) is at least 0.02.
- Metric: `/aggregation/median_abs_gap`.
- Expected: at least 0.02 (range 0.02 to 0.2; about 0.04 expected from uneven class counts of about 2.5 claims per class).
- Falsifier: median_abs_gap below 0.02 at seed 7.
- Command: `cargo run --locked -q --release -- aggregation-audit --seed 7 --n 400 --pick 15 --draws 200 --as-of 2026-01-01 --out results/294-aggregation-audit.json`
- Limit: a statement about one simulated world and the aggregation unit only; no real claim is scored.

## 2026-11-18T22:07:01Z convention-audit proposal:ad382c57568b829baec43d42d1168122fdd2b70a8489f2dd02cb1f484ddc00ff

Motivation: a median slip can keep overdue never-happened claims as censored lower bounds or drop them, and the two conventions differ. This section fixes a SIMULATED check of how far apart they sit per class, before any real claim set is scored both ways.

- Hypothesis: on the 400-claim simulated world (seed 7, calibrated forecaster, as-of 2026-01-01), dropping overdue never-happened claims from the slip calculation lowers the capability-class median slip by at least 0.5 months relative to keeping them as censored lower bounds.
- Metric: `/by_class/capability/median_slip_gap_months` (keep-censored minus drop).
- Expected: at least 0.5 months (range 0.5 to 24).
- Falsifier: a capability-class gap below 0.5 months at seed 7.
- Command: `cargo run --locked -q --release -- convention-audit --seed 7 --n 400 --as-of 2026-01-01 --out results/448-censor-convention.json`
- Limit: a statement about one simulated world and the slip convention only; the share of never-happened claims in the simulation is whatever the generator produces, not a fixed 30 percent. No real claim is scored.

## 2026-11-18T22:28:17Z lead-audit-censored proposal:816798b410620be8a7bd56efd9dc014303747f41ab2f2e137e8ecba0495aca22

Motivation: the earlier lead-audit run placed every due date before its as-of date, so no claim stayed open. This section fixes a SIMULATED check with claims made 450 days before the as-of date, so long-lead due dates fall after it, and open claims are counted as right-censored rather than dropped.

- Hypothesis: on a 400-claim simulated world (seed 7, calibrated forecaster, claims made 450 days before the as-of date 2025-09-01), the share of claims resolved (hit or miss, not open) is lower for long-lead claims (lead over 365 days) than for short-lead ones by at least 0.2.
- Metric: `/long_minus_short_resolved_share` (long minus short).
- Expected: at most -0.2 (range -1.0 to -0.2).
- Falsifier: a long-minus-short resolved share above -0.2 at seed 7.
- Command: `cargo run --locked -q --release -- lead-audit-censored --seed 7 --n 400 --as-of 2025-09-01 --out results/452-lead-censored.json`
- Limit: a statement about one simulated world and the lead-time split only; the world differs from the 1100-day world used by other audits. No real claim is scored.

## 2026-11-18T22:39:12Z km-slip proposal:98cc4481bcc8b388ccc029925f1f939d0b78071cf0af3269bf07815fb55e072b

Motivation: a slip estimate that drops open claims is biased when events are still pending. This section fixes a SIMULATED check of how far a naive median over resolved long-lead claims sits from a Kaplan-Meier median that treats open and overdue-never-happened claims as right-censored at as-of minus due date.

- Hypothesis: on the 400-claim simulated world (seed 7, calibrated forecaster, claims made 450 days before the as-of date 2025-09-01), the naive median slip over resolved long-lead claims (lead over 365 days; occurred claims only) is lower than the censoring-aware Kaplan-Meier median slip by at least 0.5 months.
- Metric: `/bias/long_naive_minus_km_median_months` (naive minus censoring-aware).
- Expected: at most -0.5 months (range -24 to -0.5).
- Falsifier: a long-lead naive-minus-censoring-aware median above -0.5 months at seed 7. An undefined Kaplan-Meier median (survival never reaches one half) is reported as null and does not support the hypothesis.
- Command: `cargo run --locked -q --release -- km-slip --seed 7 --n 400 --as-of 2025-09-01 --out results/455-km-slip.json`
- Limit: a statement about one simulated world and the slip estimator only. No real claim is scored.

## 2026-11-18T22:43:04Z class-mix proposal:0486d1517f66ccd09d7b2a47d5d6c633783db97211de6c49e7758b842f578599

Motivation: a pooled median slip depends on which claim classes are in the sample. This section fixes a SIMULATED check of how far the pooled median moves when the capability share of the sample is set by resampling, before any real claim set is pooled.

- Hypothesis: on the 400-claim simulated world (seed 7, calibrated forecaster, as-of 2026-01-01), resampling claims with replacement (draws of 400, 200 per share, median over draws) so the capability-class share is 0.1, 0.5 or 0.9 moves the pooled median slip by at least 0.5 months between the lowest and highest share.
- Metric: `/mix_sensitivity/pooled_range_months` (max minus min of the per-share median over draws of the pooled median slip).
- Expected: at least 0.5 months (range 0.5 to 24).
- Falsifier: a range below 0.5 months at seed 7.
- Command: `cargo run --locked -q --release -- class-mix --seed 7 --n 400 --as-of 2026-01-01 --draws 200 --out results/456-class-mix.json`
- Limit: a statement about one simulated world and the pooling convention only; the generator assigns classes by index, so any class effect is whatever the generator produces. No real claim is scored.

## 2026-11-18T22:52:13Z pace-snapshots proposal:79d89e4d7845ff334ee8690a6ee5c9c3a8cda00e767571fbb21b645e8865b6f6

Motivation: a self-graded pace figure can move between snapshots only because more claims have resolved. This section fixes a SIMULATED check on one unchanging world scored at an early and a late as-of date.

- Hypothesis: on one fixed 400-claim simulated world (seed 7, calibrated forecaster, claims made 450 days before the late as-of date 2025-09-01), the naive median slip over resolved occurred claims at the late as-of date exceeds the same estimate at the early as-of date 2025-03-01 by at least 0.5 months, while the resolved share also rises, although the world is unchanged.
- Metric: `/pace_censoring/naive_late_minus_early_median_months` (late minus early).
- Expected: at least 0.5 months (range 0.5 to 24).
- Falsifier: a late-minus-early naive median slip below 0.5 months at seed 7, or a resolved share that does not rise between the two as-of dates.
- Command: `cargo run --locked -q --release -- pace-snapshots --seed 7 --n 400 --as-of-early 2025-03-01 --as-of 2025-09-01 --out results/458-pace-snapshots.json`
- Limit: a statement about one simulated world and the naive estimator only. No real claim is scored.

## 2026-11-18T23:22:20Z group-gap proposal:7cdebb032be5c14017e9f72ed10ac78acf8a4675430649d8b02933e62d6d687d

Motivation: a fast-versus-slow comparison of timing slip is only trustworthy if the instrument recovers a known gap when claims are still open. This section fixes a SIMULATED check with a planted gap.

- Hypothesis: in a simulated world (seed 7, 400 claims, half per group, claims made 450 days before the as-of date 2025-09-01) where the fast group's dates are planted 2.0 months too early (slip +2.0) and the slow group's 2.0 months too late (slip -2.0), the difference in median slip (fast minus slow) measured on resolved claims only recovers the planted 4.0-month gap to within 1.0 month in absolute error.
- Metric: `/groups/gap_recovery_error` (absolute months between measured and planted gap).
- Expected: at most 1.0 month (range 0.0 to 1.0).
- Falsifier: an absolute gap recovery error above 1.0 month at seed 7.
- Command: `cargo run --release --locked -- group-gap --seed 7 --n 400 --as-of 2025-09-01 --out results/464-group-gap.json`
- Limit: a statement about one simulated world and the resolved-only estimator. Claims whose event falls after the as-of date are open and dropped. No real claim or source is scored.

## 2026-11-18T23:51:52Z ceiling-doubling proposal:0f6f09116eba6da13b5db2b3bdc0e2ac7a417759e77ac136d97bb5b938f5bdcf

Motivation: a fitted doubling time depends on the task suite, so a suite that saturates can change a pace estimate with no change in capability. This section fixes a SIMULATED check of that bias with a known true doubling time.

- Hypothesis: in a simulated exponential horizon series with a true doubling time of 6 months (37 monthly points, log2 noise of half-width 0.1), a log-linear fit on observations clipped at a ceiling inside the observed range overstates the doubling time; the bias at the lowest cap level (0.25 of the top horizon) exceeds the uncapped bias by at least 1.0 months, and the uncapped fit has absolute bias under 0.1 months.
- Metric: `/metrics/doubling_time_bias_months_at_cap` (lowest-cap bias minus uncapped bias, months).
- Expected: at least 1.0 months (range 1.0 to 24).
- Falsifier: that value below 1.0 months at seed 7, or an uncapped absolute bias of 0.1 months or more.
- Command: `cargo run --locked --release -- ceiling-doubling --seed 7 --true-doubling-months 6 --caps 0.25,0.5,0.75,1.0 --out results/471-ceiling-doubling.json`
- Limit: a statement about one simulated series and an ordinary-least-squares log fit only. No real benchmark or measured horizon is scored.

## 2026-11-18T23:55:52Z revision-audit proposal:711e1d339063c21d3c80712e4ec1f516f6e6dee9523f9c924ffbe05e70bba747

Motivation: no source scores forecasters' own revisions against what resolved. This section fixes a SIMULATED check that the instrument separates an informative revision from a noise-only one.

- Hypothesis: in a simulated world (seed 7, 400 claims resolved by the as-of date, true outcome probability uniform on [0,1]) where the prior forecast is a weak noisy signal of that probability and the revised forecast sees a less noisy partial signal, the revised forecast's Brier skill against the coin exceeds the prior's by at least 0.05, a noise-only revision (prior plus independent noise) gains no more than 0.02, and an always-0.5 responder scores exactly 0.
- Metric: `/metrics/brier_skill_revised_minus_prior` (informative reviser).
- Expected: at least 0.05 (range 0.05 to 1.0).
- Falsifier: that value below 0.05 at seed 7, or the noise-only gain above 0.02, or the always-0.5 skill not equal to 0.
- Command: `cargo run --release --locked -- revision-audit --out results/472-revision-audit.json --seed 7`
- Limit: a statement about one simulated world. No real forecast or revision is scored.

## 2026-11-18T23:59:59Z suite-refresh proposal:960979eebbf016a4c1e62399d7348772fb299e99d30bf3e3ffe75ea886e2c9cc

Motivation: a doubling-time fit depends on the task suite, so a suite refresh that shifts the level can change a pace estimate with no change in capability. This section fixes a SIMULATED check with a known true doubling time.

- Hypothesis: in a simulated exponential horizon series (seed 7, 37 monthly points, true doubling time 6 months, log2 noise of half-width 0.1) with a suite refresh at month 18 that adds a step of 1.0 log2 units to every later observation, a naive ordinary-least-squares log fit understates the doubling time by at least 0.8 months (bias at most -0.8), while a fit with a step term at the refresh has absolute bias under 0.1 months.
- Metric: `/metrics/naive_bias_months`.
- Expected: at most -0.8 months (range -3.0 to -0.8).
- Falsifier: naive bias above -0.8 months at seed 7, or an absolute bias of 0.1 months or more for the step-term fit.
- Command: `cargo run --locked --release -- suite-refresh --seed 7 --true-doubling-months 6 --refresh-month 18 --step-log2 1.0 --out results/473-suite-refresh.json`
- Limit: a statement about one simulated series, a known refresh month and an ordinary-least-squares fit only. No real benchmark or measured horizon is scored.

## 2026-11-19T00:04:30Z pace-ratio-conventions proposal:85cc63d9835f4c99462fe6f077617a41355d200ee1dfa218e3b308606aa8e4d3

Motivation: published pace ratios for quantitative AI 2027 claims differ by update, so the ratio depends on which claims and which curve are counted. This section fixes a SIMULATED check of how far three counting conventions disagree when the true ratio is known.

- Hypothesis: on a simulated set of 40 quantitative claims (seed 7) with a known true pace ratio of 0.65, heavy-tailed per-claim ratios and 20% of claims graded against a wrong curve, the maximum minus minimum pace ratio across mean-of-ratios, ratio-of-sums and median-of-ratios is at least 0.10, and with no mis-graded claims the spread is smaller than with them.
- Metric: `/convention_spread/max_minus_min_pace_ratio`.
- Expected: at least 0.10 (range 0.10 to 1.0).
- Falsifier: that value below 0.10 at seed 7, or a spread with mis-grading that is not larger than the spread without it.
- Command: `cargo run --locked -q --release -- pace-ratio-conventions --seed 7 --n 40 --true-ratio 0.65 --misgrade-share 0.2 --out results/474-pace-ratio-conventions.json`
- Limit: a statement about one simulated claim set. No real claim, grading or source is scored.

## 2026-11-19T00:08:30Z fit-points proposal:e1e92e2ee2af450812ed8391fbbb5671d4032cc13c4253f730fc82bdaa803c4c

Motivation: a faster doubling-time estimate is said to rest on few recent points, so the error of a projected date should depend on how many points enter the fit. This section fixes a SIMULATED check with a known true doubling time.

- Hypothesis: in simulated monthly log2-horizon series (seed 7, 500 replicate series, true doubling time 6 months, triangular log2 noise of half-width 0.1, fit on the last n of 37 points, projection to month 60), the median absolute error of the projected date when fitting the last 6 points is at least twice the error when fitting the last 20 points.
- Metric: `/fit_points/projection_error_months_at_n6`.
- Expected: at least 2 times the n=20 error (range 0.5 to 12.0 months); 95% interval coverage at n=6 differs from 0.95.
- Falsifier: median absolute error at n=6 below twice that at n=20 at seed 7.
- Command: `cargo run --release --locked -- fit-points --out results/475-fit-points.json --seed 7`
- Limit: a statement about one simulated series family with correct exponential form and an ordinary-least-squares fit. Interval coverage is that of the 95% confidence interval for the fitted level at month 60, with Student t critical values. No real benchmark or measured horizon is scored.

## 2026-11-19T00:12:50Z snapshot-count proposal:07dc6863e74ea61e061d2d3727829c785c772308a0607fcc4593e5d471250f1c

Motivation: third-party trackers count confirmed predictions differently on different snapshots, so a hit count depends on the snapshot date and the counting method. This section fixes a SIMULATED check of which moves the count more.

- Hypothesis: in one simulated world of 400 claims (seed 7, claims made 2024-06-01, due dates spread over time), the spread in confirmed-claim count across four as-of snapshots six months apart (2025-01-01, 2025-07-01, 2026-01-01, 2026-07-01; strict rule, zero slack) exceeds the largest strict-versus-90-day-slack count difference at any single snapshot, so the snapshot date moves a tracker's count more than its counting convention does.
- Metric: `/snapshot_audit/count_spread_snapshot_only`.
- Expected: the ratio of count_spread_snapshot_only to count_spread_convention_only is at least 1.5 (range 1.5 to 20.0).
- Falsifier: that ratio below 1.0 at seed 7.
- Command: `cargo run --locked --release -- snapshot-count --seed 7 --n 400 --out results/476-snapshot-count.json`
- Limit: a statement about one simulated world. No real tracker, claim or grading is scored.

## 2026-11-19T00:34:00Z window-spread proposal:9f33c680b665742585039eb090235d9873bb7774756a56511551b03fea005e3f

Motivation: a measured doubling time is said to depend on the window chosen. This section fixes a SIMULATED check of how much the start year alone spreads the estimate when the truth is one constant doubling time.

- Hypothesis: on a simulated frontier with one constant true doubling time, noisy measurements and sparse early points, fitting doubling time from windows that start at four different years gives a range (max minus min) of at least 20 days across windows in the median seed, so a window-dependent doubling time alone is not evidence of acceleration.
- Metric: `/window_spread/doubling_days_range`.
- Expected: median over seeds of the range is at least 20 days (range 20 to 400).
- Falsifier: a median doubling-days range below 20 days across the four start windows.
- Command: `cargo run --release --locked -- window-spread --seed 7 --out results/481-window-spread.json`
- Limit: one simulated series family with correct exponential form and an ordinary-least-squares fit. No real benchmark or measured horizon is scored.

## 2026-11-19T00:38:12Z label-noise proposal:a2b0c0c2c445d0b144c6d9d86b1d245a8d6e90feb440d927d209e67d542a99fe

Motivation: METR names noise in human task-length estimates as the aspect it is least certain about, and critics say intervals treat human times as known. This section fixes a SIMULATED check of how noisy length labels bias a fitted horizon when the truth is known.

- Hypothesis: in a simulated suite (seed 7, 200 tasks log-uniform in length from 1 to 1000 minutes, true logistic slope 1.0 per natural-log unit of length, true 50% horizon 60 minutes, 500 replicates, one attempt per task), multiplicative log-normal noise on the length labels attenuates the fitted slope: the median log ratio of fitted to true 80% horizon at sigma 1.0 is at most -0.3, the fitted 50% horizon stays within 0.1 in absolute log ratio, and at sigma 0 the 80% log ratio is within 0.05 of zero.
- Metric: `/bias/h80_log_ratio_at_sigma_max`.
- Expected: at most -0.3 (range -2.0 to -0.3).
- Falsifier: median 80% log ratio above -0.3 at sigma 1.0, or an absolute 50% log ratio of 0.1 or more, or an absolute 80% log ratio of 0.05 or more at sigma 0.
- Command: `cargo run --release --locked -- label-noise --seed 7 --out results/482-label-noise.json`
- Limit: one simulated suite with a correct logistic form and a maximum-likelihood fit on the noisy labels. Sigma is in natural-log units of length. No real benchmark or measured horizon is scored.

## 2026-11-19T00:43:00Z difficulty-spread proposal:0e2bd5aa8198e0490d860cd44cfbf61783c05e9c23db2db14494b1e166cfb8f7

Motivation: a critique argues a typical 80% horizon is optimistic because hard tasks drag the average more than easy ones push it up. This section fixes a SIMULATED check of how far a two-parameter logistic fit's 50% and 80% horizons depart from the true marginal horizons when per-task difficulty varies around a log-linear function of length.

- Hypothesis: in a simulated suite (seed 7, 200 tasks log-uniform in length from 1 to 1000 minutes, log-odds slope 1.0 per natural-log unit of length, log-odds zero at 60 minutes, per-task difficulty offset N(0, tau^2) in log-odds, 500 replicates, one attempt per task), the median absolute log ratio of fitted to marginal 80% horizon is within 0.1 at tau 0 but at least 0.3 at tau 2, and the 50% horizon departs by less than the 80% one. Marginal horizons are computed by numerical integration of success probability over the difficulty offset.
- Metric: `/gap/fit_vs_marginal_log_ratio_at_tau_max`.
- Expected: absolute log ratio of fitted to marginal 80% horizon at tau 2 at least 0.3 (range 0.3 to 5.0).
- Falsifier: that value below 0.3, or the 50% horizon departing as much as the 80% one.
- Command: `cargo run --release --locked -- difficulty-spread --seed 7 --out results/483-difficulty-spread.json`
- Limit: one simulated suite with Gaussian difficulty offsets and a maximum-likelihood two-parameter fit. Tau is in log-odds units. No real benchmark or measured horizon is scored.

## 2026-11-19T00:52:04Z tasks-per-model proposal:cb8248a22ef0af6e918b2893de9e40f2bb8728e1e2492b10b5c35b3bbf89acc4

Motivation: a trend slope is only as stable as the number of tasks behind each model's horizon. This section fixes a SIMULATED sweep of tasks per model on a constant-exponential frontier with known doubling time.

- Hypothesis: with a true doubling time of 7 months, 8 models spaced 6 months apart, 200 replicates per setting, task lengths log-uniform from 1 to 1000 minutes, log-odds slope 1.0 per natural-log unit and one attempt per task, giving each model only 10 tasks makes the share of replicates with a wrong-sign or undefined fitted slope at least 0.05, while at 200 tasks per model that share is below 0.01, and the median absolute log error of the fitted doubling time at 10 tasks is at least 3 times that at 200 tasks. A model whose logistic fit is separable or has a non-decreasing success curve is dropped; a replicate with fewer than 3 usable models is undefined.
- Metric: `/wrong_sign_share_at_smallest_n`.
- Expected: at least 0.05 at 10 tasks per model (range 0.05 to 1.0).
- Falsifier: the share at 10 tasks below 0.05, or the share at 200 tasks at least 0.01.
- Command: `cargo run --release --locked -- tasks-per-model --seed 7 --out results/485-tasks-per-model.json`
- Limit: one simulated frontier with correct logistic form and a maximum-likelihood per-model fit followed by ordinary least squares on log horizon. No real benchmark or measured horizon is scored.
