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
    };

    let json = serde_json::to_string(&result).unwrap();
    let deserialized: SearchResult = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.asin, "B0ABCDEF12");
    assert_eq!(deserialized.title, "Huawei Watch GT 4");
    assert_eq!(deserialized.position, 3);
    assert!(deserialized.is_sponsored);
}

#[test]
fn monitor_state_round_trip() {
    let state = MonitorState {
        huawei_ad_visible: true,
        huawei_positions: vec![1, 3],
        total_results_scraped: 48,
        updated_at: chrono::Utc::now(),
    };

    let json = serde_json::to_string(&state).unwrap();
    let deserialized: MonitorState = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.huawei_ad_visible, state.huawei_ad_visible);
    assert_eq!(deserialized.huawei_positions, state.huawei_positions);
    assert_eq!(
        deserialized.total_results_scraped,
        state.total_results_scraped
    );
    assert_eq!(deserialized.updated_at, state.updated_at);
}

#[test]
fn monitor_state_no_ad() {
    let state = MonitorState {
        huawei_ad_visible: false,
        huawei_positions: vec![],
        total_results_scraped: 24,
        updated_at: chrono::Utc::now(),
    };

    let json = serde_json::to_string(&state).unwrap();
    let deserialized: MonitorState = serde_json::from_str(&json).unwrap();

    assert!(!deserialized.huawei_ad_visible);
    assert!(deserialized.huawei_positions.is_empty());
}
