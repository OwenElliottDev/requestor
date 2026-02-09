use crate::models::{KeyValue, RequestState};
use dioxus::prelude::*;

pub fn key_value_editor(
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
