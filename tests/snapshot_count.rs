use timelineaudit::{parse_date, snapshot_count};

fn snaps() -> Vec<i64> {
    ["2025-01-01", "2025-07-01", "2026-01-01", "2026-07-01"]
        .iter()
        .map(|d| parse_date(d).unwrap())
        .collect()
}

#[test]
fn deterministic() {
    let m = parse_date("2024-06-01").unwrap();
    assert_eq!(
        snapshot_count(400, 7, m, &snaps(), 90),
        snapshot_count(400, 7, m, &snaps(), 90)
    );
}

#[test]
fn lenient_never_below_strict_and_counts_grow() {
    let m = parse_date("2024-06-01").unwrap();
    let r = snapshot_count(400, 7, m, &snaps(), 90);
    for x in &r {
        assert!(x.lenient_hits >= x.strict_hits);
    }
    assert!(r.last().unwrap().strict_hits >= r[0].strict_hits);
}
