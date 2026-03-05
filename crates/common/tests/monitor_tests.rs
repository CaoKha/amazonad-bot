use mts_common::models::{CheckOutcome, KeywordState};

fn make_keyword_state(brand_visible: bool) -> KeywordState {
    KeywordState {
        brand_ad_visible: brand_visible,
        brand_positions: vec![],
        last_changed: None,
        last_checked: None,
        last_results: vec![],
    }
}

fn determine_outcome(prev: &KeywordState, current: &KeywordState) -> &'static str {
    if !prev.brand_ad_visible && current.brand_ad_visible {
        "appeared"
    } else if prev.brand_ad_visible && !current.brand_ad_visible {
        "disappeared"
    } else {
        "no_change"
    }
}

#[test]
fn ad_appeared_when_was_absent() {
    let prev = make_keyword_state(false);
    let current = make_keyword_state(true);
    assert_eq!(determine_outcome(&prev, &current), "appeared");
}

#[test]
fn ad_disappeared_when_was_present() {
    let prev = make_keyword_state(true);
    let current = make_keyword_state(false);
    assert_eq!(determine_outcome(&prev, &current), "disappeared");
}

#[test]
fn no_change_when_ad_stays_visible() {
    let prev = make_keyword_state(true);
    let current = make_keyword_state(true);
    assert_eq!(determine_outcome(&prev, &current), "no_change");
}

#[test]
fn no_change_when_ad_stays_absent() {
    let prev = make_keyword_state(false);
    let current = make_keyword_state(false);
    assert_eq!(determine_outcome(&prev, &current), "no_change");
}

#[test]
fn no_change_when_position_changes_but_still_visible() {
    // Both visible — outcome is no_change regardless of position
    let prev = make_keyword_state(true);
    let current = make_keyword_state(true);
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
