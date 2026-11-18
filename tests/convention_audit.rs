use timelineaudit::{convention_audit, parse_date, synth_claims, Forecaster, CLASSES};

fn as_of() -> i64 {
    parse_date("2026-01-01").unwrap()
}

#[test]
fn one_row_per_class_and_deterministic() {
    let claims = synth_claims(Forecaster::Calibrated, 400, 7, as_of());
    let a = convention_audit(&claims, as_of());
    let b = convention_audit(&claims, as_of());
    assert_eq!(a.len(), CLASSES.len());
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(x.class, y.class);
        assert_eq!(x.resolved, y.resolved);
        assert_eq!(x.censored, y.censored);
    }
}

#[test]
fn keeping_lower_bounds_never_lowers_the_median() {
    let claims = synth_claims(Forecaster::Calibrated, 400, 7, as_of());
    for r in convention_audit(&claims, as_of()) {
        if r.gap_months.is_finite() {
            // Censored bounds are overdue time, at least as large as any
            // observed slip that is on time or earlier.
            assert!(r.censored <= r.resolved);
            assert!(r.gap_months >= -1e-9, "{}: {}", r.class, r.gap_months);
        }
    }
}
