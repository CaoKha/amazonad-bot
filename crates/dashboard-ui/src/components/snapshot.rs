use crate::models::MarketSnapshot;
use leptos::prelude::*;

#[component]
pub fn SnapshotCards(data: Vec<MarketSnapshot>) -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {data
                .into_iter()
                .map(|snap| {
                    let sov_color = if snap.sov_pct > 50.0 {
                        "text-green-400"
                    } else if snap.sov_pct > 20.0 {
                        "text-yellow-400"
                    } else {
                        "text-red-400"
                    };
                    view! {
                        <div class="bg-gray-800 rounded-lg p-4 border border-gray-700">
                            <div class="flex items-center justify-between mb-2">
                                <span class="text-lg font-semibold">
                                    {snap.marketplace.clone()}
                                </span>
                                <span class="text-xs text-gray-400">
                                    {snap.keyword.clone()}
                                </span>
                            </div>
                            <div class="flex items-baseline gap-2 mb-1">
                                <span class={format!("text-2xl font-bold {sov_color}")}>
                                    {format!("{:.1}%", snap.sov_pct)}
                                </span>
                                <span class="text-gray-400 text-sm">"SOV"</span>
                            </div>
                            <div class="text-xs text-gray-500">
                                {format!(
                                    "{} sponsored / {} total | {} brand matches",
                                    snap.sponsored_count,
                                    snap.total_results,
                                    snap.brand_match_count,
                                )}
                            </div>
                        </div>
                    }
                })
                .collect::<Vec<_>>()}
        </div>
    }
}
