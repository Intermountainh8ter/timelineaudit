use timelineaudit::{group_gap, parse_date};

#[test]
fn deterministic_and_counts_bounded() {
    let d = parse_date("2025-09-01").unwrap();
    let a = group_gap(400, 7, d, 450, 2.0);
    assert_eq!(a, group_gap(400, 7, d, 450, 2.0));
    assert!(a.fast_resolved <= 200 && a.slow_resolved <= 200);
    assert_eq!(a.planted_gap_months, 4.0);
}

#[test]
fn zero_planted_gap_has_planted_zero() {
    let d = parse_date("2030-01-01").unwrap();
    let g = group_gap(2000, 3, d, 450, 0.0);
    assert_eq!(g.planted_gap_months, 0.0);
    assert!(g.measured_gap_months.abs() < 1.0);
}
