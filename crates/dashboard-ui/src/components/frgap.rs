use crate::chart;
use crate::models::FrGapPoint;
use gloo_timers::callback::Timeout;
use leptos::prelude::*;

#[component]
pub fn FrGapChart(data: Vec<FrGapPoint>) -> impl IntoView {
    let option_json = build_option(&data);
    let json_clone = option_json.clone();

    Timeout::new(0, move || {
        chart::update_chart("frgap-chart", &json_clone);
    })
    .forget();

    view! {
        <div class="bg-gray-800 rounded-lg p-4 border border-gray-700">
            <h2 class="text-lg font-semibold mb-2">"FR Gap vs DE/ES (30 days)"</h2>
            <div id="frgap-chart" style="width: 100%; height: 320px;"></div>
        </div>
    }
}

fn build_option(data: &[FrGapPoint]) -> String {
    let days: Vec<&str> = data.iter().map(|p| p.day.as_str()).collect();
    let fr: Vec<f64> = data
        .iter()
        .map(|p| (p.fr_sov * 10.0).round() / 10.0)
        .collect();
    let de: Vec<f64> = data
        .iter()
        .map(|p| (p.de_sov * 10.0).round() / 10.0)
        .collect();
    let es: Vec<f64> = data
        .iter()
        .map(|p| (p.es_sov * 10.0).round() / 10.0)
        .collect();

    serde_json::json!({
        "tooltip": { "trigger": "axis" },
        "legend": { "data": ["FR", "DE", "ES"], "textStyle": { "color": "#9ca3af" } },
        "grid": { "left": "3%", "right": "4%", "bottom": "3%", "containLabel": true },
        "xAxis": { "type": "category", "data": days, "axisLabel": { "color": "#9ca3af" } },
        "yAxis": { "type": "value", "name": "SOV %", "nameTextStyle": { "color": "#9ca3af" }, "axisLabel": { "color": "#9ca3af" } },
        "series": [
            { "name": "FR", "type": "line", "smooth": true, "data": fr, "areaStyle": { "opacity": 0.15 }, "itemStyle": { "color": "#60a5fa" } },
            { "name": "DE", "type": "line", "smooth": true, "data": de, "lineStyle": { "type": "dashed" }, "itemStyle": { "color": "#f97316" } },
            { "name": "ES", "type": "line", "smooth": true, "data": es, "lineStyle": { "type": "dashed" }, "itemStyle": { "color": "#a78bfa" } },
        ],
    })
    .to_string()
}
