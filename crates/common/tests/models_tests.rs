use mts_common::models::{MonitorState, PlacementType, SearchResult};

#[test]
fn search_result_serialization() {
    let result = SearchResult {
        asin: "B0ABCDEF12".to_string(),
        title: "Huawei Watch GT 4".to_string(),
        position: 3,
        page: 1,
        position_in_page: 3,
        is_sponsored: true,
        placement_type: Some(PlacementType::SponsoredProduct),
        ..Default::default()
    };

    let json = serde_json::to_string(&result).unwrap();
    let deserialized: SearchResult = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.asin, "B0ABCDEF12");
    assert_eq!(deserialized.title, "Huawei Watch GT 4");
    assert_eq!(deserialized.position, 3);
    assert!(deserialized.is_sponsored);
}

#[test]
fn monitor_state_default_is_empty() {
    let state = MonitorState::default();
    assert!(state.keywords.is_empty());
}

#[test]
fn monitor_state_round_trip() {
    let state = MonitorState::default();
    let json = serde_json::to_string(&state).unwrap();
    let loaded: MonitorState = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.keywords.len(), 0);
}
