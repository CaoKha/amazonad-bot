use js_sys::Function;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

/// Call window.mts_update_chart(id, optionJson) defined in index.html.
pub fn update_chart(chart_id: &str, option_json: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(func_val) = js_sys::Reflect::get(&window, &JsValue::from_str("mts_update_chart")) else {
        return;
    };
    let Some(func) = func_val.dyn_ref::<Function>() else {
        return;
    };
    let _ = func.call2(
        &JsValue::NULL,
        &JsValue::from_str(chart_id),
        &JsValue::from_str(option_json),
    );
}
