use super::*;

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
