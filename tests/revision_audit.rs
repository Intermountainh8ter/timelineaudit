use timelineaudit::revision_audit;

#[test]
fn deterministic_and_half_scores_zero() {
    let a = revision_audit(7, 400);
    assert_eq!(a, revision_audit(7, 400));
    assert!(a.always_half_skill.abs() < 1e-12);
}

#[test]
fn informative_revision_beats_prior_and_noise_does_not() {
    let a = revision_audit(7, 400);
    assert!(a.revised_skill - a.prior_skill > 0.05);
    assert!(a.noise_revised_skill - a.prior_skill <= 0.02);
}
