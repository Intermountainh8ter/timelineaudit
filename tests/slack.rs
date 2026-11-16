use timelineaudit::{parse_date, slack_sensitivity, synth_claims, Forecaster};

fn as_of() -> i64 {
    parse_date("2026-01-01").unwrap()
}

#[test]
fn slack_zero_flips_nothing_and_flips_are_non_decreasing() {
    let claims = synth_claims(Forecaster::Calibrated, 400, 7, as_of());
    let rows = slack_sensitivity(&claims, as_of(), &[0, 30, 90, 180]);
    assert_eq!(rows[0].miss_to_hit_count, 0);
    assert_eq!(rows[0].miss_to_open_count, 0);
    for w in rows.windows(2) {
        assert!(w[1].miss_to_hit_count >= w[0].miss_to_hit_count);
        assert!(w[1].hits >= w[0].hits);
    }
}

#[test]
fn counts_cover_every_claim() {
    let claims = synth_claims(Forecaster::Calibrated, 100, 3, as_of());
    for r in slack_sensitivity(&claims, as_of(), &[0, 90]) {
        assert_eq!(r.hits + r.misses + r.open, 100);
    }
}
