#![allow(non_snake_case)]

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

static CSS: Asset = asset!("/assets/styles.css");

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct KeyValue {
    key: String,
    value: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct RequestState {
    method: String,
    url: String,
    query_params: Vec<KeyValue>,
    headers: Vec<KeyValue>,
    body: String,
}

#[derive(Serialize, Clone)]
struct SendRequestArgs {
    args: RequestState,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CompletedRequest {
    req: RequestState,
    resp: ResponseState,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CompletedRequestArgs {
    args: CompletedRequest,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct ResponseState {
    status: u16,
    body: String,
    response_time: f64,
}

fn key_value_editor(
    mut request: Signal<RequestState>,
    field: fn(&mut RequestState) -> &mut Vec<KeyValue>,
) -> Element {
    let items = {
        let snapshot = request.read();
        field(&mut snapshot.clone()).clone()
    };

    rsx! {
        div { class: "kv-editor",
            for (idx , kv) in items.iter().enumerate() {
                div { class: "kv-row",
                    input {
                        placeholder: "Key",
                        value: "{kv.key}",
                        oninput: move |e| {
                            request
                                .with_mut(|r| {
                                    field(r)[idx].key = e.value();
                                });
                        },
                    }

                    input {
                        placeholder: "Value",
                        value: "{kv.value}",
                        oninput: move |e| {
                            request
                                .with_mut(|r| {
                                    field(r)[idx].value = e.value();
                                });
                        },
                    }

                    button {
                        onclick: move |_| {
                            request
                                .with_mut(|r| {
                                    field(r).remove(idx);
                                });
                        },
                        "✕"
                    }
                }
            }

            button {
                onclick: move |_| {
                    request
                        .with_mut(|r| {
                            field(r).push(KeyValue::default());
                        });
                },
                "+ Add"
            }
        }
    }
}

pub fn App() -> Element {
    let mut request = use_signal(RequestState::default);
    let response = use_signal(|| None::<ResponseState>);
    let mut request_history = use_signal(Vec::<CompletedRequest>::new);

    let send_request = {
        let request_signal = request.clone();
        let response_signal = response.clone();

        move |_| {
            let req_owned = SendRequestArgs {
                args: request_signal.read().clone(),
            };

            let js_args = match to_value(&req_owned) {
                Ok(v) => v,
                Err(err) => {
                    web_sys::console::error_1(&format!("serialize err: {err:?}").into());
                    return;
                }
            };

            let mut response_signal = response_signal.clone();

            wasm_bindgen_futures::spawn_local(async move {
                let result: Result<ResponseState, String> = invoke("send_request", js_args)
                    .await
                    .map_err(|e| format!("invoke failed: {e:?}"))
                    .and_then(|js_val| {
                        from_value::<ResponseState>(js_val)
                            .map_err(|e| format!("deserialize err: {e:?}"))
                    });

                match result {
                    Ok(resp) => {
                        response_signal.set(Some(resp.clone()));
                        let completed_request_data = CompletedRequest {
                            req: request_signal.read().clone(),
                            resp: resp,
                        };
                        let completed_request = CompletedRequestArgs {
                            args: completed_request_data.clone(),
                        };

                        request_history.with_mut(|hist| hist.push(completed_request_data));

                        let completed_request_js = match to_value(&completed_request) {
                            Ok(v) => v,
                            Err(err) => {
                                web_sys::console::error_1(
                                    &format!("serialize err: {err:?}").into(),
                                );
                                return;
                            }
                        };

                        let save_result: Result<JsValue, String> =
                            invoke("save_request", completed_request_js)
                                .await
                                .map_err(|e| format!("save_request invoke failed: {e:?}"));

                        if let Err(e) = save_result {
                            web_sys::console::error_1(&e.into());
                        }
                    }
                    Err(msg) => {
                        web_sys::console::error_1(&msg.clone().into());

                        response_signal.set(Some(ResponseState {
                            status: 0,
                            response_time: 0.0,
                            body: msg,
                        }));
                    }
                }
            });
        }
    };

    use_effect(move || {
        let mut history_signal = request_history.clone();

        wasm_bindgen_futures::spawn_local(async move {
            let result: Result<Vec<CompletedRequest>, String> =
                invoke("get_requests", JsValue::NULL)
                    .await
                    .map_err(|e| format!("invoke failed: {:?}", e))
                    .and_then(|js_val| {
                        from_value::<Vec<CompletedRequest>>(js_val)
                            .map_err(|e| format!("deserialize failed: {:?}", e))
                    });

            if let Ok(history) = result {
                history_signal.set(history);
            } else if let Err(err) = result {
                web_sys::console::error_1(&err.into());
            }
        });
    });

    rsx! {
        link { rel: "stylesheet", href: CSS }

        main { class: "container",
            h1 { "Requestor" }

            div { class: "request-line",
                select {
                    value: "{request.read().method}",
                    onchange: move |e| request.with_mut(|r| r.method = e.value()),
                    option { value: "", disabled: true, "Method" }
                    option { "GET" }
                    option { "POST" }
                    option { "PUT" }
                    option { "DELETE" }
                    option { "PATCH" }
                }

                input {
                    placeholder: "https://api.example.com",
                    value: "{request.read().url}",
                    oninput: move |e| request.with_mut(|r| r.url = e.value()),
                }

                button { onclick: send_request, "Send" }
            }

            section {
                h3 { "Headers" }
                {key_value_editor(request, |r| &mut r.headers)}
            }

            section {
                h3 { "Query Params" }
                {key_value_editor(request, |r| &mut r.query_params)}
            }

            section {
                h3 { "Body" }
                textarea {
                    placeholder: "Raw request body...",
                    value: "{request.read().body}",
                    oninput: move |e| request.with_mut(|r| r.body = e.value()),
                }
            }

            section { class: "response",
                {
                    if let Some(resp) = response.read().as_ref() {
                        rsx! {
                            h3 { "Response" }

                            p { class: if resp.status >= 200 && resp.status < 300 { "status-ok" } else { "status-error" },
                                strong { "Status: " }
                                "{resp.status}"
                            }

                            p {
                                strong { "Processing time (ms): " }
                                "{resp.response_time}"
                            }

                            pre { "{resp.body}" }
                        }
                    } else {
                        rsx! {}
                    }
                }
            }

            {
                let history: Vec<CompletedRequest> = request_history.read().clone();

                rsx! {
                    section {
                        h3 { "History" }
                        div { class: "history-list-container",
                            ul { class: "history-list",
                                for (idx , completed) in history.into_iter().rev().enumerate() {
                                    li {
                                        key: "{idx}",
                                        class: "history-item",
                                        onclick: {
                                            let mut request = request.clone();
                                            move |_| {
                                                request.set(completed.req.clone());
                                            }
                                        },
                                        p {
                                            "{completed.req.method} {completed.req.url} → "
                                            span { class: if completed.resp.status >= 200 && completed.resp.status < 300 { "status-ok" } else { "status-error" },
                                                "{completed.resp.status}"
                                            }
                                            span { class: "response-time",
                                                {format!("({:.2} ms)", completed.resp.response_time)}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
