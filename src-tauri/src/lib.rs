use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use rusqlite::{
    params,
    types::{FromSql, FromSqlError, ValueRef},
    Connection, Result,
};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use syntect::easy::HighlightLines;
use syntect::html::{
    start_highlighted_html_snippet, styled_line_to_highlighted_html, IncludeBackground,
};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use vscode_theme_syntect::parse_vscode_theme;
// use tauri::Manager;

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(|| SyntaxSet::load_defaults_newlines());
static THEME: Lazy<syntect::highlighting::Theme> = Lazy::new(|| {
    let vscode = parse_vscode_theme(include_str!("../highlight_themes/dark_plus.json"))
        .expect("Failed to parse VS Code theme");

    syntect::highlighting::Theme::try_from(vscode).expect("Failed to convert to syntect Theme")
});

fn rfc3339_now() -> String {
    let now = SystemTime::now();
    let now: DateTime<Utc> = now.into();
    now.to_rfc3339()
}

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

impl HttpMethod {
    pub fn as_str(&self) -> &str {
        match self {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
            HttpMethod::PUT => "PUT",
            HttpMethod::DELETE => "DELETE",
            HttpMethod::PATCH => "PATCH",
            HttpMethod::HEAD => "HEAD",
            HttpMethod::OPTIONS => "OPTIONS",
            HttpMethod::TRACE => "TRACE",
            HttpMethod::CONNECT => "CONNECT",
        }
    }
}

impl FromSql for HttpMethod {
    fn column_result(value: ValueRef<'_>) -> Result<Self, FromSqlError> {
        match value.as_str()? {
            "GET" => Ok(HttpMethod::GET),
            "POST" => Ok(HttpMethod::POST),
            "PUT" => Ok(HttpMethod::PUT),
            "DELETE" => Ok(HttpMethod::DELETE),
            "PATCH" => Ok(HttpMethod::PATCH),
            "HEAD" => Ok(HttpMethod::HEAD),
            "OPTIONS" => Ok(HttpMethod::OPTIONS),
            "TRACE" => Ok(HttpMethod::TRACE),
            "CONNECT" => Ok(HttpMethod::CONNECT),
            _ => Err(FromSqlError::InvalidType),
        }
    }
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

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ResponseData {
    status: u16,
    body: String,
    response_time: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CompletedRequestArgs {
    req: RequestArgs,
    resp: ResponseData,
}

#[tauri::command]
async fn send_request(args: RequestArgs) -> Result<ResponseData, String> {
    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    for kv in args.headers {
        if kv.key.is_empty() {
            continue;
        }
        headers.insert(
            HeaderName::from_bytes(kv.key.as_bytes()).map_err(|e| e.to_string())?,
            HeaderValue::from_str(&kv.value).map_err(|e| e.to_string())?,
        );
    }

    let start = std::time::Instant::now();

    let res = match args.method {
        HttpMethod::GET => client.get(&args.url).headers(headers).send().await,
        HttpMethod::POST => {
            client
                .post(&args.url)
                .headers(headers)
                .body(args.body)
                .send()
                .await
        }
        HttpMethod::PUT => {
            client
                .put(&args.url)
                .headers(headers)
                .body(args.body)
                .send()
                .await
        }
        HttpMethod::DELETE => client.delete(&args.url).headers(headers).send().await,
        HttpMethod::PATCH => {
            client
                .patch(&args.url)
                .headers(headers)
                .body(args.body)
                .send()
                .await
        }
        HttpMethod::HEAD => client.head(&args.url).headers(headers).send().await,
        HttpMethod::OPTIONS => {
            client
                .request(reqwest::Method::OPTIONS, &args.url)
                .headers(headers)
                .send()
                .await
        }
        HttpMethod::TRACE => {
            client
                .request(reqwest::Method::TRACE, &args.url)
                .headers(headers)
                .send()
                .await
        }
        HttpMethod::CONNECT => {
            client
                .request(reqwest::Method::CONNECT, &args.url)
                .headers(headers)
                .send()
                .await
        }
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

#[tauri::command]
fn save_request(args: CompletedRequestArgs) -> Result<(), String> {
    let conn = Connection::open("requests.db").map_err(|e| e.to_string())?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS requests (
            id INTEGER PRIMARY KEY,
            method TEXT,
            url TEXT,
            query_params TEXT,
            headers TEXT,
            body TEXT,
            status INTEGER,
            response_body TEXT,
            response_time REAL,
            created_at TEXT
        )",
        [],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO requests (method, url, query_params, headers, body, status, response_body, response_time, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            args.req.method.as_str(),
            args.req.url,
            serde_json::to_string(&args.req.query_params).map_err(|e| e.to_string())?,
            serde_json::to_string(&args.req.headers).map_err(|e| e.to_string())?,
            args.req.body,
            args.resp.status,
            args.resp.body,
            args.resp.response_time,
            rfc3339_now()
        ],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_requests() -> Result<Vec<CompletedRequestArgs>, String> {
    log::debug!("Getting requests!");
    let conn = Connection::open("requests.db").map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT method, url, query_params, headers, body, status, response_body, response_time FROM requests")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(CompletedRequestArgs {
                req: RequestArgs {
                    method: row.get(0)?,
                    url: row.get(1)?,
                    query_params: serde_json::from_str(&row.get::<_, String>(2)?)
                        .unwrap_or_default(),
                    headers: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or_default(),
                    body: row.get(4)?,
                },
                resp: ResponseData {
                    status: row.get(5)?,
                    body: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                    response_time: row.get(7)?,
                },
            })
        })
        .map_err(|e| e.to_string())?;

    let mut requests = Vec::new();
    for r in rows {
        requests.push(r.map_err(|e| e.to_string())?);
    }
    Ok(requests)
}

#[tauri::command]
fn highlight_code(code: String, lang: String) -> Result<String, String> {
    let theme = &THEME;
    let syntax = SYNTAX_SET.find_syntax_by_extension(&lang).unwrap();
    let mut h = HighlightLines::new(syntax, &theme);
    let (mut html, _bg) = start_highlighted_html_snippet(&theme);

    for line in LinesWithEndings::from(&code) {
        let regions = h
            .highlight_line(line, &SYNTAX_SET)
            .map_err(|e| e.to_string())?;
        let line_html = styled_line_to_highlighted_html(&regions[..], IncludeBackground::No)
            .map_err(|e| e.to_string())?;
        html.push_str(&line_html);
    }
    html.push_str("</pre>");
    Ok(html)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // .setup(|app| {
        //     #[cfg(debug_assertions)]
        //     {
        //         let window = app.get_webview_window("main").unwrap();
        //         window.open_devtools();
        //     }
        //     Ok(())
        // })
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(tauri_plugin_log::log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            send_request,
            save_request,
            get_requests,
            highlight_code
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
