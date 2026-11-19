//! timelineaudit: how have the dated, checkable claims in AGI-timeline
//! scenarios held up against what happened?
//!
//! A scenario that ends in 2027 cannot be checked before 2027. The claims it
//! makes on the way can. Each row of the claim table is one statement of the
//! form "X will have happened by date D", taken from a published source, with
//! the date the event was observed (if it has been). Three scores come out:
//!
//! - hit rate: the share of resolved claims whose event happened by its date;
//! - timing slip: months between the stated date and the observed one, with
//!   events that have not happened yet counted as a lower bound;
//! - Brier skill: for claims that carried a probability, squared error
//!   against a forecaster that says 0.5 to everything.
//!
//! Every score is audited against forecasters that know nothing (always yes,
//! always no, a coin) and one that states every date too early.
//!
//! No dependencies: dates, statistics, the PRNG and the JSON writer are all in
//! this file so the component builds offline.

use std::collections::BTreeSet;
use std::fmt::Write as _;

pub const DAYS_PER_MONTH: f64 = 30.4375;
/// Claims due within this many days of being made count as short-lead.
pub const SHORT_LEAD_DAYS: i64 = 365;
pub const CLASSES: [&str; 6] = [
    "capability",
    "deployment",
    "economy",
    "governance",
    "safety",
    "other",
];

// ---------------------------------------------------------------- dates

/// Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm).
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// Parse `YYYY-MM-DD` into days since the epoch. Rejects dates that do not
/// exist (2026-02-30).
pub fn parse_date(s: &str) -> Result<i64, String> {
    let bad = || format!("`{s}` is not a YYYY-MM-DD date");
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return Err(bad());
    }
    let y: i64 = parts[0].parse().map_err(|_| bad())?;
    let m: i64 = parts[1].parse().map_err(|_| bad())?;
    let d: i64 = parts[2].parse().map_err(|_| bad())?;
    let days = days_from_civil(y, m, d);
    if civil_from_days(days) != (y, m, d) {
        return Err(bad());
    }
    Ok(days)
}

pub fn format_date(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

// ---------------------------------------------------------------- claims

/// What the public record says about the event of a claim.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    /// The event was observed on this day.
    Occurred(i64),
    /// The event has not been observed as of the scoring date.
    NotYet,
    /// The public record does not settle it. Counted, never scored.
    Unclear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Hit,
    Miss,
    Open,
    Unclear,
}

/// One row of the claim table: "the event will have happened by `due_on`".
///
/// Tab-separated columns:
/// `id source class made_on due_on prob outcome url`.
///
/// - `class`: one of [`CLASSES`].
/// - `prob`: the probability the source attached, or `-` if it stated the
///   claim flatly.
/// - `outcome`: the date the event was observed, `-` if it has not been, or
///   `unclear`.
#[derive(Debug, Clone, PartialEq)]
pub struct Claim {
    pub id: String,
    pub source: String,
    pub class: String,
    pub made_on: i64,
    pub due_on: i64,
    pub prob: Option<f64>,
    pub outcome: Outcome,
    pub url: String,
}

/// Months of slip, and whether it is only a lower bound.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slip {
    pub months: f64,
    pub censored: bool,
}

impl Claim {
    pub fn status(&self, as_of: i64) -> Status {
        match self.outcome {
            Outcome::Unclear => Status::Unclear,
            Outcome::Occurred(d) if d <= self.due_on => Status::Hit,
            Outcome::Occurred(_) => Status::Miss,
            Outcome::NotYet if self.due_on < as_of => Status::Miss,
            Outcome::NotYet => Status::Open,
        }
    }

    /// Observed date minus stated date, in months. Negative means earlier
    /// than stated. An event that is overdue and has not happened gives the
    /// time overdue so far, marked as a lower bound.
    pub fn slip(&self, as_of: i64) -> Option<Slip> {
        match self.outcome {
            Outcome::Occurred(d) => Some(Slip {
                months: (d - self.due_on) as f64 / DAYS_PER_MONTH,
                censored: false,
            }),
            Outcome::NotYet if self.due_on < as_of => Some(Slip {
                months: (as_of - self.due_on) as f64 / DAYS_PER_MONTH,
                censored: true,
            }),
            _ => None,
        }
    }

    pub fn is_short_lead(&self) -> bool {
        self.due_on - self.made_on <= SHORT_LEAD_DAYS
    }
}

/// Parse and validate the claim table. `as_of` is the scoring date; it is an
/// argument, never the system clock, so a result reproduces.
pub fn parse_claims(text: &str, as_of: i64) -> Result<Vec<Claim>, String> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let at = |msg: String| format!("line {}: {msg}", i + 1);
        let cols: Vec<&str> = line.split('\t').map(str::trim).collect();
        if cols.len() != 8 {
            return Err(at(format!(
                "expected 8 tab-separated columns, found {}",
                cols.len()
            )));
        }
        let id = cols[0].to_string();
        if id.is_empty() || !seen.insert(id.clone()) {
            return Err(at(format!("claim id `{id}` is empty or repeated")));
        }
        if cols[1].is_empty() {
            return Err(at("source is empty".into()));
        }
        if !CLASSES.contains(&cols[2]) {
            return Err(at(format!("unknown class `{}`", cols[2])));
        }
        let made_on = parse_date(cols[3]).map_err(&at)?;
        let due_on = parse_date(cols[4]).map_err(&at)?;
        if made_on > as_of {
            return Err(at("the claim is dated after the scoring date".into()));
        }
        if due_on < made_on {
            return Err(at("the due date is before the claim was made".into()));
        }
        let prob = match cols[5] {
            "-" => None,
            p => {
                let v: f64 = p
                    .parse()
                    .map_err(|_| at(format!("probability `{p}` is not a number or `-`")))?;
                if !(0.0..=1.0).contains(&v) {
                    return Err(at(format!("probability {v} is outside 0..1")));
                }
                Some(v)
            }
        };
        let outcome = match cols[6] {
            "-" => Outcome::NotYet,
            "unclear" => Outcome::Unclear,
            d => {
                let day = parse_date(d).map_err(&at)?;
                if day > as_of {
                    return Err(at("the outcome is dated after the scoring date".into()));
                }
                if day < made_on {
                    return Err(at(
                        "the event happened before the claim was made, so it is not a forecast"
                            .into(),
                    ));
                }
                Outcome::Occurred(day)
            }
        };
        if !(cols[7].starts_with("https://") || cols[7].starts_with("http://")) {
            return Err(at(format!("`{}` is not a source URL", cols[7])));
        }
        out.push(Claim {
            id,
            source: cols[1].to_string(),
            class: cols[2].to_string(),
            made_on,
            due_on,
            prob,
            outcome,
            url: cols[7].to_string(),
        });
    }
    Ok(out)
}

pub const TSV_HEADER: &str = "# id\tsource\tclass\tmade_on\tdue_on\tprob\toutcome\turl\n";

pub fn claims_to_tsv(claims: &[Claim]) -> String {
    let mut s = String::from(TSV_HEADER);
    for c in claims {
        let prob = c.prob.map_or("-".to_string(), |p| format!("{p}"));
        let outcome = match c.outcome {
            Outcome::Occurred(d) => format_date(d),
            Outcome::NotYet => "-".into(),
            Outcome::Unclear => "unclear".into(),
        };
        let _ = writeln!(
            s,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            c.id,
            c.source,
            c.class,
            format_date(c.made_on),
            format_date(c.due_on),
            prob,
            outcome,
            c.url
        );
    }
    s
}

// ---------------------------------------------------------------- PRNG

/// SplitMix64: small, seedable, identical on every platform.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.unit() * n as f64) as usize % n.max(1)
    }
    pub fn coin(&mut self, p: f64) -> bool {
        self.unit() < p
    }
}

// ---------------------------------------------------------------- statistics

pub fn median(vals: &mut [f64]) -> f64 {
    if vals.is_empty() {
        return f64::NAN;
    }
    vals.sort_by(|a, b| a.partial_cmp(b).expect("no NaN in a median"));
    let n = vals.len();
    if n % 2 == 1 {
        vals[n / 2]
    } else {
        (vals[n / 2 - 1] + vals[n / 2]) / 2.0
    }
}

/// Median slip, and whether it is only a lower bound.
///
/// An overdue event that has not happened has a slip of at least the time
/// overdue. The median is exact when every such bound already sits at or
/// above it; otherwise the bounds stand in for the unknown values and the
/// result is a lower bound on the true median.
pub fn median_slip(slips: &[Slip]) -> (f64, bool) {
    if slips.is_empty() {
        return (f64::NAN, false);
    }
    let mut open_ended: Vec<f64> = slips
        .iter()
        .map(|s| if s.censored { f64::INFINITY } else { s.months })
        .collect();
    let m = median(&mut open_ended);
    if m.is_finite() && slips.iter().filter(|s| s.censored).all(|s| s.months >= m) {
        return (m, false);
    }
    let mut bounds: Vec<f64> = slips.iter().map(|s| s.months).collect();
    (median(&mut bounds), true)
}

/// Percentile bootstrap interval for a statistic of `n` resampled units.
pub fn bootstrap_ci<F: Fn(&[usize]) -> f64>(
    n: usize,
    stat: F,
    resamples: usize,
    seed: u64,
) -> (f64, f64) {
    if n == 0 || resamples == 0 {
        return (f64::NAN, f64::NAN);
    }
    let mut rng = Rng::new(seed);
    let mut vals = Vec::with_capacity(resamples);
    let mut idx = vec![0usize; n];
    for _ in 0..resamples {
        for slot in idx.iter_mut() {
            *slot = rng.below(n);
        }
        let v = stat(&idx);
        if v.is_finite() {
            vals.push(v);
        }
    }
    if vals.is_empty() {
        return (f64::NAN, f64::NAN);
    }
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let at = |q: f64| vals[((vals.len() - 1) as f64 * q).round() as usize];
    (at(0.025), at(0.975))
}

/// Brier score of a forecaster that says 0.5 to everything.
pub const COIN_BRIER: f64 = 0.25;

// ---------------------------------------------------------------- scoring

/// Scores for one group of claims (all of them, one source, or one class).
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub name: String,
    pub n_claims: usize,
    pub n_open: usize,
    pub n_unclear: usize,
    /// Hits plus misses.
    pub n_resolved: usize,
    pub hit_rate: f64,
    pub hit_rate_ci: (f64, f64),
    pub n_short_lead: usize,
    pub hit_rate_short_lead: f64,
    pub n_long_lead: usize,
    pub hit_rate_long_lead: f64,
    pub n_slip: usize,
    pub n_censored: usize,
    pub median_slip_months: f64,
    pub median_slip_ci: (f64, f64),
    pub slip_is_lower_bound: bool,
    pub n_with_probability: usize,
    pub brier: f64,
    pub brier_ci: (f64, f64),
    /// 1 - brier / 0.25. Zero for a coin, negative for worse than a coin.
    pub brier_skill_vs_coin: f64,
    pub brier_skill_ci: (f64, f64),
}

fn rate(flags: &[bool]) -> f64 {
    if flags.is_empty() {
        return f64::NAN;
    }
    flags.iter().filter(|&&b| b).count() as f64 / flags.len() as f64
}

pub fn score_group(
    name: &str,
    claims: &[&Claim],
    as_of: i64,
    resamples: usize,
    seed: u64,
) -> Group {
    let status: Vec<Status> = claims.iter().map(|c| c.status(as_of)).collect();
    let count = |s: Status| status.iter().filter(|&&x| x == s).count();

    // (hit, short lead) for every resolved claim.
    let resolved: Vec<(bool, bool)> = claims
        .iter()
        .zip(&status)
        .filter(|(_, s)| matches!(s, Status::Hit | Status::Miss))
        .map(|(c, s)| (*s == Status::Hit, c.is_short_lead()))
        .collect();
    let hits: Vec<bool> = resolved.iter().map(|r| r.0).collect();
    let short: Vec<bool> = resolved.iter().filter(|r| r.1).map(|r| r.0).collect();
    let long: Vec<bool> = resolved.iter().filter(|r| !r.1).map(|r| r.0).collect();
    let hit_rate_ci = bootstrap_ci(
        hits.len(),
        |idx| idx.iter().filter(|&&i| hits[i]).count() as f64 / idx.len() as f64,
        resamples,
        seed,
    );

    let slips: Vec<Slip> = claims.iter().filter_map(|c| c.slip(as_of)).collect();
    let (median_slip_months, slip_is_lower_bound) = median_slip(&slips);
    let median_slip_ci = bootstrap_ci(
        slips.len(),
        |idx| {
            let sample: Vec<Slip> = idx.iter().map(|&i| slips[i]).collect();
            median_slip(&sample).0
        },
        resamples,
        seed.wrapping_add(1),
    );

    // Squared error for every resolved claim that carried a probability.
    let sq: Vec<f64> = claims
        .iter()
        .zip(&status)
        .filter_map(|(c, s)| {
            let o = match s {
                Status::Hit => 1.0,
                Status::Miss => 0.0,
                _ => return None,
            };
            c.prob.map(|p| (p - o) * (p - o))
        })
        .collect();
    let mean_sq = |idx: &[usize]| idx.iter().map(|&i| sq[i]).sum::<f64>() / idx.len() as f64;
    let brier = if sq.is_empty() {
        f64::NAN
    } else {
        sq.iter().sum::<f64>() / sq.len() as f64
    };
    let brier_ci = bootstrap_ci(sq.len(), mean_sq, resamples, seed.wrapping_add(2));
    let skill = |b: f64| 1.0 - b / COIN_BRIER;
    // Skill falls as Brier rises, so the interval ends swap.
    let brier_skill_ci = (skill(brier_ci.1), skill(brier_ci.0));

    Group {
        name: name.to_string(),
        n_claims: claims.len(),
        n_open: count(Status::Open),
        n_unclear: count(Status::Unclear),
        n_resolved: resolved.len(),
        hit_rate: rate(&hits),
        hit_rate_ci,
        n_short_lead: short.len(),
        hit_rate_short_lead: rate(&short),
        n_long_lead: long.len(),
        hit_rate_long_lead: rate(&long),
        n_slip: slips.len(),
        n_censored: slips.iter().filter(|s| s.censored).count(),
        median_slip_months,
        median_slip_ci,
        slip_is_lower_bound,
        n_with_probability: sq.len(),
        brier,
        brier_ci,
        brier_skill_vs_coin: skill(brier),
        brier_skill_ci,
    }
}

/// Score every claim together, then each source, then each class.
pub fn score(claims: &[Claim], as_of: i64, resamples: usize, seed: u64) -> Vec<Group> {
    let all: Vec<&Claim> = claims.iter().collect();
    let mut out = vec![score_group("overall", &all, as_of, resamples, seed)];
    let sources: BTreeSet<&str> = claims.iter().map(|c| c.source.as_str()).collect();
    for s in sources {
        let part: Vec<&Claim> = claims.iter().filter(|c| c.source == s).collect();
        out.push(score_group(
            &format!("source:{s}"),
            &part,
            as_of,
            resamples,
            seed,
        ));
    }
    let classes: BTreeSet<&str> = claims.iter().map(|c| c.class.as_str()).collect();
    for k in classes {
        let part: Vec<&Claim> = claims.iter().filter(|c| c.class == k).collect();
        out.push(score_group(
            &format!("class:{k}"),
            &part,
            as_of,
            resamples,
            seed,
        ));
    }
    out
}

// ---------------------------------------------------------------- synthetic forecasters

/// Forecasters for the audit. All of them face the same simulated world, in
/// which each event has its own true chance of happening by its date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Forecaster {
    /// States the true chance. The only one with any knowledge.
    Calibrated,
    AlwaysYes,
    AlwaysNo,
    Coin,
    /// States the true chance but puts every date [`EARLY_DAYS`] too soon.
    Early,
}

pub const EARLY_DAYS: i64 = 90;

impl Forecaster {
    pub const ALL: [Forecaster; 5] = [
        Forecaster::Calibrated,
        Forecaster::AlwaysYes,
        Forecaster::AlwaysNo,
        Forecaster::Coin,
        Forecaster::Early,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Forecaster::Calibrated => "calibrated",
            Forecaster::AlwaysYes => "always_yes",
            Forecaster::AlwaysNo => "always_no",
            Forecaster::Coin => "coin",
            Forecaster::Early => "early",
        }
    }
}

/// `n` simulated claims. The world (which events happen, and when) depends
/// only on the seed, so every forecaster is scored on identical events.
pub fn synth_claims(f: Forecaster, n: usize, seed: u64, as_of: i64) -> Vec<Claim> {
    synth_claims_made_on(f, n, seed, as_of, as_of - 1100)
}

/// As [`synth_claims`] with the date the claims were made given explicitly,
/// so the as-of date can fall inside the range of due dates.
pub fn synth_claims_made_on(
    f: Forecaster,
    n: usize,
    seed: u64,
    as_of: i64,
    made_on: i64,
) -> Vec<Claim> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|i| {
            // Every draw is made for every claim so the stream never forks.
            let chance = 0.05 + 0.9 * rng.unit();
            let lead = 200 + rng.below(500) as i64;
            let happens = rng.coin(chance);
            let before = rng.below(180) as i64;
            let delay = 1 + rng.below(600) as i64;
            let true_due = made_on + lead;
            let day = if happens {
                true_due - before
            } else {
                true_due + delay
            };
            let outcome = if day <= as_of {
                Outcome::Occurred(day)
            } else {
                Outcome::NotYet
            };
            let (due_on, prob) = match f {
                Forecaster::Calibrated => (true_due, chance),
                Forecaster::AlwaysYes => (true_due, 1.0),
                Forecaster::AlwaysNo => (true_due, 0.0),
                Forecaster::Coin => (true_due, 0.5),
                Forecaster::Early => (true_due - EARLY_DAYS, chance),
            };
            Claim {
                id: format!("{}-{i}", f.name()),
                source: f.name().to_string(),
                class: CLASSES[i % CLASSES.len()].to_string(),
                made_on,
                due_on,
                prob: Some(prob),
                outcome,
                url: "https://example.invalid/synthetic".to_string(),
            }
        })
        .collect()
}

// ---------------------------------------------------------------- selection audit

/// How far a subset of claims moves the headline measures from the full set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shift {
    pub median_abs_slip_shift_months: f64,
    pub median_abs_hit_rate_shift: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    Random,
    HitFavouring,
    MissFavouring,
}

impl Pick {
    pub const ALL: [Pick; 3] = [Pick::Random, Pick::HitFavouring, Pick::MissFavouring];
    pub fn name(self) -> &'static str {
        match self {
            Pick::Random => "random",
            Pick::HitFavouring => "hit_favouring",
            Pick::MissFavouring => "miss_favouring",
        }
    }
}

fn subset_measures(claims: &[&Claim], as_of: i64) -> (f64, f64) {
    let slips: Vec<Slip> = claims.iter().filter_map(|c| c.slip(as_of)).collect();
    let hits = claims
        .iter()
        .filter(|c| c.status(as_of) == Status::Hit)
        .count();
    (median_slip(&slips).0, hits as f64 / claims.len() as f64)
}

/// Draw `pick` claims by `rule` `draws` times from the resolved claims and
/// report the median absolute shift of median slip and of hit rate from the
/// full resolved set. Returns `None` if the pool is smaller than `pick`.
pub fn selection_audit(
    claims: &[Claim],
    as_of: i64,
    rule: Pick,
    pick: usize,
    draws: usize,
    seed: u64,
) -> Option<Shift> {
    let resolved: Vec<&Claim> = claims
        .iter()
        .filter(|c| matches!(c.status(as_of), Status::Hit | Status::Miss))
        .collect();
    let (full_slip, full_hit) = subset_measures(&resolved, as_of);
    let pool: Vec<&Claim> = resolved
        .iter()
        .copied()
        .filter(|c| match rule {
            Pick::Random => true,
            Pick::HitFavouring => c.status(as_of) == Status::Hit,
            Pick::MissFavouring => c.status(as_of) == Status::Miss,
        })
        .collect();
    if pick == 0 || pool.len() < pick {
        return None;
    }
    let mut rng = Rng::new(seed);
    let mut slip_shifts = Vec::with_capacity(draws);
    let mut hit_shifts = Vec::with_capacity(draws);
    for _ in 0..draws {
        // Partial Fisher-Yates: the first `pick` entries are the draw.
        let mut idx: Vec<usize> = (0..pool.len()).collect();
        for i in 0..pick {
            let j = i + rng.below(pool.len() - i);
            idx.swap(i, j);
        }
        let chosen: Vec<&Claim> = idx[..pick].iter().map(|&i| pool[i]).collect();
        let (s, h) = subset_measures(&chosen, as_of);
        slip_shifts.push((s - full_slip).abs());
        hit_shifts.push((h - full_hit).abs());
    }
    Some(Shift {
        median_abs_slip_shift_months: median(&mut slip_shifts),
        median_abs_hit_rate_shift: median(&mut hit_shifts),
    })
}

/// Draw `pick` resolved claims without replacement `draws` times and report
/// the median absolute gap between the claim-pooled hit rate and the
/// unweighted mean of per-class hit rates (classes present in the draw).
/// Returns `None` if fewer than `pick` claims are resolved.
pub fn aggregation_audit(
    claims: &[Claim],
    as_of: i64,
    pick: usize,
    draws: usize,
    seed: u64,
) -> Option<f64> {
    let resolved: Vec<&Claim> = claims
        .iter()
        .filter(|c| matches!(c.status(as_of), Status::Hit | Status::Miss))
        .collect();
    if pick == 0 || resolved.len() < pick {
        return None;
    }
    let mut rng = Rng::new(seed);
    let mut gaps = Vec::with_capacity(draws);
    for _ in 0..draws {
        let mut idx: Vec<usize> = (0..resolved.len()).collect();
        for i in 0..pick {
            let j = i + rng.below(resolved.len() - i);
            idx.swap(i, j);
        }
        let mut per_class: Vec<(usize, usize)> = vec![(0, 0); CLASSES.len()];
        let mut hits = 0usize;
        for &i in &idx[..pick] {
            let c = resolved[i];
            let k = CLASSES.iter().position(|x| *x == c.class).unwrap_or(0);
            per_class[k].1 += 1;
            if c.status(as_of) == Status::Hit {
                per_class[k].0 += 1;
                hits += 1;
            }
        }
        let rates: Vec<f64> = per_class
            .iter()
            .filter(|(_, n)| *n > 0)
            .map(|(h, n)| *h as f64 / *n as f64)
            .collect();
        let class_mean = rates.iter().sum::<f64>() / rates.len() as f64;
        gaps.push((hits as f64 / pick as f64 - class_mean).abs());
    }
    Some(median(&mut gaps))
}

// ---------------------------------------------------------------- power of the Brier measure

/// How often the bootstrap interval on Brier skill excludes zero for claim
/// sets of one size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrierPower {
    pub size: usize,
    pub excludes_zero_rate: f64,
    pub median_skill: f64,
}

/// Draw `size` resolved claims without replacement `draws` times and report
/// the share whose `boot`-resample interval on Brier skill has a lower end
/// above zero. Returns `None` if fewer than `size` resolved claims carry a
/// probability.
pub fn power_brier(
    claims: &[Claim],
    as_of: i64,
    size: usize,
    draws: usize,
    boot: usize,
    seed: u64,
) -> Option<BrierPower> {
    let sq: Vec<f64> = claims
        .iter()
        .filter_map(|c| {
            let o = match c.status(as_of) {
                Status::Hit => 1.0,
                Status::Miss => 0.0,
                _ => return None,
            };
            c.prob.map(|p| (p - o) * (p - o))
        })
        .collect();
    if size == 0 || sq.len() < size || draws == 0 {
        return None;
    }
    let mut rng = Rng::new(seed);
    let mut excluded = 0usize;
    let mut skills = Vec::with_capacity(draws);
    for d in 0..draws {
        let mut idx: Vec<usize> = (0..sq.len()).collect();
        for i in 0..size {
            let j = i + rng.below(sq.len() - i);
            idx.swap(i, j);
        }
        let chosen: Vec<f64> = idx[..size].iter().map(|&i| sq[i]).collect();
        let ci = bootstrap_ci(
            size,
            |ix| ix.iter().map(|&i| chosen[i]).sum::<f64>() / ix.len() as f64,
            boot,
            seed.wrapping_add(1 + d as u64),
        );
        // Skill falls as Brier rises, so the lower end comes from the upper Brier end.
        if 1.0 - ci.1 / COIN_BRIER > 0.0 {
            excluded += 1;
        }
        skills.push(1.0 - chosen.iter().sum::<f64>() / size as f64 / COIN_BRIER);
    }
    Some(BrierPower {
        size,
        excludes_zero_rate: excluded as f64 / draws as f64,
        median_skill: median(&mut skills),
    })
}

/// Replace every stated probability with a uniform random draw independent
/// of the outcome: a forecaster with no foresight and no fixed answer.
pub fn randomise_probs(claims: &[Claim], seed: u64) -> Vec<Claim> {
    let mut rng = Rng::new(seed ^ 0x5bd1_e995);
    claims
        .iter()
        .map(|c| {
            let mut c = c.clone();
            c.prob = Some(rng.unit());
            c
        })
        .collect()
}

// ---------------------------------------------------------------- JSON

fn num(v: f64) -> String {
    if v.is_finite() {
        // Six decimals: stable across platforms and enough for a rate.
        let s = format!("{v:.6}");
        if s == "-0.000000" {
            "0.000000".into()
        } else {
            s
        }
    } else {
        "null".into()
    }
}

fn pair(ci: (f64, f64)) -> String {
    format!("[{}, {}]", num(ci.0), num(ci.1))
}

// ---------------------------------------------------------------- slack sensitivity

/// Verdict counts when the on-time window is the stated date plus `slack_days`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlackRow {
    pub slack_days: i64,
    pub hits: usize,
    pub misses: usize,
    pub open: usize,
    pub hit_rate: f64,
    /// Claims that are a miss at slack 0 and a hit at this slack.
    pub miss_to_hit_count: usize,
    /// Claims that are a miss at slack 0 and still undecided at this slack.
    pub miss_to_open_count: usize,
}

fn status_with_slack(c: &Claim, as_of: i64, slack: i64) -> Status {
    let mut shifted = c.clone();
    shifted.due_on += slack;
    shifted.status(as_of)
}

/// Score the same claims under each slack. Unclear claims are never scored.
pub fn slack_sensitivity(claims: &[Claim], as_of: i64, slacks: &[i64]) -> Vec<SlackRow> {
    slacks
        .iter()
        .map(|&slack| {
            let (mut hits, mut misses, mut open) = (0, 0, 0);
            let (mut m2h, mut m2o) = (0, 0);
            for c in claims {
                let now = status_with_slack(c, as_of, slack);
                let base = status_with_slack(c, as_of, 0);
                match now {
                    Status::Hit => hits += 1,
                    Status::Miss => misses += 1,
                    Status::Open => open += 1,
                    Status::Unclear => {}
                }
                if base == Status::Miss {
                    match now {
                        Status::Hit => m2h += 1,
                        Status::Open => m2o += 1,
                        _ => {}
                    }
                }
            }
            let resolved = hits + misses;
            SlackRow {
                slack_days: slack,
                hits,
                misses,
                open,
                hit_rate: if resolved == 0 {
                    0.0
                } else {
                    hits as f64 / resolved as f64
                },
                miss_to_hit_count: m2h,
                miss_to_open_count: m2o,
            }
        })
        .collect()
}

pub fn json_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o
}

/// One group as a single-line JSON object.
pub fn group_json(g: &Group) -> String {
    format!(
        "{{\"n_claims\": {}, \"n_open\": {}, \"n_unclear\": {}, \"n_resolved\": {}, \
\"hit_rate\": {}, \"hit_rate_ci95\": {}, \
\"n_short_lead\": {}, \"hit_rate_short_lead\": {}, \"n_long_lead\": {}, \"hit_rate_long_lead\": {}, \
\"n_slip\": {}, \"n_censored\": {}, \"median_slip_months\": {}, \"median_slip_ci95\": {}, \"slip_is_lower_bound\": {}, \
\"n_with_probability\": {}, \"brier\": {}, \"brier_ci95\": {}, \"brier_skill_vs_coin\": {}, \"brier_skill_ci95\": {}}}",
        g.n_claims,
        g.n_open,
        g.n_unclear,
        g.n_resolved,
        num(g.hit_rate),
        pair(g.hit_rate_ci),
        g.n_short_lead,
        num(g.hit_rate_short_lead),
        g.n_long_lead,
        num(g.hit_rate_long_lead),
        g.n_slip,
        g.n_censored,
        num(g.median_slip_months),
        pair(g.median_slip_ci),
        g.slip_is_lower_bound,
        g.n_with_probability,
        num(g.brier),
        pair(g.brier_ci),
        num(g.brier_skill_vs_coin),
        pair(g.brier_skill_ci),
    )
}

/// Seconds since the epoch as `YYYY-MM-DDTHH:MM:SSZ`.
pub fn iso_utc(secs: u64) -> String {
    let rem = secs % 86_400;
    format!(
        "{}T{:02}:{:02}:{:02}Z",
        format_date((secs / 86_400) as i64),
        rem / 3_600,
        rem % 3_600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn suite_refresh_naive_understates_and_step_fit_recovers() {
        let r = suite_refresh(7, 6.0, 18, 1.0);
        assert!(r.naive_bias_months < 0.0);
        assert!(r.step_bias_months.abs() < r.naive_bias_months.abs());
        assert_eq!(r, suite_refresh(7, 6.0, 18, 1.0));
        let z = suite_refresh(7, 6.0, 18, 0.0);
        assert!((z.naive_bias_months - z.step_bias_months).abs() < 0.5);
    }

    use super::*;

    #[test]
    fn dates_round_trip_and_impossible_dates_are_rejected() {
        assert_eq!(parse_date("1970-01-01").unwrap(), 0);
        assert_eq!(parse_date("2027-01-01").unwrap(), 20_819);
        assert_eq!(format_date(parse_date("2024-02-29").unwrap()), "2024-02-29");
        for bad in ["2026-02-30", "2025-02-29", "2026-13-01", "26-01-01", "soon"] {
            assert!(parse_date(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn median_handles_even_odd_and_empty() {
        assert_eq!(median(&mut [3.0, 1.0, 2.0]), 2.0);
        assert_eq!(median(&mut [4.0, 1.0, 2.0, 3.0]), 2.5);
        assert!(median(&mut []).is_nan());
    }

    #[test]
    fn iso_utc_formats_a_known_instant() {
        assert_eq!(iso_utc(1_700_000_000), "2023-11-14T22:13:20Z");
    }
}

// ---------------------------------------------------------------- convention audit

/// Median slip of one class under two conventions.
#[derive(Debug, Clone, PartialEq)]
pub struct ConventionRow {
    pub class: String,
    pub resolved: usize,
    pub censored: usize,
    /// Median over all slips, censored ones entering as their lower bound.
    pub keep_median_months: f64,
    /// Median over uncensored slips only.
    pub drop_median_months: f64,
    /// Keep minus drop. NaN when either median is undefined.
    pub gap_months: f64,
}

/// Per-class median slip with overdue never-happened claims kept as lower
/// bounds versus dropped.
pub fn convention_audit(claims: &[Claim], as_of: i64) -> Vec<ConventionRow> {
    CLASSES
        .iter()
        .map(|&class| {
            let slips: Vec<Slip> = claims
                .iter()
                .filter(|c| c.class == class)
                .filter_map(|c| c.slip(as_of))
                .collect();
            let mut keep: Vec<f64> = slips.iter().map(|s| s.months).collect();
            let mut drop: Vec<f64> = slips
                .iter()
                .filter(|s| !s.censored)
                .map(|s| s.months)
                .collect();
            let k = median(&mut keep);
            let d = median(&mut drop);
            ConventionRow {
                class: class.to_string(),
                resolved: slips.len(),
                censored: slips.iter().filter(|s| s.censored).count(),
                keep_median_months: k,
                drop_median_months: d,
                gap_months: k - d,
            }
        })
        .collect()
}

// ---------------------------------------------------------------- lead audit (censored)

/// Resolved share and median slip for one lead-time bin.
#[derive(Debug, Clone, PartialEq)]
pub struct LeadRow {
    pub bin: &'static str,
    pub claims: usize,
    pub open: usize,
    pub resolved_share: f64,
    pub censored: usize,
    /// Median slip with overdue never-happened claims as lower bounds; NaN if none.
    pub median_slip_months: f64,
}

/// Per lead bin (short, long): share of claims resolved (hit or miss, not
/// open) and median slip. Open claims count as right-censored: they stay in
/// the denominator and carry no slip.
pub fn lead_audit_censored(claims: &[Claim], as_of: i64) -> Vec<LeadRow> {
    [("short", true), ("long", false)]
        .iter()
        .map(|&(bin, short)| {
            let sel: Vec<&Claim> = claims
                .iter()
                .filter(|c| c.is_short_lead() == short)
                .collect();
            let open = sel
                .iter()
                .filter(|c| c.status(as_of) == Status::Open)
                .count();
            let mut slips: Vec<f64> = sel
                .iter()
                .filter_map(|c| c.slip(as_of))
                .map(|s| s.months)
                .collect();
            let censored = sel
                .iter()
                .filter_map(|c| c.slip(as_of))
                .filter(|s| s.censored)
                .count();
            LeadRow {
                bin,
                claims: sel.len(),
                open,
                resolved_share: if sel.is_empty() {
                    f64::NAN
                } else {
                    (sel.len() - open) as f64 / sel.len() as f64
                },
                censored,
                median_slip_months: median(&mut slips),
            }
        })
        .collect()
}

// ---------------------------------------------------------------- km slip

/// Naive and Kaplan-Meier median slip for one lead bin.
#[derive(Debug, Clone, PartialEq)]
pub struct KmRow {
    pub bin: &'static str,
    pub claims: usize,
    pub events: usize,
    pub censored: usize,
    /// Median over occurred claims only; NaN if none.
    pub naive_median_months: f64,
    /// Kaplan-Meier median with open and overdue claims right-censored; NaN
    /// when survival never reaches one half.
    pub km_median_months: f64,
}

/// Kaplan-Meier median of (time, is_event) pairs; NaN if undefined.
pub fn km_median(obs: &[(f64, bool)]) -> f64 {
    let mut v: Vec<(f64, bool)> = obs.to_vec();
    v.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .expect("no NaN in a time")
            .then(b.1.cmp(&a.1))
    });
    let mut at_risk = v.len() as f64;
    let mut surv = 1.0;
    let mut i = 0;
    while i < v.len() {
        let t = v[i].0;
        let mut deaths = 0.0;
        let mut total = 0.0;
        while i < v.len() && v[i].0 == t {
            if v[i].1 {
                deaths += 1.0;
            }
            total += 1.0;
            i += 1;
        }
        if deaths > 0.0 {
            surv *= 1.0 - deaths / at_risk;
            if surv <= 0.5 {
                return t;
            }
        }
        at_risk -= total;
    }
    f64::NAN
}

/// Per lead bin (short, long): naive median over occurred claims versus a
/// Kaplan-Meier median where claims that have not happened by as-of are
/// censored at as-of minus due date.
pub fn km_slip(claims: &[Claim], as_of: i64) -> Vec<KmRow> {
    [("short", true), ("long", false)]
        .iter()
        .map(|&(bin, short)| {
            let mut obs: Vec<(f64, bool)> = Vec::new();
            for c in claims.iter().filter(|c| c.is_short_lead() == short) {
                match c.outcome {
                    Outcome::Occurred(d) => {
                        obs.push(((d - c.due_on) as f64 / DAYS_PER_MONTH, true))
                    }
                    Outcome::NotYet => {
                        obs.push(((as_of - c.due_on) as f64 / DAYS_PER_MONTH, false))
                    }
                    Outcome::Unclear => {}
                }
            }
            let mut ev: Vec<f64> = obs.iter().filter(|o| o.1).map(|o| o.0).collect();
            KmRow {
                bin,
                claims: obs.len(),
                events: ev.len(),
                censored: obs.len() - ev.len(),
                naive_median_months: median(&mut ev),
                km_median_months: km_median(&obs),
            }
        })
        .collect()
}

#[cfg(test)]
mod km_tests {
    use super::*;

    #[test]
    fn km_median_without_censoring_is_lower_median() {
        let obs = [(1.0, true), (2.0, true), (3.0, true), (4.0, true)];
        assert_eq!(km_median(&obs), 2.0);
    }

    #[test]
    fn km_median_moves_up_with_early_censoring() {
        let obs = [
            (1.0, true),
            (1.5, false),
            (2.0, false),
            (3.0, true),
            (4.0, true),
        ];
        assert_eq!(km_median(&obs), 3.0);
    }

    #[test]
    fn km_median_undefined_when_survival_stays_high() {
        let obs = [(1.0, true), (2.0, false), (3.0, false), (4.0, false)];
        assert!(km_median(&obs).is_nan());
    }
}

// ---------------------------------------------------------------- class-mix sensitivity

/// Median over `draws` resamples (with replacement, size `claims.len()`) of
/// the pooled median slip, for each capability-class share in `shares`.
/// Each resampled claim is drawn from the capability pool with probability
/// `share`, otherwise from the other classes. Claims with no slip (open or
/// unclear) are skipped when pooling. Returns `None` if either pool is empty.
pub fn class_mix(
    claims: &[Claim],
    as_of: i64,
    shares: &[f64],
    draws: usize,
    seed: u64,
) -> Option<Vec<(f64, f64)>> {
    let cap: Vec<Slip> = claims
        .iter()
        .filter(|c| c.class == "capability")
        .filter_map(|c| c.slip(as_of))
        .collect();
    let other: Vec<Slip> = claims
        .iter()
        .filter(|c| c.class != "capability")
        .filter_map(|c| c.slip(as_of))
        .collect();
    if cap.is_empty() || other.is_empty() {
        return None;
    }
    let n = claims.len();
    let mut rng = Rng::new(seed);
    let mut out = Vec::with_capacity(shares.len());
    for &share in shares {
        let mut meds = Vec::with_capacity(draws);
        for _ in 0..draws {
            let sample: Vec<Slip> = (0..n)
                .map(|_| {
                    if rng.coin(share) {
                        cap[rng.below(cap.len())]
                    } else {
                        other[rng.below(other.len())]
                    }
                })
                .collect();
            meds.push(median_slip(&sample).0);
        }
        out.push((share, median(&mut meds)));
    }
    Some(out)
}

// ---------------------------------------------------------- pace snapshots

/// One as-of snapshot of a fixed world: share resolved and the naive median
/// slip over occurred claims only.
#[derive(Debug, Clone, PartialEq)]
pub struct PaceSnapshot {
    pub claims: usize,
    pub resolved_share: f64,
    pub events: usize,
    /// Median over occurred claims only; NaN if none.
    pub naive_median_months: f64,
}

/// Score the same claims at one as-of date.
pub fn pace_snapshot(claims: &[Claim], as_of: i64) -> PaceSnapshot {
    let open = claims
        .iter()
        .filter(|c| c.status(as_of) == Status::Open)
        .count();
    let mut ev: Vec<f64> = claims
        .iter()
        .filter_map(|c| match c.outcome {
            Outcome::Occurred(d) => Some((d - c.due_on) as f64 / DAYS_PER_MONTH),
            _ => None,
        })
        .collect();
    PaceSnapshot {
        claims: claims.len(),
        resolved_share: if claims.is_empty() {
            f64::NAN
        } else {
            (claims.len() - open) as f64 / claims.len() as f64
        },
        events: ev.len(),
        naive_median_months: median(&mut ev),
    }
}

// ---------------------------------------------------------------- group gap

/// Result of the planted-gap check on resolved claims only.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupGap {
    pub fast_resolved: usize,
    pub slow_resolved: usize,
    pub fast_median_months: f64,
    pub slow_median_months: f64,
    pub planted_gap_months: f64,
    pub measured_gap_months: f64,
    pub recovery_error_months: f64,
}

/// Simulate `n` claims split evenly between a fast group (planted slip
/// `+planted` months) and a slow group (`-planted`), each with zero-mean
/// triangular noise of half-width 6 months, made `made_days` before `as_of`
/// with leads of 60 to 700 days. A claim is resolved only if its event date
/// is on or before `as_of`. The gap is fast minus slow median slip over
/// resolved claims, compared with `2 * planted`.
pub fn group_gap(n: usize, seed: u64, as_of: i64, made_days: i64, planted: f64) -> GroupGap {
    let mut rng = Rng::new(seed);
    let made_on = as_of - made_days;
    let mut fast = Vec::new();
    let mut slow = Vec::new();
    for i in 0..n {
        let lead = 60 + rng.below(641) as i64;
        let due = made_on + lead;
        let noise = (rng.unit() + rng.unit() - 1.0) * 6.0;
        let (bias, bucket) = if i % 2 == 0 {
            (planted, &mut fast)
        } else {
            (-planted, &mut slow)
        };
        let slip = bias + noise;
        let event = due + (slip * DAYS_PER_MONTH).round() as i64;
        if event <= as_of {
            bucket.push((event - due) as f64 / DAYS_PER_MONTH);
        }
    }
    let (fr, sr) = (fast.len(), slow.len());
    let fm = median(&mut fast);
    let sm = median(&mut slow);
    let measured = fm - sm;
    let planted_gap = 2.0 * planted;
    GroupGap {
        fast_resolved: fr,
        slow_resolved: sr,
        fast_median_months: fm,
        slow_median_months: sm,
        planted_gap_months: planted_gap,
        measured_gap_months: measured,
        recovery_error_months: (measured - planted_gap).abs(),
    }
}

// ---------------------------------------------------------------- ceiling doubling

/// Doubling-time fit on a series capped at one suite ceiling.
#[derive(Debug, Clone, PartialEq)]
pub struct CeilingFit {
    pub cap_fraction: f64,
    pub capped_points: usize,
    pub fitted_doubling_months: f64,
    pub bias_months: f64,
}

/// Result of the ceiling check: the uncapped fit and one fit per cap.
#[derive(Debug, Clone, PartialEq)]
pub struct CeilingDoubling {
    pub true_doubling_months: f64,
    pub uncapped_doubling_months: f64,
    pub uncapped_bias_months: f64,
    pub fits: Vec<CeilingFit>,
}

/// Least-squares slope of `y` on `x`.
fn ols_slope(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    let mx = x.iter().sum::<f64>() / n;
    let my = y.iter().sum::<f64>() / n;
    let sxy: f64 = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum();
    let sxx: f64 = x.iter().map(|a| (a - mx) * (a - mx)).sum();
    sxy / sxx
}

/// Simulate a horizon series `h(t) = 2^(t / true_doubling)` at monthly points
/// `t = 0..=36`, with zero-mean triangular noise of half-width 0.1 in log2.
/// For each cap fraction `f`, observations are clipped at `f` times the largest
/// noiseless horizon (the suite ceiling); a log-linear fit then estimates the
/// doubling time. Bias is fitted minus true doubling time, in months.
pub fn ceiling_doubling(seed: u64, true_doubling: f64, caps: &[f64]) -> CeilingDoubling {
    let mut rng = Rng::new(seed);
    let t: Vec<f64> = (0..=36).map(|m| m as f64).collect();
    let log_h: Vec<f64> = t
        .iter()
        .map(|&m| m / true_doubling + (rng.unit() + rng.unit() - 1.0) * 0.1)
        .collect();
    let top = 36.0 / true_doubling;
    let uncapped = 1.0 / ols_slope(&t, &log_h);
    let fits = caps
        .iter()
        .map(|&f| {
            let ceiling = top + f.log2();
            let y: Vec<f64> = log_h.iter().map(|&v| v.min(ceiling)).collect();
            let capped = log_h.iter().filter(|&&v| v > ceiling).count();
            let d = 1.0 / ols_slope(&t, &y);
            CeilingFit {
                cap_fraction: f,
                capped_points: capped,
                fitted_doubling_months: d,
                bias_months: d - true_doubling,
            }
        })
        .collect();
    CeilingDoubling {
        true_doubling_months: true_doubling,
        uncapped_doubling_months: uncapped,
        uncapped_bias_months: uncapped - true_doubling,
        fits,
    }
}

// ---------------------------------------------------------------- suite refresh

/// Naive and step-term doubling-time fits on a series with a level step.
#[derive(Debug, Clone, PartialEq)]
pub struct SuiteRefresh {
    pub true_doubling_months: f64,
    pub naive_doubling_months: f64,
    pub naive_bias_months: f64,
    pub step_doubling_months: f64,
    pub step_bias_months: f64,
    pub step_estimate_log2: f64,
}

/// Simulate `h(t) = 2^(t / true_doubling)` at `t = 0..=36` with triangular log2
/// noise of half-width 0.1, plus `step_log2` added to every point from
/// `refresh_month` on. The naive fit is a log-linear slope; the step fit
/// regresses on `t` and a step indicator (two-variable least squares).
pub fn suite_refresh(
    seed: u64,
    true_doubling: f64,
    refresh_month: usize,
    step_log2: f64,
) -> SuiteRefresh {
    let mut rng = Rng::new(seed);
    let t: Vec<f64> = (0..=36).map(|m| m as f64).collect();
    let s: Vec<f64> = (0..=36)
        .map(|m| if m >= refresh_month { 1.0 } else { 0.0 })
        .collect();
    let y: Vec<f64> = t
        .iter()
        .zip(&s)
        .map(|(&m, &st)| m / true_doubling + st * step_log2 + (rng.unit() + rng.unit() - 1.0) * 0.1)
        .collect();
    let naive = 1.0 / ols_slope(&t, &y);
    let n = t.len() as f64;
    let (mt, ms, my) = (
        t.iter().sum::<f64>() / n,
        s.iter().sum::<f64>() / n,
        y.iter().sum::<f64>() / n,
    );
    let (mut stt, mut sts, mut sss, mut sty, mut ssy) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for i in 0..t.len() {
        let (a, b, c) = (t[i] - mt, s[i] - ms, y[i] - my);
        stt += a * a;
        sts += a * b;
        sss += b * b;
        sty += a * c;
        ssy += b * c;
    }
    let det = stt * sss - sts * sts;
    let slope = (sty * sss - ssy * sts) / det;
    let step = (ssy * stt - sty * sts) / det;
    let step_d = 1.0 / slope;
    SuiteRefresh {
        true_doubling_months: true_doubling,
        naive_doubling_months: naive,
        naive_bias_months: naive - true_doubling,
        step_doubling_months: step_d,
        step_bias_months: step_d - true_doubling,
        step_estimate_log2: step,
    }
}

// ---------------------------------------------------------------- pace-ratio conventions

/// Pace ratios of one simulated claim set under three counting conventions.
#[derive(Debug, Clone, PartialEq)]
pub struct PaceConventions {
    pub n: usize,
    pub misgraded: usize,
    pub mean_of_ratios: f64,
    pub ratio_of_sums: f64,
    pub median_of_ratios: f64,
    pub max_minus_min: f64,
}

fn gauss(rng: &mut Rng) -> f64 {
    let u = rng.unit().max(1e-12);
    let v = rng.unit();
    (-2.0 * u.ln()).sqrt() * (2.0 * std::f64::consts::PI * v).cos()
}

/// Simulate `n` quantitative claims, each with a predicted quantity
/// (log-normal, sigma 1) and an achieved quantity equal to the predicted one
/// times `true_ratio` times log-normal noise (sigma 0.8). A `misgrade_share`
/// of claims is graded against a wrong curve: the achieved value is scaled by
/// 2.5 or 0.4 at random. The same claims are scored with and without that
/// perturbation; call again with share 0.0 for the clean spread.
pub fn pace_ratio_conventions(
    seed: u64,
    n: usize,
    true_ratio: f64,
    misgrade_share: f64,
) -> PaceConventions {
    let mut rng = Rng::new(seed);
    let mut pred = Vec::with_capacity(n);
    let mut ach = Vec::with_capacity(n);
    let mut flags = Vec::with_capacity(n);
    for _ in 0..n {
        let p = (gauss(&mut rng)).exp() * 100.0;
        let a = p * true_ratio * (0.8 * gauss(&mut rng)).exp();
        let bad = rng.coin(misgrade_share);
        let up = rng.coin(0.5);
        pred.push(p);
        ach.push(if bad {
            a * if up { 2.5 } else { 0.4 }
        } else {
            a
        });
        flags.push(bad);
    }
    let mut ratios: Vec<f64> = pred.iter().zip(&ach).map(|(p, a)| a / p).collect();
    let mean = ratios.iter().sum::<f64>() / n.max(1) as f64;
    let ros = ach.iter().sum::<f64>() / pred.iter().sum::<f64>();
    let med = median(&mut ratios);
    let hi = mean.max(ros).max(med);
    let lo = mean.min(ros).min(med);
    PaceConventions {
        n,
        misgraded: flags.iter().filter(|&&b| b).count(),
        mean_of_ratios: mean,
        ratio_of_sums: ros,
        median_of_ratios: med,
        max_minus_min: hi - lo,
    }
}

// ---------------------------------------------------------------- revision audit

/// Brier skill of prior, informative revision, noise-only revision and 0.5.
#[derive(Debug, Clone, PartialEq)]
pub struct RevisionAudit {
    pub n: usize,
    pub prior_skill: f64,
    pub revised_skill: f64,
    pub noise_revised_skill: f64,
    pub always_half_skill: f64,
}

fn clamp_prob(p: f64) -> f64 {
    p.clamp(0.01, 0.99)
}

/// Simulate `n` resolved claims with true outcome probability `p` (uniform) and a
/// drawn outcome. The prior forecast is `0.5 + 0.25 (p - 0.5)` plus triangular
/// noise of half-width 0.2; the informative revision is `p` plus the same kind
/// of noise; the noise-only revision is the prior plus independent noise.
pub fn revision_audit(seed: u64, n: usize) -> RevisionAudit {
    let mut rng = Rng::new(seed);
    let tri = |r: &mut Rng| (r.unit() + r.unit() - 1.0) * 0.2;
    let (mut bp, mut br, mut bn, mut bh) = (0.0, 0.0, 0.0, 0.0);
    for _ in 0..n {
        let p = rng.unit();
        let y = if rng.coin(p) { 1.0 } else { 0.0 };
        let prior = clamp_prob(0.5 + 0.25 * (p - 0.5) + tri(&mut rng));
        let revised = clamp_prob(p + tri(&mut rng));
        let noise = clamp_prob(prior + tri(&mut rng));
        bp += (prior - y) * (prior - y);
        br += (revised - y) * (revised - y);
        bn += (noise - y) * (noise - y);
        bh += 0.25;
    }
    let skill = |b: f64| 1.0 - (b / n as f64) / 0.25;
    RevisionAudit {
        n,
        prior_skill: skill(bp),
        revised_skill: skill(br),
        noise_revised_skill: skill(bn),
        always_half_skill: skill(bh),
    }
}

// ---------------------------------------------------------------- fit points

/// Error and coverage of a projected date when fitting the last `n` points.
#[derive(Debug, Clone, PartialEq)]
pub struct FitPointsRow {
    pub n: usize,
    pub median_abs_error_months: f64,
    pub interval_coverage: f64,
}

/// 95% two-sided Student t critical value for `df` degrees of freedom.
fn t_crit_95(df: usize) -> f64 {
    match df {
        2 => 4.303,
        4 => 2.776,
        6 => 2.447,
        10 => 2.228,
        18 => 2.101,
        _ => 1.96,
    }
}

/// For each `n` in `ns`, simulate `reps` series `log2 h(t) = t / true_doubling`
/// plus triangular noise of half-width 0.1 at `t = 0..=36`, fit a line to the
/// last `n` points and project the month at which the fitted line reaches the
/// true level at month 60. Reports the median absolute date error and the
/// share of replicates whose 95% confidence interval for the fitted level at
/// month 60 contains the true level. Every `n` sees the same series.
pub fn fit_points(seed: u64, true_doubling: f64, reps: usize, ns: &[usize]) -> Vec<FitPointsRow> {
    let target_t = 60.0;
    let level = target_t / true_doubling;
    let mut errs: Vec<Vec<f64>> = vec![Vec::new(); ns.len()];
    let mut hits = vec![0usize; ns.len()];
    let mut rng = Rng::new(seed);
    for _ in 0..reps {
        let y: Vec<f64> = (0..=36)
            .map(|m| m as f64 / true_doubling + (rng.unit() + rng.unit() - 1.0) * 0.1)
            .collect();
        for (k, &n) in ns.iter().enumerate() {
            let start = y.len() - n;
            let xs: Vec<f64> = (start..y.len()).map(|m| m as f64).collect();
            let ys = &y[start..];
            let b = ols_slope(&xs, ys);
            let nf = n as f64;
            let mx = xs.iter().sum::<f64>() / nf;
            let my = ys.iter().sum::<f64>() / nf;
            let a = my - b * mx;
            errs[k].push(((level - a) / b - target_t).abs());
            let sse: f64 = xs
                .iter()
                .zip(ys)
                .map(|(x, v)| (v - a - b * x).powi(2))
                .sum();
            let sxx: f64 = xs.iter().map(|x| (x - mx).powi(2)).sum();
            let s = (sse / (nf - 2.0)).sqrt();
            let hw = t_crit_95(n - 2) * s * (1.0 / nf + (target_t - mx).powi(2) / sxx).sqrt();
            if (a + b * target_t - level).abs() <= hw {
                hits[k] += 1;
            }
        }
    }
    ns.iter()
        .enumerate()
        .map(|(k, &n)| {
            let mut e = errs[k].clone();
            e.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let med = if e.len() % 2 == 1 {
                e[e.len() / 2]
            } else {
                (e[e.len() / 2 - 1] + e[e.len() / 2]) / 2.0
            };
            FitPointsRow {
                n,
                median_abs_error_months: med,
                interval_coverage: hits[k] as f64 / reps as f64,
            }
        })
        .collect()
}

#[cfg(test)]
mod fit_points_tests {
    use super::*;

    #[test]
    fn deterministic_and_fewer_points_err_more() {
        let a = fit_points(7, 6.0, 200, &[4, 6, 8, 12, 20]);
        assert_eq!(a, fit_points(7, 6.0, 200, &[4, 6, 8, 12, 20]));
        assert!(a[1].median_abs_error_months > a[4].median_abs_error_months);
        assert!(a.iter().all(|r| (0.0..=1.0).contains(&r.interval_coverage)));
    }
}

// ---------------------------------------------------------------- snapshot count

/// Confirmed-claim count at one as-of date under one slack rule.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotCount {
    pub as_of: i64,
    pub strict_hits: usize,
    pub lenient_hits: usize,
}

/// Count hits in one fixed simulated world at each snapshot date, with zero
/// slack and with `slack_days`. Events that happen after a snapshot are not
/// visible at it.
pub fn snapshot_count(
    n: usize,
    seed: u64,
    made_on: i64,
    snapshots: &[i64],
    slack_days: i64,
) -> Vec<SnapshotCount> {
    // Far-future as-of so every event in the world has a known date.
    let world = synth_claims_made_on(Forecaster::Calibrated, n, seed, made_on + 100_000, made_on);
    snapshots
        .iter()
        .map(|&s| {
            let seen: Vec<Claim> = world
                .iter()
                .map(|c| {
                    let mut c = c.clone();
                    if let Outcome::Occurred(d) = c.outcome {
                        if d > s {
                            c.outcome = Outcome::NotYet;
                        }
                    }
                    c
                })
                .collect();
            let hits = |slack: i64| {
                seen.iter()
                    .filter(|c| status_with_slack(c, s, slack) == Status::Hit)
                    .count()
            };
            SnapshotCount {
                as_of: s,
                strict_hits: hits(0),
                lenient_hits: hits(slack_days),
            }
        })
        .collect()
}

// ---------------------------------------------------------------- window spread

/// Spread of fitted doubling time across start-year windows.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowSpread {
    pub median_range_days: f64,
    pub median_abs_error_days: Vec<f64>,
    pub share_range_at_least_20: f64,
}

/// Simulate `reps` frontiers `log2 h(t) = t / true_doubling_days * 30.4375`
/// over 72 months with triangular noise of half-width `noise`. Points are
/// every 6 months before month 36 and every 2 months from month 36 on. For
/// each start month in `starts` fit a line to all points from that month to
/// the end and convert the slope to a doubling time in days. The range is max
/// minus min across windows, per replicate.
pub fn window_spread(
    seed: u64,
    true_doubling_days: f64,
    noise: f64,
    reps: usize,
    starts: &[usize],
) -> WindowSpread {
    let days_per_month = 30.4375;
    let times: Vec<usize> = (0..=72)
        .filter(|&m| if m < 36 { m % 6 == 0 } else { m % 2 == 0 })
        .collect();
    let mut rng = Rng::new(seed);
    let mut ranges = Vec::with_capacity(reps);
    let mut errs: Vec<Vec<f64>> = vec![Vec::new(); starts.len()];
    for _ in 0..reps {
        let y: Vec<f64> = times
            .iter()
            .map(|&m| {
                m as f64 * days_per_month / true_doubling_days
                    + (rng.unit() + rng.unit() - 1.0) * noise
            })
            .collect();
        let mut est = Vec::with_capacity(starts.len());
        for (k, &s) in starts.iter().enumerate() {
            let xs: Vec<f64> = times
                .iter()
                .filter(|&&m| m >= s)
                .map(|&m| m as f64)
                .collect();
            let ys = &y[times.len() - xs.len()..];
            let slope = ols_slope(&xs, ys);
            let d = days_per_month / slope;
            errs[k].push((d - true_doubling_days).abs());
            est.push(d);
        }
        let mx = est.iter().cloned().fold(f64::MIN, f64::max);
        let mn = est.iter().cloned().fold(f64::MAX, f64::min);
        ranges.push(mx - mn);
    }
    let median = |v: &[f64]| {
        let mut e = v.to_vec();
        e.sort_by(|a, b| a.partial_cmp(b).unwrap());
        if e.len() % 2 == 1 {
            e[e.len() / 2]
        } else {
            (e[e.len() / 2 - 1] + e[e.len() / 2]) / 2.0
        }
    };
    WindowSpread {
        median_range_days: median(&ranges),
        median_abs_error_days: errs.iter().map(|e| median(e)).collect(),
        share_range_at_least_20: ranges.iter().filter(|&&r| r >= 20.0).count() as f64 / reps as f64,
    }
}

#[cfg(test)]
mod window_spread_tests {
    use super::*;

    #[test]
    fn deterministic_and_noise_free_has_no_spread() {
        let a = window_spread(7, 210.0, 0.3, 100, &[0, 12, 24, 36]);
        assert_eq!(a, window_spread(7, 210.0, 0.3, 100, &[0, 12, 24, 36]));
        let z = window_spread(7, 210.0, 0.0, 10, &[0, 12, 24, 36]);
        assert!(z.median_range_days < 1e-6);
        assert!(a.median_range_days > 0.0);
    }
}

// ---------------------------------------------------------------- label noise

/// Median log ratio of fitted to true horizons at one label-noise sigma.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelNoiseRow {
    pub sigma: f64,
    pub h50_log_ratio: f64,
    pub h80_log_ratio: f64,
    pub median_slope: f64,
}

/// Maximum-likelihood logistic fit `P(success) = sigmoid(a + b x)` by Newton
/// steps. Returns `None` when the data are separable or the fit diverges.
fn logistic_fit(x: &[f64], y: &[bool]) -> Option<(f64, f64)> {
    let (mut a, mut b) = (0.0_f64, 0.0_f64);
    for _ in 0..50 {
        let (mut ga, mut gb, mut haa, mut hab, mut hbb) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for (&xi, &yi) in x.iter().zip(y) {
            let p = 1.0 / (1.0 + (-(a + b * xi)).exp());
            let r = if yi { 1.0 } else { 0.0 } - p;
            let w = p * (1.0 - p);
            ga += r;
            gb += r * xi;
            haa += w;
            hab += w * xi;
            hbb += w * xi * xi;
        }
        let det = haa * hbb - hab * hab;
        if det.abs() < 1e-12 {
            return None;
        }
        let da = (hbb * ga - hab * gb) / det;
        let db = (haa * gb - hab * ga) / det;
        a += da;
        b += db;
        if !a.is_finite() || !b.is_finite() || b.abs() > 50.0 {
            return None;
        }
        if da.abs() < 1e-9 && db.abs() < 1e-9 {
            return Some((a, b));
        }
    }
    None
}

/// Simulate `reps` suites of `n_tasks` tasks with natural-log length uniform
/// on `[0, ln 1000]` (minutes), true success `sigmoid(slope * (ln h50 - x))`,
/// one attempt each. Fit a logistic of success on the noisy label
/// `x + sigma * N(0,1)` and report the median log ratio of the fitted to the
/// true 50% and 80% horizons, per sigma. Replicates that cannot be fitted
/// (separable data) are skipped.
pub fn label_noise(
    seed: u64,
    n_tasks: usize,
    reps: usize,
    true_slope: f64,
    true_h50: f64,
    sigmas: &[f64],
) -> Vec<LabelNoiseRow> {
    let hi = 1000.0_f64.ln();
    let ln50 = true_h50.ln();
    let ln80_true = ln50 - 4.0_f64.ln() / true_slope;
    sigmas
        .iter()
        .map(|&sigma| {
            let mut rng = Rng::new(seed);
            let (mut r50, mut r80, mut sl) = (Vec::new(), Vec::new(), Vec::new());
            for _ in 0..reps {
                let mut xs = Vec::with_capacity(n_tasks);
                let mut ys = Vec::with_capacity(n_tasks);
                for _ in 0..n_tasks {
                    let x = rng.unit() * hi;
                    let p = 1.0 / (1.0 + (-true_slope * (ln50 - x)).exp());
                    ys.push(rng.coin(p));
                    let g = (-2.0 * (1.0 - rng.unit()).ln()).sqrt()
                        * (2.0 * std::f64::consts::PI * rng.unit()).cos();
                    xs.push(x + sigma * g);
                }
                if let Some((a, b)) = logistic_fit(&xs, &ys) {
                    if b < 0.0 {
                        r50.push(-a / b - ln50);
                        r80.push((4.0_f64.ln() - a) / b - ln80_true);
                        sl.push(-b);
                    }
                }
            }
            LabelNoiseRow {
                sigma,
                h50_log_ratio: median(&mut r50),
                h80_log_ratio: median(&mut r80),
                median_slope: median(&mut sl),
            }
        })
        .collect()
}

#[cfg(test)]
mod label_noise_tests {
    use super::*;

    #[test]
    fn deterministic_and_noise_attenuates() {
        let a = label_noise(7, 200, 60, 1.0, 60.0, &[0.0, 1.0]);
        assert_eq!(a, label_noise(7, 200, 60, 1.0, 60.0, &[0.0, 1.0]));
        assert!(a[0].h80_log_ratio.abs() < 0.2);
        assert!(a[1].median_slope < a[0].median_slope);
    }

    #[test]
    fn logistic_fit_rejects_separable_data() {
        let x: Vec<f64> = (0..200).map(|i| i as f64 / 20.0).collect();
        let y: Vec<bool> = x.iter().map(|&v| v < 5.0).collect();
        assert!(logistic_fit(&x, &y).is_none());
    }
}

// ---------------------------------------------------------------- tasks per model

/// Result of the tasks-per-model sweep at one n.
#[derive(Debug, Clone, PartialEq)]
pub struct TasksPerModelRow {
    pub n_tasks: usize,
    pub wrong_sign_share: f64,
    pub median_abs_log_error: f64,
}

/// Simulate `reps` frontiers of `n_models` models spaced 6 months apart whose
/// true 50% horizon doubles every `true_doubling` months (4 minutes at month
/// 0), each given `n` log-uniform tasks (1 to 1000 minutes, slope 1.0 per
/// natural-log unit, one attempt). Fit a logistic per model, regress ln
/// horizon on date, and report the share of replicates with an undefined or
/// non-positive slope and the median absolute ln ratio of fitted to true
/// doubling time among the defined ones.
pub fn tasks_per_model(
    seed: u64,
    true_doubling: f64,
    n_models: usize,
    reps: usize,
    ns: &[usize],
) -> Vec<TasksPerModelRow> {
    let hi = 1000.0_f64.ln();
    ns.iter()
        .map(|&n| {
            let mut rng = Rng::new(seed);
            let (mut bad, mut errs) = (0usize, Vec::new());
            for _ in 0..reps {
                let (mut ts, mut hs) = (Vec::new(), Vec::new());
                for m in 0..n_models {
                    let t = 6.0 * m as f64;
                    let ln50 = 4.0_f64.ln() + t / true_doubling * std::f64::consts::LN_2;
                    let mut xs = Vec::with_capacity(n);
                    let mut ys = Vec::with_capacity(n);
                    for _ in 0..n {
                        let x = rng.unit() * hi;
                        let p = 1.0 / (1.0 + (-(ln50 - x)).exp());
                        xs.push(x);
                        ys.push(rng.coin(p));
                    }
                    if let Some((a, b)) = logistic_fit(&xs, &ys) {
                        if b < 0.0 {
                            ts.push(t);
                            hs.push(-a / b);
                        }
                    }
                }
                if ts.len() < 3 {
                    bad += 1;
                    continue;
                }
                let k = ts.len() as f64;
                let (mt, mh) = (ts.iter().sum::<f64>() / k, hs.iter().sum::<f64>() / k);
                let sxx: f64 = ts.iter().map(|t| (t - mt) * (t - mt)).sum();
                let sxy: f64 = ts.iter().zip(&hs).map(|(t, h)| (t - mt) * (h - mh)).sum();
                let slope = sxy / sxx;
                if slope <= 0.0 {
                    bad += 1;
                    continue;
                }
                errs.push(
                    ((std::f64::consts::LN_2 / slope) / true_doubling)
                        .ln()
                        .abs(),
                );
            }
            TasksPerModelRow {
                n_tasks: n,
                wrong_sign_share: bad as f64 / reps as f64,
                median_abs_log_error: median(&mut errs),
            }
        })
        .collect()
}

#[cfg(test)]
mod tasks_per_model_tests {
    use super::*;

    #[test]
    fn deterministic_and_more_tasks_help() {
        let a = tasks_per_model(7, 7.0, 8, 40, &[10, 200]);
        assert_eq!(a, tasks_per_model(7, 7.0, 8, 40, &[10, 200]));
        assert!(a[1].median_abs_log_error < a[0].median_abs_log_error);
        assert!(a[1].wrong_sign_share <= a[0].wrong_sign_share);
    }
}

/// One row of the difficulty-spread audit.
#[derive(Debug, Clone, PartialEq)]
pub struct DifficultyRow {
    pub tau: f64,
    pub marginal_h50: f64,
    pub marginal_h80: f64,
    pub h50_log_ratio: f64,
    pub h80_log_ratio: f64,
}

/// Marginal success probability at log length `x` when the log-odds carry an
/// N(0, tau^2) per-task offset, by trapezoid quadrature over the offset.
fn marginal_p(x: f64, slope: f64, ln50: f64, tau: f64) -> f64 {
    if tau == 0.0 {
        return 1.0 / (1.0 + (-slope * (ln50 - x)).exp());
    }
    let (steps, lim) = (400usize, 6.0_f64);
    let h = 2.0 * lim / steps as f64;
    let (mut num, mut den) = (0.0, 0.0);
    for i in 0..=steps {
        let g = -lim + h * i as f64;
        let w = (-0.5 * g * g).exp() * if i == 0 || i == steps { 0.5 } else { 1.0 };
        num += w / (1.0 + (-(slope * (ln50 - x) + tau * g)).exp());
        den += w;
    }
    num / den
}

/// Bisection for the log length where the marginal success rate equals `q`.
fn marginal_horizon(q: f64, slope: f64, ln50: f64, tau: f64) -> f64 {
    let (mut lo, mut hi) = (ln50 - 60.0, ln50 + 60.0);
    for _ in 0..100 {
        let mid = 0.5 * (lo + hi);
        if marginal_p(mid, slope, ln50, tau) > q {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// SIMULATED. Each task's log-odds of success is `slope * (ln h50 - x)` plus
/// an N(0, tau^2) offset; a two-parameter logistic of success on log length is
/// fitted and its 50% and 80% horizons compared with the true marginal ones.
/// Reports the median absolute log ratio per tau. Separable replicates are
/// skipped.
pub fn difficulty_spread(
    seed: u64,
    n_tasks: usize,
    reps: usize,
    slope: f64,
    h50: f64,
    taus: &[f64],
) -> Vec<DifficultyRow> {
    let hi = 1000.0_f64.ln();
    let ln50 = h50.ln();
    taus.iter()
        .map(|&tau| {
            let m50 = marginal_horizon(0.5, slope, ln50, tau);
            let m80 = marginal_horizon(0.8, slope, ln50, tau);
            let mut rng = Rng::new(seed);
            let (mut r50, mut r80) = (Vec::new(), Vec::new());
            for _ in 0..reps {
                let mut xs = Vec::with_capacity(n_tasks);
                let mut ys = Vec::with_capacity(n_tasks);
                for _ in 0..n_tasks {
                    let x = rng.unit() * hi;
                    let g = gauss(&mut rng);
                    let p = 1.0 / (1.0 + (-(slope * (ln50 - x) + tau * g)).exp());
                    ys.push(rng.coin(p));
                    xs.push(x);
                }
                if let Some((a, b)) = logistic_fit(&xs, &ys) {
                    if b < 0.0 {
                        r50.push(((-a / b) - m50).abs());
                        r80.push((((4.0_f64.ln() - a) / b) - m80).abs());
                    }
                }
            }
            DifficultyRow {
                tau,
                marginal_h50: m50.exp(),
                marginal_h80: m80.exp(),
                h50_log_ratio: median(&mut r50),
                h80_log_ratio: median(&mut r80),
            }
        })
        .collect()
}

#[cfg(test)]
mod difficulty_spread_tests {
    use super::*;

    #[test]
    fn marginal_matches_closed_form_at_zero_tau() {
        let h = marginal_horizon(0.8, 1.0, 60.0_f64.ln(), 0.0);
        assert!((h - (60.0_f64.ln() - 4.0_f64.ln())).abs() < 1e-9);
    }

    #[test]
    fn marginal_h50_is_symmetric_and_deterministic() {
        let a = difficulty_spread(7, 200, 20, 1.0, 60.0, &[0.0, 2.0]);
        assert_eq!(a, difficulty_spread(7, 200, 20, 1.0, 60.0, &[0.0, 2.0]));
        assert!((a[1].marginal_h50 - 60.0).abs() < 1e-6);
        assert!(a[1].marginal_h80 < a[0].marginal_h80);
    }
}
