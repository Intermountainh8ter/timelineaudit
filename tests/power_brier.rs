use timelineaudit::{parse_date, power_brier, synth_claims, Forecaster};

fn as_of() -> i64 {
    parse_date("2026-01-01").unwrap()
}

#[test]
fn power_rises_with_claim_count_for_the_calibrated_forecaster() {
    let claims = synth_claims(Forecaster::Calibrated, 400, 7, as_of());
    let small = power_brier(&claims, as_of(), 20, 40, 200, 7).unwrap();
    let large = power_brier(&claims, as_of(), 100, 40, 200, 7).unwrap();
    assert!(small.excludes_zero_rate < large.excludes_zero_rate);
}

#[test]
fn the_coin_never_excludes_zero_and_is_reproducible() {
    let claims = synth_claims(Forecaster::Coin, 400, 7, as_of());
    let a = power_brier(&claims, as_of(), 50, 30, 200, 7).unwrap();
    assert_eq!(a.excludes_zero_rate, 0.0);
    assert_eq!(a, power_brier(&claims, as_of(), 50, 30, 200, 7).unwrap());
}

#[test]
fn too_few_claims_gives_none() {
    let claims = synth_claims(Forecaster::Calibrated, 10, 7, as_of());
    assert!(power_brier(&claims, as_of(), 50, 5, 50, 7).is_none());
}
