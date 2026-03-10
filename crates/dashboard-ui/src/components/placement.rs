use crate::chart;
use crate::models::PlacementPoint;
use gloo_timers::callback::Timeout;
use leptos::prelude::*;
use std::collections::BTreeSet;

#[component]
pub fn PlacementChart(data: Vec<PlacementPoint>) -> impl IntoView {
    let option_json = build_option(&data);
    let json_clone = option_json.clone();

    Timeout::new(0, move || {
        chart::update_chart("placement-chart", &json_clone);
    })
    .forget();

    view! {
        <div class="bg-gray-800 rounded-lg p-4 border border-gray-700">
            <h2 class="text-lg font-semibold mb-2">"Placement Type Mix (30 days)"</h2>
            <div id="placement-chart" style="width: 100%; height: 320px;"></div>
        </div>
    }
}

fn build_option(data: &[PlacementPoint]) -> String {
    let markets: Vec<&str> = {
        let set: BTreeSet<&str> = data.iter().map(|p| p.marketplace.as_str()).collect();
        set.into_iter().collect()
    };
    let placement_types: Vec<&str> = {
        let set: BTreeSet<&str> = data.iter().map(|p| p.placement_type.as_str()).collect();
        set.into_iter().collect()
    };

    let colors = [
        "#60a5fa", "#f97316", "#a78bfa", "#34d399", "#f472b6", "#fbbf24",
    ];

    let mut series = Vec::new();
    for (i, &pt) in placement_types.iter().enumerate() {
        let values: Vec<i64> = markets
            .iter()
            .map(|&mkt| {
                data.iter()
                    .find(|p| p.marketplace == mkt && p.placement_type == pt)
                    .map(|p| p.count)
                    .unwrap_or(0)
            })
            .collect();
        series.push(serde_json::json!({
            "name": pt,
            "type": "bar",
            "stack": "total",
            "data": values,
            "itemStyle": { "color": colors[i % colors.len()] },
        }));
    }

    serde_json::json!({
        "tooltip": { "trigger": "axis", "axisPointer": { "type": "shadow" } },
        "legend": { "data": placement_types, "textStyle": { "color": "#9ca3af" }, "type": "scroll" },
        "grid": { "left": "3%", "right": "4%", "bottom": "3%", "containLabel": true },
        "xAxis": { "type": "category", "data": markets, "axisLabel": { "color": "#9ca3af" } },
        "yAxis": { "type": "value", "name": "Count", "nameTextStyle": { "color": "#9ca3af" }, "axisLabel": { "color": "#9ca3af" } },
        "series": series,
    })
    .to_string()
}
