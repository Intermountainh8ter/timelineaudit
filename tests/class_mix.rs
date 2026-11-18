use timelineaudit::{class_mix, parse_date, synth_claims, Forecaster};

fn as_of() -> i64 {
    parse_date("2026-01-01").unwrap()
}

#[test]
fn class_mix_is_seeded_and_covers_each_share() {
    let claims = synth_claims(Forecaster::Calibrated, 400, 7, as_of());
    let a = class_mix(&claims, as_of(), &[0.1, 0.5, 0.9], 20, 7).unwrap();
    let b = class_mix(&claims, as_of(), &[0.1, 0.5, 0.9], 20, 7).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.len(), 3);
    assert!(a.iter().all(|r| r.1.is_finite()));
}

#[test]
fn class_mix_is_none_without_a_capability_pool() {
    let claims: Vec<_> = synth_claims(Forecaster::Calibrated, 50, 7, as_of())
        .into_iter()
        .filter(|c| c.class != "capability")
        .collect();
    assert!(class_mix(&claims, as_of(), &[0.5], 5, 7).is_none());
}
