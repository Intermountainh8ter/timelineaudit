use timelineaudit::{aggregation_audit, parse_date, synth_claims, Forecaster};

fn as_of() -> i64 {
    parse_date("2026-01-01").unwrap()
}

#[test]
fn gap_is_a_rate_difference_and_deterministic() {
    let claims = synth_claims(Forecaster::Calibrated, 400, 7, as_of());
    let a = aggregation_audit(&claims, as_of(), 15, 50, 7).unwrap();
    let b = aggregation_audit(&claims, as_of(), 15, 50, 7).unwrap();
    assert_eq!(a, b);
    assert!((0.0..=1.0).contains(&a));
}

#[test]
fn pool_smaller_than_pick_is_none() {
    let claims = synth_claims(Forecaster::Calibrated, 10, 7, as_of());
    assert!(aggregation_audit(&claims, as_of(), 500, 5, 7).is_none());
}
