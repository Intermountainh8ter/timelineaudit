use timelineaudit::{parse_date, power_brier, randomise_probs, synth_claims, Forecaster};

fn as_of() -> i64 {
    parse_date("2026-01-01").unwrap()
}

#[test]
fn random_probabilities_do_not_exclude_zero() {
    let base = synth_claims(Forecaster::Calibrated, 400, 7, as_of());
    let claims = randomise_probs(&base, 7);
    let p = power_brier(&claims, as_of(), 50, 30, 200, 7).unwrap();
    assert!(p.excludes_zero_rate <= 0.05);
    assert!(p.median_skill < 0.0);
}

#[test]
fn randomising_is_reproducible_and_changes_probabilities() {
    let base = synth_claims(Forecaster::Calibrated, 50, 7, as_of());
    let a = randomise_probs(&base, 7);
    let b = randomise_probs(&base, 7);
    assert_eq!(a, b);
    assert!(a.iter().zip(&base).any(|(x, y)| x.prob != y.prob));
}
