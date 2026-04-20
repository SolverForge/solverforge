use super::*;

#[test]
fn enumerate_2opt_count() {
    let patterns = enumerate_reconnections(2);
    assert_eq!(patterns.len(), 1);
    assert!(patterns[0].should_reverse(1));
}

#[test]
fn enumerate_3opt_count() {
    let patterns = enumerate_reconnections(3);
    assert_eq!(patterns.len(), 7);
}

#[test]
fn enumerate_matches_static_3opt() {
    let dynamic = enumerate_reconnections(3);
    assert_eq!(dynamic.len(), THREE_OPT_RECONNECTIONS.len());
}

#[test]
fn enumerate_4opt_count() {
    let patterns = enumerate_reconnections(4);
    assert_eq!(patterns.len(), 47);
}

#[test]
fn enumerate_5opt_count() {
    let patterns = enumerate_reconnections(5);
    assert_eq!(patterns.len(), 383);
}

#[test]
fn no_identity_in_patterns() {
    for k in 2..=5 {
        let patterns = enumerate_reconnections(k);
        for p in &patterns {
            assert!(!p.is_identity(), "Found identity in {}-opt patterns", k);
        }
    }
}

#[test]
fn static_patterns_not_identity() {
    for p in THREE_OPT_RECONNECTIONS {
        assert!(!p.is_identity());
    }
}
