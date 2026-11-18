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
