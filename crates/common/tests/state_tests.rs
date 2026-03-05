use std::path::PathBuf;

use chrono::Utc;
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
    MonitorState {
        huawei_ad_visible: true,
        huawei_positions: vec![2, 5],
        total_results_scraped: 48,
        updated_at: Utc::now(),
    }
}

#[test]
fn round_trip() {
    let path = temp_state_path();
    let manager = StateManager::new(path.clone());
    let original = sample_state();

    manager.save(&original).unwrap();
    let loaded = manager.load().unwrap().expect("should load saved state");

    assert_eq!(loaded.huawei_ad_visible, original.huawei_ad_visible);
    assert_eq!(loaded.huawei_positions, original.huawei_positions);
    assert_eq!(loaded.total_results_scraped, original.total_results_scraped);
    assert_eq!(loaded.updated_at, original.updated_at);

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
