use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RequestState {
    pub method: String,
    pub url: String,
    pub query_params: Vec<KeyValue>,
    pub headers: Vec<KeyValue>,
    pub body: String,
}

#[derive(Serialize, Clone)]
pub struct SendRequestArgs {
    pub args: RequestState,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompletedRequest {
    pub req: RequestState,
    pub resp: ResponseState,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompletedRequestArgs {
    pub args: CompletedRequest,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ResponseState {
    pub status: u16,
    pub body: String,
    pub response_time: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HighlightArgs<'a> {
    pub code: &'a str,
    pub lang: &'a str,
}
