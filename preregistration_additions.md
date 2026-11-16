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
