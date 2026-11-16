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
