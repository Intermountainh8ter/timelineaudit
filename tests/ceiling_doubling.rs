use timelineaudit::ceiling_doubling;

#[test]
fn deterministic_and_uncapped_near_truth() {
    let a = ceiling_doubling(7, 6.0, &[0.25, 1.0]);
    assert_eq!(a, ceiling_doubling(7, 6.0, &[0.25, 1.0]));
    assert!(a.uncapped_bias_months.abs() < 0.5);
    assert_eq!(a.fits.len(), 2);
}

#[test]
fn lower_cap_overstates_more() {
    let a = ceiling_doubling(7, 6.0, &[0.25, 0.5, 0.75]);
    assert!(a.fits[0].bias_months > a.fits[1].bias_months);
    assert!(a.fits[1].bias_months > a.fits[2].bias_months);
    assert!(a.fits[0].capped_points > a.fits[2].capped_points);
}
