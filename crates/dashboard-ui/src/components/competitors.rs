use crate::models::CompetitorRow;
use leptos::prelude::*;

#[component]
pub fn CompetitorsTable(data: Vec<CompetitorRow>) -> impl IntoView {
    view! {
        <div class="bg-gray-800 rounded-lg p-4 border border-gray-700">
            <h2 class="text-lg font-semibold mb-3">"Top Competitors (30 days)"</h2>
            <div class="overflow-y-auto max-h-80">
                <table class="w-full text-sm text-left">
                    <thead class="text-xs text-gray-400 uppercase border-b border-gray-700">
                        <tr>
                            <th class="px-3 py-2">"Mkt"</th>
                            <th class="px-3 py-2">"Keyword"</th>
                            <th class="px-3 py-2">"Brand"</th>
                            <th class="px-3 py-2 text-right">"Seen"</th>
                            <th class="px-3 py-2 text-right">"Avg Pos"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {data
                            .into_iter()
                            .map(|row| {
                                view! {
                                    <tr class="border-b border-gray-700/50 hover:bg-gray-700/30">
                                        <td class="px-3 py-2 font-medium">
                                            {row.marketplace.clone()}
                                        </td>
                                        <td class="px-3 py-2 text-gray-300">
                                            {row.keyword.clone()}
                                        </td>
                                        <td class="px-3 py-2 text-gray-300">{row.brand.clone()}</td>
                                        <td class="px-3 py-2 text-right">{row.times_seen}</td>
                                        <td class="px-3 py-2 text-right text-gray-300">
                                            {format!("{:.1}", row.avg_position)}
                                        </td>
                                    </tr>
                                }
                            })
                            .collect::<Vec<_>>()}
                    </tbody>
                </table>
            </div>
        </div>
    }
}
