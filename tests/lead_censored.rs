use timelineaudit::{lead_audit_censored, parse_date, synth_claims_made_on, Forecaster};

fn as_of() -> i64 {
    parse_date("2025-09-01").unwrap()
}

#[test]
fn deterministic_and_counts_add_up() {
    let claims = synth_claims_made_on(Forecaster::Calibrated, 400, 7, as_of(), as_of() - 450);
    let a = lead_audit_censored(&claims, as_of());
    let b = lead_audit_censored(&claims, as_of());
    assert_eq!(a, b);
    assert_eq!(a.iter().map(|r| r.claims).sum::<usize>(), 400);
    for r in &a {
        assert!(r.open <= r.claims);
    }
}

#[test]
fn some_claims_stay_open() {
    let claims = synth_claims_made_on(Forecaster::Calibrated, 400, 7, as_of(), as_of() - 450);
    let rows = lead_audit_censored(&claims, as_of());
    assert!(rows[1].open > 0);
}
