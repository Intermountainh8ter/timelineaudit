//! The degenerate-forecaster audit and the scorer's behaviour on tables whose
//! right answer is known by construction.

use timelineaudit::{
    claims_to_tsv, median_slip, parse_claims, parse_date, score, synth_claims, Forecaster, Group,
    Slip, Status, DAYS_PER_MONTH, EARLY_DAYS,
};

const N: usize = 2000;
const RESAMPLES: usize = 400;

fn as_of() -> i64 {
    parse_date("2026-01-01").unwrap()
}

fn overall(f: Forecaster) -> Group {
    score(&synth_claims(f, N, 7, as_of()), as_of(), RESAMPLES, 7).remove(0)
}

fn row(id: &str, made: &str, due: &str, prob: &str, outcome: &str) -> String {
    format!("{id}\tsrc\tcapability\t{made}\t{due}\t{prob}\t{outcome}\thttps://example.invalid/x\n")
}

#[test]
fn only_the_forecaster_that_knows_the_chances_shows_brier_skill() {
    let cal = overall(Forecaster::Calibrated);
    assert!(cal.brier_skill_vs_coin > 0.15, "{cal:?}");
    assert!(cal.brier_skill_ci.0 > 0.0, "{cal:?}");
    for f in [Forecaster::AlwaysYes, Forecaster::AlwaysNo] {
        let g = overall(f);
        assert!(g.brier_skill_ci.1 < 0.0, "{} {g:?}", f.name());
    }
    let coin = overall(Forecaster::Coin);
    assert_eq!(coin.brier_skill_vs_coin, 0.0);
    assert_eq!(coin.brier_skill_ci, (0.0, 0.0));
}

#[test]
fn hit_rate_alone_cannot_tell_the_forecasters_apart() {
    // Same world, same dates: the hit rate is a property of the events, not
    // of the probabilities attached to them. This is why it is never read as
    // a measure of forecasting skill on its own.
    let cal = overall(Forecaster::Calibrated);
    for f in [
        Forecaster::AlwaysYes,
        Forecaster::AlwaysNo,
        Forecaster::Coin,
    ] {
        assert_eq!(overall(f).hit_rate, cal.hit_rate, "{}", f.name());
    }
    assert!((cal.hit_rate - 0.5).abs() < 0.05, "{cal:?}");
}

#[test]
fn stating_every_date_too_early_shows_as_slip_and_as_fewer_hits() {
    let cal = overall(Forecaster::Calibrated);
    let early = overall(Forecaster::Early);
    let shift = EARLY_DAYS as f64 / DAYS_PER_MONTH;
    assert!(
        (early.median_slip_months - cal.median_slip_months - shift).abs() < 1e-9,
        "{} vs {}",
        early.median_slip_months,
        cal.median_slip_months
    );
    assert!(early.hit_rate < cal.hit_rate - 0.1, "{early:?}");
    assert!(early.brier > cal.brier, "{early:?}");
}

#[test]
fn status_follows_from_the_dates() {
    let text = [
        row("hit", "2025-01-01", "2025-06-01", "-", "2025-05-01"),
        row("on-the-day", "2025-01-01", "2025-06-01", "-", "2025-06-01"),
        row("late", "2025-01-01", "2025-06-01", "-", "2025-07-01"),
        row("overdue", "2025-01-01", "2025-06-01", "-", "-"),
        row("open", "2025-01-01", "2027-01-01", "-", "-"),
        row("early-hit", "2025-01-01", "2027-01-01", "-", "2025-03-01"),
        row("murky", "2025-01-01", "2025-06-01", "-", "unclear"),
    ]
    .concat();
    let claims = parse_claims(&text, as_of()).unwrap();
    let got: Vec<Status> = claims.iter().map(|c| c.status(as_of())).collect();
    use Status::*;
    assert_eq!(got, [Hit, Hit, Miss, Miss, Open, Hit, Unclear]);
    let g = score(&claims, as_of(), 50, 7).remove(0);
    assert_eq!(
        (g.n_claims, g.n_open, g.n_unclear, g.n_resolved),
        (7, 1, 1, 5)
    );
    assert_eq!(g.hit_rate, 0.6);
    assert_eq!((g.n_slip, g.n_censored), (5, 1));
    // No probabilities were stated, so there is no Brier score to report.
    assert_eq!(g.n_with_probability, 0);
    assert!(g.brier.is_nan());
}

#[test]
fn brier_uses_only_resolved_claims_that_carried_a_probability() {
    let text = [
        row("a", "2025-01-01", "2025-06-01", "0.9", "2025-05-01"), // (0.9-1)^2
        row("b", "2025-01-01", "2025-06-01", "0.8", "-"),          // (0.8-0)^2
        row("c", "2025-01-01", "2025-06-01", "-", "2025-05-01"),
        row("d", "2025-01-01", "2027-06-01", "0.5", "-"),
    ]
    .concat();
    let g = score(&parse_claims(&text, as_of()).unwrap(), as_of(), 50, 7).remove(0);
    assert_eq!(g.n_with_probability, 2);
    assert!((g.brier - (0.01 + 0.64) / 2.0).abs() < 1e-12);
    assert!((g.brier_skill_vs_coin - (1.0 - 0.325 / 0.25)).abs() < 1e-12);
}

#[test]
fn an_overdue_event_makes_the_median_slip_a_lower_bound_only_when_it_matters() {
    let s = |months, censored| Slip { months, censored };
    // The overdue claim's bound is already above the median: exact.
    assert_eq!(
        median_slip(&[s(-1.0, false), s(2.0, false), s(9.0, true)]),
        (2.0, false)
    );
    // Its true slip could land on either side of the median: lower bound.
    assert_eq!(
        median_slip(&[s(1.0, true), s(2.0, false), s(9.0, false)]),
        (2.0, true)
    );
    // Most claims are overdue: the median is one of the bounds.
    assert_eq!(
        median_slip(&[s(-1.0, false), s(4.0, true), s(6.0, true)]),
        (4.0, true)
    );
    assert_eq!(median_slip(&[s(3.0, false)]), (3.0, false));
    assert!(median_slip(&[]).0.is_nan());
}

#[test]
fn short_and_long_lead_claims_are_scored_apart() {
    let text = [
        row("s1", "2024-01-01", "2024-06-01", "-", "2024-05-01"),
        row("s2", "2024-01-01", "2024-12-31", "-", "2024-05-01"),
        row("l1", "2024-01-01", "2025-01-01", "-", "-"),
        row("l2", "2024-01-01", "2025-06-01", "-", "2025-05-01"),
    ]
    .concat();
    let g = score(&parse_claims(&text, as_of()).unwrap(), as_of(), 50, 7).remove(0);
    assert_eq!((g.n_short_lead, g.hit_rate_short_lead), (2, 1.0));
    assert_eq!((g.n_long_lead, g.hit_rate_long_lead), (2, 0.5));
}

#[test]
fn claims_that_are_not_forecasts_are_rejected_with_the_line_number() {
    let reject = |line: String, needle: &str| {
        let err = parse_claims(&format!("# header\n{line}"), as_of()).unwrap_err();
        assert!(err.contains("line 2") && err.contains(needle), "{err}");
    };
    // The event had already happened when the claim was written.
    reject(
        row("x", "2025-03-01", "2025-06-01", "-", "2025-02-01"),
        "not a forecast",
    );
    // An outcome nobody can have observed yet.
    reject(
        row("x", "2025-03-01", "2026-06-01", "-", "2026-02-01"),
        "after the scoring date",
    );
    reject(
        row("x", "2026-03-01", "2026-06-01", "-", "-"),
        "after the scoring date",
    );
    reject(
        row("x", "2025-03-01", "2025-01-01", "-", "-"),
        "before the claim was made",
    );
    reject(
        row("x", "2025-03-01", "2025-06-01", "1.2", "-"),
        "outside 0..1",
    );
    reject(row("x", "2025-02-30", "2025-06-01", "-", "-"), "YYYY-MM-DD");
    reject(
        "x\tsrc\tvibes\t2025-01-01\t2025-06-01\t-\t-\thttps://e.invalid\n".into(),
        "vibes",
    );
    reject(
        "x\tsrc\tsafety\t2025-01-01\t2025-06-01\t-\t-\tsomeone said\n".into(),
        "source URL",
    );
    reject("x\tsrc\tsafety\t2025-01-01\n".into(), "8");
    let twice = row("x", "2025-01-01", "2025-06-01", "-", "-").repeat(2);
    assert!(parse_claims(&twice, as_of())
        .unwrap_err()
        .contains("repeated"));
}

#[test]
fn point_estimates_do_not_depend_on_row_order() {
    let mut claims = synth_claims(Forecaster::Calibrated, 301, 3, as_of());
    let a = score(&claims, as_of(), 0, 7).remove(0);
    claims.reverse();
    let b = score(&claims, as_of(), 0, 7).remove(0);
    assert_eq!(a.hit_rate, b.hit_rate);
    assert_eq!(a.median_slip_months, b.median_slip_months);
    assert!((a.brier - b.brier).abs() < 1e-12);
}

#[test]
fn groups_cover_every_source_and_class() {
    let mut claims = synth_claims(Forecaster::Calibrated, 60, 3, as_of());
    claims.extend(synth_claims(Forecaster::Coin, 60, 3, as_of()));
    let names: Vec<String> = score(&claims, as_of(), 20, 7)
        .into_iter()
        .map(|g| g.name)
        .collect();
    assert_eq!(names[0], "overall");
    assert!(names.contains(&"source:calibrated".to_string()));
    assert!(names.contains(&"source:coin".to_string()));
    assert_eq!(names.iter().filter(|n| n.starts_with("class:")).count(), 6);
}

#[test]
fn scoring_is_identical_for_the_same_seed() {
    let claims = synth_claims(Forecaster::Early, 300, 11, as_of());
    assert_eq!(
        score(&claims, as_of(), 200, 7),
        score(&claims, as_of(), 200, 7)
    );
}

#[test]
fn the_claim_table_round_trips_through_tsv() {
    let claims = synth_claims(Forecaster::Calibrated, 40, 2, as_of());
    assert_eq!(
        parse_claims(&claims_to_tsv(&claims), as_of()).unwrap(),
        claims
    );
}
