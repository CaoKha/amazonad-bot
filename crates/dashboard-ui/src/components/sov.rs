use crate::chart;
use crate::models::SovPoint;
use gloo_timers::callback::Timeout;
use leptos::prelude::*;
use std::collections::BTreeSet;

#[component]
pub fn SovChart(data: Vec<SovPoint>) -> impl IntoView {
    let option_json = build_option(&data);
    let json_clone = option_json.clone();

    // Defer chart update to after DOM mount
    Timeout::new(0, move || {
        chart::update_chart("sov-chart", &json_clone);
    })
    .forget();

    view! {
        <div class="bg-gray-800 rounded-lg p-4 border border-gray-700">
            <h2 class="text-lg font-semibold mb-2">"SOV Trend (30 days)"</h2>
            <div id="sov-chart" style="width: 100%; height: 320px;"></div>
        </div>
    }
}

fn build_option(data: &[SovPoint]) -> String {
    let days: BTreeSet<&str> = data.iter().map(|p| p.day.as_str()).collect();
    let days: Vec<&str> = days.into_iter().collect();
    let markets = ["FR", "DE", "ES"];
    let colors = ["#60a5fa", "#f97316", "#a78bfa"];

    let mut series = Vec::new();
    for (i, &mkt) in markets.iter().enumerate() {
        let values: Vec<f64> = days
            .iter()
            .map(|&d| {
                data.iter()
                    .find(|p| p.day == d && p.marketplace == mkt)
                    .map(|p| (p.avg_sov * 10.0).round() / 10.0)
                    .unwrap_or(0.0)
            })
            .collect();
        series.push(serde_json::json!({
            "name": mkt,
            "type": "line",
            "smooth": true,
            "data": values,
            "itemStyle": { "color": colors[i] },
        }));
    }

    serde_json::json!({
        "tooltip": { "trigger": "axis" },
        "legend": { "data": markets, "textStyle": { "color": "#9ca3af" } },
        "grid": { "left": "3%", "right": "4%", "bottom": "3%", "containLabel": true },
        "xAxis": { "type": "category", "data": days, "axisLabel": { "color": "#9ca3af" } },
        "yAxis": { "type": "value", "name": "SOV %", "nameTextStyle": { "color": "#9ca3af" }, "axisLabel": { "color": "#9ca3af" } },
        "series": series,
    })
    .to_string()
}
