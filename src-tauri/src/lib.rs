// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use serde::{Deserialize, Serialize};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
    TRACE,
    CONNECT,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct KeyValue {
    key: String,
    value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RequestArgs {
    method: HttpMethod,
    url: String,
    query_params: Vec<KeyValue>,
    headers: Vec<KeyValue>,
    body: String,
}

#[derive(Clone, Debug, Serialize)]
struct ResponseData {
    status: u16,
    body: String,
    response_time: f32
}

#[tauri::command]
async fn send_request(args: RequestArgs) -> Result<ResponseData, String> {
    let client = reqwest::Client::new();
    
    let mut headers = HeaderMap::new();
    for kv in args.headers {
        if kv.key.is_empty() { continue; }
        headers.insert(
            HeaderName::from_bytes(kv.key.as_bytes()).map_err(|e| e.to_string())?,
            HeaderValue::from_str(&kv.value).map_err(|e| e.to_string())?
        );
    }

    let start = std::time::Instant::now();

    let res = match args.method {
        HttpMethod::GET => client.get(&args.url).headers(headers).send().await,
        HttpMethod::POST => client.post(&args.url).headers(headers).body(args.body).send().await,
        HttpMethod::PUT => client.put(&args.url).headers(headers).body(args.body).send().await,
        HttpMethod::DELETE => client.delete(&args.url).headers(headers).send().await,
        _ => return Err("Unsupported method".into()),
    };

    let res = res.map_err(|e| e.to_string())?;
    let status = res.status().as_u16();
    let body = res.text().await.map_err(|e| e.to_string())?;
    let elapsed = start.elapsed().as_secs_f32() * 1000.0;

    Ok(ResponseData {
        status,
        response_time: elapsed,
        body,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![send_request])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
