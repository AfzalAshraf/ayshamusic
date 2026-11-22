//! Preserve the standalone/no-public-proxy policy when reconciling old main.
#[test]
fn production_resolver_does_not_route_bot_challenges_through_public_proxies() {
    let source = include_str!("../src/youtube.rs");
    for forbidden in ["resolve_via_public", "AYSHA_INVIDIOUS", "invidious_bases", "local=true"] {
        assert!(!source.contains(forbidden), "public proxy fallback must remain disabled: {forbidden}");
    }
}
