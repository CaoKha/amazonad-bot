use std::path::PathBuf;

use mts_common::models::MonitorState;
use mts_common::state::StateManager;

fn temp_state_path() -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("mts_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(format!("state_{nanos}.json"))
}

fn sample_state() -> MonitorState {
    MonitorState::default()
}

#[test]
fn round_trip() {
    let path = temp_state_path();
    let manager = StateManager::new(path.clone());
    let original = sample_state();

    manager.save(&original).unwrap();
    let loaded = manager.load().unwrap().expect("should load saved state");

    assert_eq!(loaded.keywords.len(), original.keywords.len());

    let _ = std::fs::remove_file(&path);
}

#[test]
fn missing_file() {
    let path = PathBuf::from("/tmp/mts_nonexistent_state_99999.json");
    let _ = std::fs::remove_file(&path);
    let manager = StateManager::new(path);

    let result = manager.load().unwrap();
    assert!(result.is_none(), "missing file should return Ok(None)");
}

#[test]
fn unknown_keyword_defaults() {
    // An absent keyword key should yield brand_ad_visible=false via unwrap_or_default
    let state = MonitorState::default();
    let kw_state = state.keywords.get("montre connectee").cloned().unwrap_or_default();
    assert!(!kw_state.brand_ad_visible, "Unknown keyword should default to not visible");
    assert!(kw_state.brand_positions.is_empty());
    assert!(kw_state.last_results.is_empty());
}

#[test]
fn corrupt_json() {
    let path = temp_state_path();
    std::fs::write(&path, "this is not json {{{").unwrap();

    let manager = StateManager::new(path.clone());
    let result = manager.load().unwrap();
    assert!(
        result.is_none(),
        "corrupt JSON should return Ok(None), not panic"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn atomic_tmp_cleanup() {
    let path = temp_state_path();
    let manager = StateManager::new(path.clone());
    let state = sample_state();

    manager.save(&state).unwrap();

    let tmp_path = path.with_extension("json.tmp");
    assert!(
        !tmp_path.exists(),
        "temp file should be cleaned up after atomic save"
    );

    let _ = std::fs::remove_file(&path);
}
