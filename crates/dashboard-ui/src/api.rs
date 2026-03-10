use crate::models::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

async fn tauri_invoke(cmd: &str) -> Result<String, String> {
    let result = invoke(cmd, JsValue::NULL)
        .await
        .map_err(|e| format!("{:?}", e))?;
    result
        .as_string()
        .ok_or_else(|| "invoke returned non-string".to_string())
}

pub async fn get_snapshots() -> Result<Vec<MarketSnapshot>, String> {
    let json = tauri_invoke("get_snapshots").await?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

pub async fn get_sov_trend() -> Result<Vec<SovPoint>, String> {
    let json = tauri_invoke("get_sov_trend").await?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

pub async fn get_placement_mix() -> Result<Vec<PlacementPoint>, String> {
    let json = tauri_invoke("get_placement_mix").await?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

pub async fn get_top_competitors() -> Result<Vec<CompetitorRow>, String> {
    let json = tauri_invoke("get_top_competitors").await?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

pub async fn get_fr_gap() -> Result<Vec<FrGapPoint>, String> {
    let json = tauri_invoke("get_fr_gap").await?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
