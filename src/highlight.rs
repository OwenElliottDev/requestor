use serde_wasm_bindgen::{from_value, to_value};

use crate::app::invoke;
use crate::models::HighlightArgs;

pub async fn highlight_to_html(code: &str, lang: &str) -> Result<String, String> {
    let args = HighlightArgs { code, lang };

    let js_val = invoke("highlight_code", to_value(&args).unwrap())
        .await
        .map_err(|e| format!("invoke failed: {:?}", e))?;

    from_value(js_val).map_err(|e| format!("deserialize failed: {:?}", e))
}
