use mts_common::models::{CheckOutcome, MonitorState};

fn make_state(huawei_visible: bool, positions: Vec<usize>) -> MonitorState {
    MonitorState {
        huawei_ad_visible: huawei_visible,
        huawei_positions: positions,
        total_results_scraped: 48,
        updated_at: chrono::Utc::now(),
    }
}

fn determine_outcome(prev: &MonitorState, current: &MonitorState) -> &'static str {
    if !prev.huawei_ad_visible && current.huawei_ad_visible {
        "appeared"
    } else if prev.huawei_ad_visible && !current.huawei_ad_visible {
        "disappeared"
    } else {
        "no_change"
    }
}

#[test]
fn ad_appeared_when_was_absent() {
    let prev = make_state(false, vec![]);
    let current = make_state(true, vec![2]);
    assert_eq!(determine_outcome(&prev, &current), "appeared");
}

#[test]
fn ad_disappeared_when_was_present() {
    let prev = make_state(true, vec![1]);
    let current = make_state(false, vec![]);
    assert_eq!(determine_outcome(&prev, &current), "disappeared");
}

#[test]
fn no_change_when_ad_stays_visible() {
    let prev = make_state(true, vec![1]);
    let current = make_state(true, vec![1]);
    assert_eq!(determine_outcome(&prev, &current), "no_change");
}

#[test]
fn no_change_when_ad_stays_absent() {
    let prev = make_state(false, vec![]);
    let current = make_state(false, vec![]);
    assert_eq!(determine_outcome(&prev, &current), "no_change");
}

#[test]
fn no_change_when_position_changes_but_still_visible() {
    let prev = make_state(true, vec![1]);
    let current = make_state(true, vec![3]);
    assert_eq!(determine_outcome(&prev, &current), "no_change");
}

#[test]
fn check_outcome_variants_are_debug() {
    let outcomes: Vec<CheckOutcome> = vec![
        CheckOutcome::AdAppeared {
            positions: vec![1],
            sample_title: "Huawei Watch".to_string(),
        },
        CheckOutcome::AdDisappeared,
        CheckOutcome::NoChange,
        CheckOutcome::ScrapeError("test error".to_string()),
        CheckOutcome::FirstRun,
    ];
    for o in &outcomes {
        let _ = format!("{:?}", o);
    }
}
