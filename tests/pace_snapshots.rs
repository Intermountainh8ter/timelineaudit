use timelineaudit::{pace_snapshot, parse_date, synth_claims_made_on, Forecaster};

fn world(as_of: &str) -> Vec<timelineaudit::Claim> {
    let late = parse_date("2025-09-01").unwrap();
    synth_claims_made_on(
        Forecaster::Calibrated,
        400,
        7,
        parse_date(as_of).unwrap(),
        late - 450,
    )
}

#[test]
fn deterministic() {
    let c = world("2025-03-01");
    let a = pace_snapshot(&c, parse_date("2025-03-01").unwrap());
    assert_eq!(a, pace_snapshot(&c, parse_date("2025-03-01").unwrap()));
    assert_eq!(a.claims, 400);
}

#[test]
fn resolved_share_rises_with_later_as_of() {
    let e = pace_snapshot(&world("2025-03-01"), parse_date("2025-03-01").unwrap());
    let l = pace_snapshot(&world("2025-09-01"), parse_date("2025-09-01").unwrap());
    assert!(l.resolved_share > e.resolved_share);
    assert!(l.events >= e.events);
}
