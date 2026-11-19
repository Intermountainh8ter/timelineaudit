use timelineaudit::pace_ratio_conventions;

#[test]
fn deterministic_and_counts() {
    let a = pace_ratio_conventions(7, 40, 0.65, 0.2);
    assert_eq!(a, pace_ratio_conventions(7, 40, 0.65, 0.2));
    assert_eq!(a.n, 40);
    assert!(a.misgraded <= 40);
}

#[test]
fn no_perturbation_flags_none_and_spread_is_max_minus_min() {
    let a = pace_ratio_conventions(7, 40, 0.65, 0.0);
    assert_eq!(a.misgraded, 0);
    let v = [a.mean_of_ratios, a.ratio_of_sums, a.median_of_ratios];
    let hi = v.iter().cloned().fold(f64::MIN, f64::max);
    let lo = v.iter().cloned().fold(f64::MAX, f64::min);
    assert!((a.max_minus_min - (hi - lo)).abs() < 1e-12);
}
