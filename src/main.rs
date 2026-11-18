use std::process::ExitCode;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use timelineaudit::{
    aggregation_audit, ceiling_doubling, class_mix, convention_audit, format_date, group_gap,
    group_json, iso_utc, json_escape, km_slip, lead_audit_censored, pace_snapshot, parse_claims,
    parse_date, power_brier, randomise_probs, revision_audit, score, selection_audit,
    slack_sensitivity, synth_claims, synth_claims_made_on, Forecaster, Pick, Status,
};

const USAGE: &str = "timelineaudit: score dated claims from AGI-timeline scenarios against what happened

USAGE:
  timelineaudit baselines --out <results.json> [--as-of 2026-01-01] [--seed 7] [--n 400] [--resamples 2000]
      Score five simulated forecasters on one simulated world: one that knows
      each event's true chance, three that know nothing (always_yes,
      always_no, coin) and one that states every date too early. Only the
      first may show Brier skill, and only the last may show extra slip.

  timelineaudit selection-audit --out <results.json> [--as-of 2026-01-01] [--seed 7] [--n 400] [--pick 15] [--draws 200]
      SIMULATED. Draw subsets of claims from one simulated world by a random
      rule, a hit-favouring rule and a miss-favouring rule, and report the
      median absolute shift of median slip and hit rate from the full set.

  timelineaudit power-brier --out <results.json> [--as-of 2026-01-01] [--seed 7] [--n 400] [--sizes 20,50,100] [--draws 200] [--boot 1000]
      SIMULATED. Draw claim sets of each size from one simulated world and
      report how often the bootstrap interval on Brier skill against the coin
      has a lower end above zero.

  timelineaudit aggregation-audit --out <results.json> [--as-of 2026-01-01] [--seed 7] [--n 400] [--pick 15] [--draws 200]
      SIMULATED. Draw small sets of resolved claims and report the median
      absolute gap between the claim-pooled hit rate and the mean of
      per-class hit rates.

  timelineaudit convention-audit --out <results.json> [--as-of 2026-01-01] [--seed 7] [--n 400]
      SIMULATED. Per class, compare the median slip with overdue
      never-happened claims kept as lower bounds against the median with
      them dropped.

  timelineaudit class-mix --out <results.json> [--as-of 2026-01-01] [--seed 7] [--n 400] [--draws 200]
      SIMULATED. Resample claims so the capability share is 0.1, 0.5 and 0.9
      and report how far the pooled median slip moves across shares.

  timelineaudit pace-snapshots --out <results.json> [--as-of-early 2025-03-01] [--as-of 2025-09-01] [--seed 7] [--n 400]
      SIMULATED. Score one fixed world (claims made 450 days before --as-of)
      at an early and a late as-of date; report resolved share and naive
      median slip at each.

  timelineaudit revision-audit --out <results.json> [--seed 7] [--n 400]
      SIMULATED. Brier skill of a prior forecast, an informative revision, a
      noise-only revision and an always-0.5 responder on resolved claims.

  timelineaudit ceiling-doubling --out <results.json> [--seed 7] [--true-doubling-months 6] [--caps 0.25,0.5,0.75,1.0]
      SIMULATED. Fit a doubling time to a noisy exponential horizon series
      clipped at suite ceilings (fractions of the top horizon); report the
      bias in months per cap and against the uncapped fit.

  timelineaudit group-gap --out <results.json> [--as-of 2025-09-01] [--seed 7] [--n 400]
      SIMULATED. A fast group planted 2.0 months late-landing and a slow group
      2.0 months early-landing, claims made 450 days before --as-of, open
      claims dropped; report how well the measured slip gap recovers 4.0.

  timelineaudit km-slip --out <results.json> [--as-of 2025-09-01] [--seed 7] [--n 400]
  timelineaudit lead-audit-censored --out <results.json> [--as-of 2025-09-01] [--seed 7] [--n 400]
      SIMULATED. Claims made 450 days before the as-of date; report resolved
      share and median slip for short-lead and long-lead claims, open claims
      counted as right-censored.

  timelineaudit slack-sensitivity --out <results.json> [--as-of 2026-01-01] [--seed 7] [--n 400] [--slacks 0,30,90,180]
      SIMULATED. Score the same claims with the stated date and with the
      stated date plus each slack in days, and count the verdicts that flip
      from miss to hit.

  timelineaudit brier-null --out <results.json> [--as-of 2026-01-01] [--seed 7] [--n 400] [--sizes 20,50,100] [--draws 200] [--boot 1000]
      SIMULATED. Like power-brier, but every stated probability is a uniform
      random draw independent of the outcome; reports how often the interval
      on Brier skill against the coin still has a lower end above zero.

  timelineaudit check --claims <claims.tsv> --as-of <YYYY-MM-DD>
      Validate the claim table and print how many claims are hits, misses,
      open and unclear. Writes nothing.

  timelineaudit score --claims <claims.tsv> --as-of <YYYY-MM-DD> --out <results.json>
                      [--seed 7] [--resamples 2000] [--license <licence of the table>]
      Score the claim table. Columns, tab-separated:
      id source class made_on due_on prob outcome url
      (see docs/PREREGISTRATION.md). --as-of is required: the scoring date is
      never taken from the clock, so the result reproduces.
";

struct Args(Vec<String>);

impl Args {
    fn value(&self, flag: &str) -> Option<&str> {
        let i = self.0.iter().position(|a| a == flag)?;
        self.0.get(i + 1).map(String::as_str)
    }
    fn required(&self, flag: &str) -> Result<&str, String> {
        self.value(flag)
            .ok_or_else(|| format!("{flag} is required\n\n{USAGE}"))
    }
    fn number(&self, flag: &str, default: u64) -> Result<u64, String> {
        match self.value(flag) {
            None => Ok(default),
            Some(v) => v
                .parse()
                .map_err(|_| format!("{flag} needs a whole number, got `{v}`")),
        }
    }
}

fn provenance(started: u64, seed: u64, wall_s: f64, sources: &str) -> String {
    let command = std::env::args().collect::<Vec<_>>().join(" ");
    format!(
        "  \"provenance\": {{\"started_at\": \"{}\", \"seeds\": [{}], \"command\": \"{}\", \"hardware\": \"{} {}\", \"wall_s\": {:.3}, \"data_sources\": {}}}",
        iso_utc(started),
        seed,
        json_escape(&command),
        std::env::consts::OS,
        std::env::consts::ARCH,
        wall_s,
        sources
    )
}

fn read_claims(args: &Args) -> Result<(Vec<timelineaudit::Claim>, i64, String), String> {
    let path = args.required("--claims")?.to_string();
    let as_of = parse_date(args.required("--as-of")?)?;
    let text = std::fs::read_to_string(&path).map_err(|e| format!("read {path}: {e}"))?;
    let claims = parse_claims(&text, as_of).map_err(|e| format!("{path}: {e}"))?;
    Ok((claims, as_of, path))
}

fn run() -> Result<(), String> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = argv.first().cloned() else {
        return Err(USAGE.into());
    };
    let args = Args(argv);

    if cmd == "check" {
        let (claims, as_of, path) = read_claims(&args)?;
        let count = |s: Status| claims.iter().filter(|c| c.status(as_of) == s).count();
        println!(
            "{path}: {} claims as of {}: {} hit, {} miss, {} open, {} unclear",
            claims.len(),
            format_date(as_of),
            count(Status::Hit),
            count(Status::Miss),
            count(Status::Open),
            count(Status::Unclear)
        );
        return Ok(());
    }

    let out = args.required("--out")?.to_string();
    let seed = args.number("--seed", 7)?;
    let resamples = args.number("--resamples", 2000)? as usize;
    let started = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let clock = Instant::now();

    let body = match cmd.as_str() {
        "baselines" => {
            let n = args.number("--n", 400)? as usize;
            let as_of = parse_date(args.value("--as-of").unwrap_or("2026-01-01"))?;
            let mut blocks = Vec::new();
            for f in Forecaster::ALL {
                // One seed for every forecaster: they face the same world.
                let claims = synth_claims(f, n, seed, as_of);
                let overall = &score(&claims, as_of, resamples, seed)[0];
                blocks.push(format!("    \"{}\": {}", f.name(), group_json(overall)));
            }
            format!(
                "  \"kind\": \"baselines\",\n  \"simulated\": true,\n  \"as_of\": \"{}\",\n  \"n_claims\": {},\n  \"resamples\": {},\n  \"forecasters\": {{\n{}\n  }},\n{}",
                format_date(as_of),
                n,
                resamples,
                blocks.join(",\n"),
                provenance(
                    started,
                    seed,
                    clock.elapsed().as_secs_f64(),
                    "[{\"url\": \"synthetic: generated by this command from the seed\", \"license\": \"MIT\"}]"
                )
            )
        }
        "selection-audit" => {
            let n = args.number("--n", 400)? as usize;
            let pick = args.number("--pick", 15)? as usize;
            let draws = args.number("--draws", 200)? as usize;
            let as_of = parse_date(args.value("--as-of").unwrap_or("2026-01-01"))?;
            let claims = synth_claims(Forecaster::Calibrated, n, seed, as_of);
            let mut blocks = Vec::new();
            for rule in Pick::ALL {
                let sh =
                    selection_audit(&claims, as_of, rule, pick, draws, seed).ok_or_else(|| {
                        format!("{}: fewer than {pick} claims to pick from", rule.name())
                    })?;
                blocks.push(format!(
                    "    \"{}\": {{\"median_abs_slip_shift_months\": {:.6}, \"median_abs_hit_rate_shift\": {:.6}}}",
                    rule.name(),
                    sh.median_abs_slip_shift_months,
                    sh.median_abs_hit_rate_shift
                ));
            }
            format!(
                "  \"kind\": \"selection-audit\",\n  \"simulated\": true,\n  \"as_of\": \"{}\",\n  \"n_claims\": {},\n  \"pick\": {},\n  \"draws\": {},\n  \"selection\": {{\n{}\n  }},\n{}",
                format_date(as_of),
                n,
                pick,
                draws,
                blocks.join(",\n"),
                provenance(
                    started,
                    seed,
                    clock.elapsed().as_secs_f64(),
                    "[{\"url\": \"synthetic: generated by this command from the seed\", \"license\": \"MIT\"}]"
                )
            )
        }
        "slack-sensitivity" => {
            let n = args.number("--n", 400)? as usize;
            let as_of = parse_date(args.value("--as-of").unwrap_or("2026-01-01"))?;
            let slacks: Vec<i64> = args
                .value("--slacks")
                .unwrap_or("0,30,90,180")
                .split(',')
                .map(|s| {
                    s.trim()
                        .parse()
                        .map_err(|_| format!("--slacks needs whole numbers, got `{s}`"))
                })
                .collect::<Result<_, _>>()?;
            let claims = synth_claims(Forecaster::Calibrated, n, seed, as_of);
            let blocks: Vec<String> = slack_sensitivity(&claims, as_of, &slacks)
                .iter()
                .map(|r| {
                    format!(
                        "      \"{}\": {{\"hits\": {}, \"misses\": {}, \"open\": {}, \"hit_rate\": {:.6}, \"miss_to_hit_count\": {}, \"miss_to_open_count\": {}}}",
                        r.slack_days, r.hits, r.misses, r.open, r.hit_rate, r.miss_to_hit_count, r.miss_to_open_count
                    )
                })
                .collect();
            format!(
                "  \"kind\": \"slack-sensitivity\",\n  \"simulated\": true,\n  \"as_of\": \"{}\",\n  \"n_claims\": {},\n  \"sensitivity\": {{\n    \"by_slack_days\": {{\n{}\n    }}\n  }},\n{}",
                format_date(as_of),
                n,
                blocks.join(",\n"),
                provenance(
                    started,
                    seed,
                    clock.elapsed().as_secs_f64(),
                    "[{\"url\": \"synthetic: generated by this command from the seed\", \"license\": \"MIT\"}]"
                )
            )
        }
        "aggregation-audit" => {
            let n = args.number("--n", 400)? as usize;
            let pick = args.number("--pick", 15)? as usize;
            let draws = args.number("--draws", 200)? as usize;
            let as_of = parse_date(args.value("--as-of").unwrap_or("2026-01-01"))?;
            let claims = synth_claims(Forecaster::Calibrated, n, seed, as_of);
            let gap = aggregation_audit(&claims, as_of, pick, draws, seed)
                .ok_or("fewer resolved claims than --pick")?;
            format!(
                "  \"kind\": \"aggregation-audit\",\n  \"simulated\": true,\n  \"as_of\": \"{}\",\n  \"n_claims\": {},\n  \"pick\": {},\n  \"draws\": {},\n  \"aggregation\": {{\"median_abs_gap\": {:.6}}},\n{}",
                format_date(as_of),
                n,
                pick,
                draws,
                gap,
                provenance(
                    started,
                    seed,
                    clock.elapsed().as_secs_f64(),
                    "[{\"url\": \"synthetic: generated by this command from the seed\", \"license\": \"MIT\"}]"
                )
            )
        }
        "convention-audit" => {
            let n = args.number("--n", 400)? as usize;
            let as_of = parse_date(args.value("--as-of").unwrap_or("2026-01-01"))?;
            let claims = synth_claims(Forecaster::Calibrated, n, seed, as_of);
            let num = |v: f64| {
                if v.is_finite() {
                    format!("{v:.6}")
                } else {
                    "null".to_string()
                }
            };
            let blocks: Vec<String> = convention_audit(&claims, as_of)
                .iter()
                .map(|r| {
                    format!(
                        "    \"{}\": {{\"resolved\": {}, \"censored\": {}, \"keep_median_slip_months\": {}, \"drop_median_slip_months\": {}, \"median_slip_gap_months\": {}}}",
                        r.class,
                        r.resolved,
                        r.censored,
                        num(r.keep_median_months),
                        num(r.drop_median_months),
                        num(r.gap_months)
                    )
                })
                .collect();
            format!(
                "  \"kind\": \"convention-audit\",\n  \"simulated\": true,\n  \"as_of\": \"{}\",\n  \"n_claims\": {},\n  \"by_class\": {{\n{}\n  }},\n{}",
                format_date(as_of),
                n,
                blocks.join(",\n"),
                provenance(
                    started,
                    seed,
                    clock.elapsed().as_secs_f64(),
                    "[{\"url\": \"synthetic: generated by this command from the seed\", \"license\": \"MIT\"}]"
                )
            )
        }
        "class-mix" => {
            let n = args.number("--n", 400)? as usize;
            let draws = args.number("--draws", 200)? as usize;
            let as_of = parse_date(args.value("--as-of").unwrap_or("2026-01-01"))?;
            let claims = synth_claims(Forecaster::Calibrated, n, seed, as_of);
            let rows = class_mix(&claims, as_of, &[0.1, 0.5, 0.9], draws, seed)
                .ok_or("a class pool has no claims with a slip")?;
            let lo = rows.iter().map(|r| r.1).fold(f64::INFINITY, f64::min);
            let hi = rows.iter().map(|r| r.1).fold(f64::NEG_INFINITY, f64::max);
            let blocks: Vec<String> = rows
                .iter()
                .map(|(s, m)| format!("      \"{s}\": {m:.6}"))
                .collect();
            format!(
                "  \"kind\": \"class-mix\",\n  \"simulated\": true,\n  \"as_of\": \"{}\",\n  \"n_claims\": {},\n  \"draws\": {},\n  \"mix_sensitivity\": {{\n    \"pooled_range_months\": {:.6},\n    \"median_slip_by_capability_share\": {{\n{}\n    }}\n  }},\n{}",
                format_date(as_of),
                n,
                draws,
                hi - lo,
                blocks.join(",\n"),
                provenance(
                    started,
                    seed,
                    clock.elapsed().as_secs_f64(),
                    "[{\"url\": \"synthetic: generated by this command from the seed\", \"license\": \"MIT\"}]"
                )
            )
        }
        "pace-snapshots" => {
            let n = args.number("--n", 400)? as usize;
            let late = parse_date(args.value("--as-of").unwrap_or("2025-09-01"))?;
            let early = parse_date(args.value("--as-of-early").unwrap_or("2025-03-01"))?;
            let made_on = late - 450;
            let num = |v: f64| {
                if v.is_finite() {
                    format!("{v:.6}")
                } else {
                    "null".to_string()
                }
            };
            let e = pace_snapshot(
                &synth_claims_made_on(Forecaster::Calibrated, n, seed, early, made_on),
                early,
            );
            let l = pace_snapshot(
                &synth_claims_made_on(Forecaster::Calibrated, n, seed, late, made_on),
                late,
            );
            let snap = |name: &str, d: i64, s: &timelineaudit::PaceSnapshot| {
                format!(
                    "    \"{}\": {{\"as_of\": \"{}\", \"claims\": {}, \"events\": {}, \"resolved_share\": {}, \"naive_median_months\": {}}}",
                    name,
                    format_date(d),
                    s.claims,
                    s.events,
                    num(s.resolved_share),
                    num(s.naive_median_months)
                )
            };
            format!(
                "  \"kind\": \"pace-snapshots\",\n  \"simulated\": true,\n  \"n_claims\": {},\n  \"made_days_before_as_of\": 450,\n  \"pace_censoring\": {{\"naive_late_minus_early_median_months\": {}, \"resolved_share_late_minus_early\": {}}},\n  \"snapshots\": {{\n{},\n{}\n  }},\n{}",
                n,
                num(l.naive_median_months - e.naive_median_months),
                num(l.resolved_share - e.resolved_share),
                snap("early", early, &e),
                snap("late", late, &l),
                provenance(
                    started,
                    seed,
                    clock.elapsed().as_secs_f64(),
                    "[{\"url\": \"synthetic: generated by this command from the seed\", \"license\": \"MIT\"}]"
                )
            )
        }
        "revision-audit" => {
            let n = args.number("--n", 400)? as usize;
            if n == 0 {
                return Err("--n must be positive".into());
            }
            let r = revision_audit(seed, n);
            format!(
                "  \"kind\": \"revision-audit\",\n  \"simulated\": true,\n  \"n_claims\": {},\n  \"metrics\": {{\"brier_skill_revised_minus_prior\": {:.6}, \"brier_skill_noise_revised_minus_prior\": {:.6}, \"brier_skill_prior\": {:.6}, \"brier_skill_revised\": {:.6}, \"brier_skill_noise_revised\": {:.6}, \"brier_skill_always_half\": {:.6}}},\n{}",
                r.n,
                r.revised_skill - r.prior_skill,
                r.noise_revised_skill - r.prior_skill,
                r.prior_skill,
                r.revised_skill,
                r.noise_revised_skill,
                r.always_half_skill,
                provenance(
                    started,
                    seed,
                    clock.elapsed().as_secs_f64(),
                    "[{\"url\": \"synthetic: generated by this command from the seed\", \"license\": \"MIT\"}]"
                )
            )
        }
        "ceiling-doubling" => {
            let td: f64 = args
                .value("--true-doubling-months")
                .unwrap_or("6")
                .parse()
                .map_err(|_| "--true-doubling-months needs a number".to_string())?;
            let caps = args
                .value("--caps")
                .unwrap_or("0.25,0.5,0.75,1.0")
                .split(',')
                .map(|c| {
                    c.trim()
                        .parse::<f64>()
                        .map_err(|_| format!("bad cap `{c}`"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            if caps.is_empty() || caps.iter().any(|c| !(*c > 0.0 && *c <= 1.0)) {
                return Err("--caps needs fractions in (0, 1]".into());
            }
            let r = ceiling_doubling(seed, td, &caps);
            let lowest = r
                .fits
                .iter()
                .min_by(|a, b| a.cap_fraction.total_cmp(&b.cap_fraction))
                .expect("caps is not empty");
            let per_cap = r
                .fits
                .iter()
                .map(|f| {
                    format!(
                        "{{\"cap_fraction\": {}, \"capped_points\": {}, \"fitted_doubling_months\": {:.6}, \"bias_months\": {:.6}}}",
                        f.cap_fraction, f.capped_points, f.fitted_doubling_months, f.bias_months
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "  \"kind\": \"ceiling-doubling\",\n  \"simulated\": true,\n  \"true_doubling_months\": {},\n  \"n_points\": 37,\n  \"metrics\": {{\"doubling_time_bias_months_at_cap\": {:.6}, \"lowest_cap_fraction\": {}, \"lowest_cap_bias_months\": {:.6}, \"uncapped_bias_months\": {:.6}, \"uncapped_doubling_months\": {:.6}}},\n  \"per_cap\": [{}],\n{}",
                r.true_doubling_months,
                lowest.bias_months - r.uncapped_bias_months,
                lowest.cap_fraction,
                lowest.bias_months,
                r.uncapped_bias_months,
                r.uncapped_doubling_months,
                per_cap,
                provenance(
                    started,
                    seed,
                    clock.elapsed().as_secs_f64(),
                    "[{\"url\": \"synthetic: generated by this command from the seed\", \"license\": \"MIT\"}]"
                )
            )
        }
        "group-gap" => {
            let n = args.number("--n", 400)? as usize;
            let as_of = parse_date(args.value("--as-of").unwrap_or("2025-09-01"))?;
            let g = group_gap(n, seed, as_of, 450, 2.0);
            let num = |v: f64| {
                if v.is_finite() {
                    format!("{v:.6}")
                } else {
                    "null".to_string()
                }
            };
            format!(
                "  \"kind\": \"group-gap\",\n  \"simulated\": true,\n  \"as_of\": \"{}\",\n  \"n_claims\": {},\n  \"made_days_before_as_of\": 450,\n  \"groups\": {{\"planted_gap_months\": {}, \"measured_gap_months\": {}, \"gap_recovery_error\": {}, \"fast\": {{\"resolved\": {}, \"median_slip_months\": {}}}, \"slow\": {{\"resolved\": {}, \"median_slip_months\": {}}}}},\n{}",
                format_date(as_of),
                n,
                num(g.planted_gap_months),
                num(g.measured_gap_months),
                num(g.recovery_error_months),
                g.fast_resolved,
                num(g.fast_median_months),
                g.slow_resolved,
                num(g.slow_median_months),
                provenance(
                    started,
                    seed,
                    clock.elapsed().as_secs_f64(),
                    "[{\"url\": \"synthetic: generated by this command from the seed\", \"license\": \"MIT\"}]"
                )
            )
        }
        "km-slip" => {
            let n = args.number("--n", 400)? as usize;
            let as_of = parse_date(args.value("--as-of").unwrap_or("2025-09-01"))?;
            let claims = synth_claims_made_on(Forecaster::Calibrated, n, seed, as_of, as_of - 450);
            let num = |v: f64| {
                if v.is_finite() {
                    format!("{v:.6}")
                } else {
                    "null".to_string()
                }
            };
            let rows = km_slip(&claims, as_of);
            let diff = rows[1].naive_median_months - rows[1].km_median_months;
            let blocks: Vec<String> = rows
                .iter()
                .map(|r| {
                    format!(
                        "    \"{}\": {{\"claims\": {}, \"events\": {}, \"censored\": {}, \"naive_median_months\": {}, \"km_median_months\": {}}}",
                        r.bin,
                        r.claims,
                        r.events,
                        r.censored,
                        num(r.naive_median_months),
                        num(r.km_median_months)
                    )
                })
                .collect();
            format!(
                "  \"kind\": \"km-slip\",\n  \"simulated\": true,\n  \"as_of\": \"{}\",\n  \"n_claims\": {},\n  \"made_days_before_as_of\": 450,\n  \"bias\": {{\"long_naive_minus_km_median_months\": {}}},\n  \"by_lead\": {{\n{}\n  }},\n{}",
                format_date(as_of),
                n,
                num(diff),
                blocks.join(",\n"),
                provenance(
                    started,
                    seed,
                    clock.elapsed().as_secs_f64(),
                    "[{\"url\": \"synthetic: generated by this command from the seed\", \"license\": \"MIT\"}]"
                )
            )
        }
        "lead-audit-censored" => {
            let n = args.number("--n", 400)? as usize;
            let as_of = parse_date(args.value("--as-of").unwrap_or("2025-09-01"))?;
            let claims = synth_claims_made_on(Forecaster::Calibrated, n, seed, as_of, as_of - 450);
            let num = |v: f64| {
                if v.is_finite() {
                    format!("{v:.6}")
                } else {
                    "null".to_string()
                }
            };
            let rows = lead_audit_censored(&claims, as_of);
            let diff = rows[1].resolved_share - rows[0].resolved_share;
            let blocks: Vec<String> = rows
                .iter()
                .map(|r| {
                    format!(
                        "    \"{}\": {{\"claims\": {}, \"open\": {}, \"resolved_share\": {}, \"censored\": {}, \"median_slip_months\": {}}}",
                        r.bin,
                        r.claims,
                        r.open,
                        num(r.resolved_share),
                        r.censored,
                        num(r.median_slip_months)
                    )
                })
                .collect();
            format!(
                "  \"kind\": \"lead-audit-censored\",\n  \"simulated\": true,\n  \"as_of\": \"{}\",\n  \"n_claims\": {},\n  \"made_days_before_as_of\": 450,\n  \"long_minus_short_resolved_share\": {},\n  \"by_lead\": {{\n{}\n  }},\n{}",
                format_date(as_of),
                n,
                num(diff),
                blocks.join(",\n"),
                provenance(
                    started,
                    seed,
                    clock.elapsed().as_secs_f64(),
                    "[{\"url\": \"synthetic: generated by this command from the seed\", \"license\": \"MIT\"}]"
                )
            )
        }
        "power-brier" => {
            let n = args.number("--n", 400)? as usize;
            let draws = args.number("--draws", 200)? as usize;
            let boot = args.number("--boot", 1000)? as usize;
            let as_of = parse_date(args.value("--as-of").unwrap_or("2026-01-01"))?;
            let sizes: Vec<usize> = args
                .value("--sizes")
                .unwrap_or("20,50,100")
                .split(',')
                .map(|s| {
                    s.trim()
                        .parse()
                        .map_err(|_| format!("--sizes needs whole numbers, got `{s}`"))
                })
                .collect::<Result<_, _>>()?;
            let claims = synth_claims(Forecaster::Calibrated, n, seed, as_of);
            let mut blocks = Vec::new();
            for &size in &sizes {
                let p = power_brier(&claims, as_of, size, draws, boot, seed).ok_or_else(|| {
                    format!("size {size}: fewer than {size} resolved claims to draw from")
                })?;
                blocks.push(format!(
                    "      \"{}\": {{\"excludes_zero_rate\": {:.6}, \"median_skill\": {:.6}}}",
                    size, p.excludes_zero_rate, p.median_skill
                ));
            }
            format!(
                "  \"kind\": \"power-brier\",\n  \"simulated\": true,\n  \"as_of\": \"{}\",\n  \"n_claims\": {},\n  \"draws\": {},\n  \"boot\": {},\n  \"power_brier\": {{\n    \"by_n\": {{\n{}\n    }}\n  }},\n{}",
                format_date(as_of),
                n,
                draws,
                boot,
                blocks.join(",\n"),
                provenance(
                    started,
                    seed,
                    clock.elapsed().as_secs_f64(),
                    "[{\"url\": \"synthetic: generated by this command from the seed\", \"license\": \"MIT\"}]"
                )
            )
        }
        "brier-null" => {
            let n = args.number("--n", 400)? as usize;
            let draws = args.number("--draws", 200)? as usize;
            let boot = args.number("--boot", 1000)? as usize;
            let as_of = parse_date(args.value("--as-of").unwrap_or("2026-01-01"))?;
            let sizes: Vec<usize> = args
                .value("--sizes")
                .unwrap_or("20,50,100")
                .split(',')
                .map(|s| {
                    s.trim()
                        .parse()
                        .map_err(|_| format!("--sizes needs whole numbers, got `{s}`"))
                })
                .collect::<Result<_, _>>()?;
            let claims =
                randomise_probs(&synth_claims(Forecaster::Calibrated, n, seed, as_of), seed);
            let mut blocks = Vec::new();
            for &size in &sizes {
                let p = power_brier(&claims, as_of, size, draws, boot, seed).ok_or_else(|| {
                    format!("size {size}: fewer than {size} resolved claims to draw from")
                })?;
                blocks.push(format!(
                    "      \"{}\": {{\"excludes_zero_rate\": {:.6}, \"median_skill\": {:.6}}}",
                    size, p.excludes_zero_rate, p.median_skill
                ));
            }
            format!(
                "  \"kind\": \"brier-null\",\n  \"simulated\": true,\n  \"as_of\": \"{}\",\n  \"n_claims\": {},\n  \"draws\": {},\n  \"boot\": {},\n  \"brier_null\": {{\n    \"by_n\": {{\n{}\n    }}\n  }},\n{}",
                format_date(as_of),
                n,
                draws,
                boot,
                blocks.join(",\n"),
                provenance(
                    started,
                    seed,
                    clock.elapsed().as_secs_f64(),
                    "[{\"url\": \"synthetic: generated by this command from the seed\", \"license\": \"MIT\"}]"
                )
            )
        }
        "score" => {
            let (claims, as_of, path) = read_claims(&args)?;
            if claims.is_empty() {
                return Err(format!("{path}: no claims to score"));
            }
            let groups = score(&claims, as_of, resamples, seed);
            let blocks: Vec<String> = groups
                .iter()
                .map(|g| format!("    \"{}\": {}", json_escape(&g.name), group_json(g)))
                .collect();
            let license = args.value("--license").unwrap_or("see each claim's source");
            let mut urls: Vec<&str> = claims.iter().map(|c| c.url.as_str()).collect();
            urls.sort_unstable();
            urls.dedup();
            let sources: Vec<String> = urls
                .iter()
                .map(|u| {
                    format!(
                        "{{\"url\": \"{}\", \"license\": \"{}\"}}",
                        json_escape(u),
                        json_escape(license)
                    )
                })
                .collect();
            format!(
                "  \"kind\": \"score\",\n  \"simulated\": false,\n  \"claims_file\": \"{}\",\n  \"as_of\": \"{}\",\n  \"n_claims\": {},\n  \"resamples\": {},\n  \"groups\": {{\n{}\n  }},\n{}",
                json_escape(&path),
                format_date(as_of),
                claims.len(),
                resamples,
                blocks.join(",\n"),
                provenance(
                    started,
                    seed,
                    clock.elapsed().as_secs_f64(),
                    &format!("[{}]", sources.join(", "))
                )
            )
        }
        other => return Err(format!("unknown command `{other}`\n\n{USAGE}")),
    };

    if std::path::Path::new(&out).exists() {
        return Err(format!(
            "{out} already exists; results files are never overwritten"
        ));
    }
    if let Some(dir) = std::path::Path::new(&out).parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
        }
    }
    std::fs::write(&out, format!("{{\n{body}\n}}\n")).map_err(|e| format!("write {out}: {e}"))?;
    println!("wrote {out}");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}
