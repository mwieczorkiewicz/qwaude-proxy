//! Pure, I/O-free message-role-coercion logic.
//!
//! vLLM's chat template rejects any `role: "system"` message that is not at
//! index 0 of the `messages` array (`raise_exception("System message must be
//! at the beginning")`). This module rewrites every *other* system-role
//! message to `role: "user"`, prepending a configurable notice prefix to its
//! content, before the request reaches the upstream model server.
//!
//! ## Zero-copy / targeted-mutation design note
//!
//! The approved design called for runtime SIMD-feature-detection dispatch
//! between `simd-json` and `serde_json`. That dual-dispatch was evaluated and
//! deliberately simplified to a `serde_json::Value`-only implementation (an
//! explicitly pre-approved fallback — see `code-generation-plan.md` Step 4 and
//! `code-summary.md`), because `simd_json::BorrowedValue`/`OwnedValue` and
//! `serde_json::Value` are structurally incompatible types: a genuine dual
//! path would require either duplicating this entire mutation algorithm once
//! per value-tree type, or introducing a trait abstraction over both — real
//! added complexity for a proxy whose latency is dominated by the upstream
//! LLM call, not JSON parsing. `simd-json` was dropped from `Cargo.toml`
//! accordingly.
//!
//! What *is* implemented is targeted mutation within `serde_json`'s ownership
//! model: the request body is parsed into an owned `Value` tree exactly once
//! (unavoidable — `serde_json::Value` has no borrowed/zero-copy mode), and
//! from then on only the `role` and `content` fields of messages that
//! actually need coercion are mutated in place. No message is cloned; no
//! message other than the ones being rewritten is touched at all. `content`
//! prefixing builds exactly one new `String` per rewritten message (sized
//! with `String::with_capacity` to avoid reallocation growth), rather than
//! allocating an intermediate prefix string and concatenating.
//!
//! One further caveat on "byte-identical" round-tripping: re-serializing a
//! `serde_json::Value` produces a canonical minimal JSON rendering. Untouched
//! *field values* round-trip exactly (this crate enables serde_json's
//! `preserve_order` feature so object key order is preserved too), but
//! insignificant input formatting that JSON does not preserve semantically —
//! extra whitespace, non-canonical number formatting (e.g. `1.50` vs `1.5`) —
//! is not reproduced. This is the unavoidable cost of the one full parse;
//! see `README.md`'s Performance section and `benches/transform_bench.rs`
//! for the measured allocation/latency cost against a naive baseline.

use serde_json::Value;

/// Role value that must never appear at any message index other than 0.
const SYSTEM_ROLE: &str = "system";
/// Role a coerced system message is rewritten to.
const USER_ROLE: &str = "user";

/// Report of what a transform call changed, for logging. Carries only
/// indices and a count — never message content — so callers can log at
/// `debug` without leaking payload data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CoercionReport {
    pub coerced_indices: Vec<usize>,
}

impl CoercionReport {
    pub fn count(&self) -> usize {
        self.coerced_indices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.coerced_indices.is_empty()
    }
}

/// Errors from the byte-level entry point (`coerce_system_messages`).
/// Deliberately free of HTTP/transport concerns — mapping to a status code
/// and JSON error body is `error.rs`'s `ProxyError`'s job, not this module's.
#[derive(Debug, thiserror::Error)]
pub enum TransformError {
    #[error("request body is not valid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("request body is not a JSON object")]
    NotAnObject,
    #[error("`messages` field is missing or is not an array")]
    MissingMessagesArray,
}

/// Core, pure coercion logic: given a mutable slice of message objects
/// (as they'd appear in a chat-completion request's `messages` array),
/// rewrite every message at index > 0 whose `role` is `"system"` to
/// `role: "user"` with `notice_prefix` prepended to its text content.
/// `messages[0]` is never touched, regardless of its role.
///
/// No I/O, no panics on malformed/non-standard message shapes: a message
/// missing a `content` field, or with a `content` shape that is neither a
/// string nor a content-block array, still has its `role` coerced but its
/// content is left as-is.
pub fn coerce_messages(messages: &mut [Value], notice_prefix: &str) -> CoercionReport {
    let mut report = CoercionReport::default();
    for (idx, message) in messages.iter_mut().enumerate() {
        if idx == 0 {
            continue;
        }
        if !is_system_role(message) {
            continue;
        }
        coerce_one_message(message, notice_prefix);
        report.coerced_indices.push(idx);
    }
    report
}

/// Byte-level entry point used by the HTTP handler: parses `body` as a JSON
/// chat-completion request object, coerces its `messages` array in place,
/// and reserializes. Returns the rewritten body bytes plus a report of what
/// was coerced.
pub fn coerce_system_messages(
    body: &[u8],
    notice_prefix: &str,
) -> Result<(Vec<u8>, CoercionReport), TransformError> {
    let mut value: Value = serde_json::from_slice(body)?;
    let obj = value.as_object_mut().ok_or(TransformError::NotAnObject)?;
    let messages = obj
        .get_mut("messages")
        .and_then(Value::as_array_mut)
        .ok_or(TransformError::MissingMessagesArray)?;
    let report = coerce_messages(messages, notice_prefix);
    let out = serde_json::to_vec(&value)?;
    Ok((out, report))
}

/// Does this message object have `role: "system"`? Any non-string or
/// missing `role` field is treated as "not system" rather than panicking.
fn is_system_role(message: &Value) -> bool {
    message.get("role").and_then(Value::as_str) == Some(SYSTEM_ROLE)
}

/// Rewrite one message in place: `role` becomes `"user"`, and `content` (if
/// present and in a recognized shape) gets `notice_prefix` prepended.
fn coerce_one_message(message: &mut Value, notice_prefix: &str) {
    let Some(obj) = message.as_object_mut() else {
        // Not an object at all (malformed input) — nothing sensible to coerce.
        return;
    };
    obj.insert("role".to_string(), Value::String(USER_ROLE.to_string()));
    if let Some(content) = obj.get_mut("content") {
        prefix_content(content, notice_prefix);
    }
}

/// Prepend `notice_prefix` to a message's text content, in place.
///
/// - Plain string content: prefix directly, one allocation sized up front.
/// - Content-block array (`[{"type": "text", "text": "..."}]`): prefix only
///   the first block whose `"type"` is `"text"` — every other block (images,
///   later text blocks, cache-control hints, etc.) is left untouched.
/// - Anything else (`null`, a number, an object, a string-less block) is
///   left as-is: there is no text to prefix, and this must never panic.
fn prefix_content(content: &mut Value, notice_prefix: &str) {
    match content {
        Value::String(s) => prepend_in_place(s, notice_prefix),
        Value::Array(blocks) => {
            if let Some(Value::String(s)) = blocks
                .iter_mut()
                .find(|b| b.get("type").and_then(Value::as_str) == Some("text"))
                .and_then(|b| b.get_mut("text"))
            {
                prepend_in_place(s, notice_prefix);
            }
        }
        _ => {}
    }
}

/// Prepend `prefix` to `s` in place with a single reallocation-sized
/// allocation, avoiding an intermediate `format!`/`+`-concatenation string.
fn prepend_in_place(s: &mut String, prefix: &str) {
    let mut rewritten = String::with_capacity(prefix.len() + s.len());
    rewritten.push_str(prefix);
    rewritten.push_str(s);
    *s = rewritten;
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use serde_json::json;
    use std::collections::HashSet;

    fn text_block(text: &str) -> Value {
        json!({"type": "text", "text": text})
    }

    // ---------------------------------------------------------------
    // Unit tests (Step 3 list)
    // ---------------------------------------------------------------

    #[test]
    fn leading_system_message_is_left_alone() {
        let mut messages = vec![
            json!({"role": "system", "content": "You are a helpful assistant."}),
            json!({"role": "user", "content": "hi"}),
        ];
        let before = messages.clone();
        let report = coerce_messages(&mut messages, "[System Notification] ");
        assert!(report.is_empty());
        assert_eq!(messages, before);
    }

    #[test]
    fn single_mid_stream_system_message_is_coerced_and_prefixed() {
        let mut messages = vec![
            json!({"role": "user", "content": "hi"}),
            json!({"role": "system", "content": "reminder"}),
            json!({"role": "assistant", "content": "ok"}),
        ];
        let report = coerce_messages(&mut messages, "[NOTICE] ");
        assert_eq!(report.coerced_indices, vec![1]);
        assert_eq!(messages[1]["role"], json!("user"));
        assert_eq!(messages[1]["content"], json!("[NOTICE] reminder"));
        assert_eq!(messages[0], json!({"role": "user", "content": "hi"}));
        assert_eq!(messages[2], json!({"role": "assistant", "content": "ok"}));
    }

    #[test]
    fn multiple_mid_stream_system_messages_are_all_coerced() {
        let mut messages = vec![
            json!({"role": "system", "content": "leading, untouched"}),
            json!({"role": "system", "content": "first reminder"}),
            json!({"role": "user", "content": "question"}),
            json!({"role": "system", "content": "second reminder"}),
        ];
        let report = coerce_messages(&mut messages, "[N] ");
        assert_eq!(report.coerced_indices, vec![1, 3]);
        assert_eq!(messages[0]["role"], json!("system"));
        assert_eq!(messages[1]["role"], json!("user"));
        assert_eq!(messages[1]["content"], json!("[N] first reminder"));
        assert_eq!(messages[3]["role"], json!("user"));
        assert_eq!(messages[3]["content"], json!("[N] second reminder"));
    }

    #[test]
    fn string_content_is_prefixed_directly() {
        let mut messages = vec![
            json!({"role": "user", "content": "hi"}),
            json!({"role": "system", "content": "plain text"}),
        ];
        coerce_messages(&mut messages, ">> ");
        assert_eq!(messages[1]["content"], json!(">> plain text"));
    }

    #[test]
    fn content_block_array_prefixes_only_first_text_block() {
        let mut messages = vec![
            json!({"role": "user", "content": "hi"}),
            json!({
                "role": "system",
                "content": [text_block("first"), text_block("second")],
            }),
        ];
        coerce_messages(&mut messages, ">> ");
        assert_eq!(messages[1]["content"][0]["text"], json!(">> first"));
        assert_eq!(messages[1]["content"][1]["text"], json!("second"));
    }

    #[test]
    fn message_with_missing_or_non_standard_content_does_not_panic() {
        let mut messages = vec![
            json!({"role": "user", "content": "hi"}),
            json!({"role": "system"}),
            json!({"role": "system", "content": null}),
            json!({"role": "system", "content": 42}),
        ];
        let report = coerce_messages(&mut messages, ">> ");
        assert_eq!(report.coerced_indices, vec![1, 2, 3]);
        for idx in [1, 2, 3] {
            assert_eq!(messages[idx]["role"], json!("user"));
        }
    }

    #[test]
    fn unknown_and_extra_fields_round_trip_unchanged() {
        let mut messages = vec![
            json!({"role": "user", "content": "hi"}),
            json!({
                "role": "system",
                "content": "reminder",
                "name": "sys-reminder",
                "tool_call_id": "call_123",
                "tool_calls": [{"id": "call_1", "type": "function"}],
                "cache_control": {"type": "ephemeral"},
                "x_unknown_future_field": {"nested": [1, 2, 3]},
            }),
        ];
        coerce_messages(&mut messages, ">> ");
        let m = &messages[1];
        assert_eq!(m["name"], json!("sys-reminder"));
        assert_eq!(m["tool_call_id"], json!("call_123"));
        assert_eq!(
            m["tool_calls"],
            json!([{"id": "call_1", "type": "function"}])
        );
        assert_eq!(m["cache_control"], json!({"type": "ephemeral"}));
        assert_eq!(m["x_unknown_future_field"], json!({"nested": [1, 2, 3]}));
    }

    #[test]
    fn byte_level_entry_point_round_trips_the_full_request() {
        let body = br#"{"model":"qwen","stream":false,"messages":[{"role":"user","content":"hi"},{"role":"system","content":"reminder"}]}"#;
        let (out, report) = coerce_system_messages(body, "[N] ").expect("valid input parses");
        assert_eq!(report.coerced_indices, vec![1]);
        let value: Value = serde_json::from_slice(&out).expect("output is valid json");
        assert_eq!(value["model"], json!("qwen"));
        assert_eq!(value["messages"][1]["role"], json!("user"));
        assert_eq!(value["messages"][1]["content"], json!("[N] reminder"));
    }

    #[test]
    fn byte_level_entry_point_rejects_non_object_body() {
        let err = coerce_system_messages(b"[1,2,3]", ">> ").unwrap_err();
        assert!(matches!(err, TransformError::NotAnObject));
    }

    #[test]
    fn byte_level_entry_point_rejects_missing_messages_array() {
        let err = coerce_system_messages(br#"{"model":"qwen"}"#, ">> ").unwrap_err();
        assert!(matches!(err, TransformError::MissingMessagesArray));
    }

    #[test]
    fn byte_level_entry_point_rejects_invalid_json() {
        let err = coerce_system_messages(b"not json", ">> ").unwrap_err();
        assert!(matches!(err, TransformError::InvalidJson(_)));
    }

    // ---------------------------------------------------------------
    // Property-based tests (proptest) — the three named invariants
    // ---------------------------------------------------------------

    fn arb_content() -> impl Strategy<Value = Value> {
        prop_oneof![
            "[a-zA-Z0-9 .,!?]{0,40}".prop_map(Value::String),
            prop::collection::vec(
                "[a-zA-Z0-9 .,!?]{0,20}".prop_map(|t| json!({"type": "text", "text": t})),
                1..4
            )
            .prop_map(Value::Array),
            Just(Value::Null),
        ]
    }

    fn arb_role() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("system".to_string()),
            Just("user".to_string()),
            Just("assistant".to_string()),
            Just("tool".to_string()),
        ]
    }

    fn arb_message() -> impl Strategy<Value = Value> {
        (arb_role(), arb_content()).prop_map(
            |(role, content)| json!({"role": role, "content": content, "name": "fixed-name"}),
        )
    }

    fn arb_messages() -> impl Strategy<Value = Vec<Value>> {
        prop::collection::vec(arb_message(), 1..8)
    }

    proptest! {
        /// Invariant (a): the output has the same length as the input.
        #[test]
        fn prop_output_length_matches_input(mut messages in arb_messages()) {
            let original_len = messages.len();
            coerce_messages(&mut messages, "[N] ");
            prop_assert_eq!(messages.len(), original_len);
        }

        /// Invariant (b): no `role: "system"` remains at any index other than 0.
        #[test]
        fn prop_no_non_leading_system_role_remains(mut messages in arb_messages()) {
            coerce_messages(&mut messages, "[N] ");
            for m in messages.iter().skip(1) {
                prop_assert_ne!(m.get("role").and_then(Value::as_str), Some(SYSTEM_ROLE));
            }
        }

        /// Invariant (c): messages that were not coerced (index 0, and any
        /// message whose role was not `"system"`) are preserved exactly —
        /// same content, same extra fields.
        #[test]
        fn prop_non_coerced_messages_are_untouched(mut messages in arb_messages()) {
            let before = messages.clone();
            let report = coerce_messages(&mut messages, "[N] ");
            let coerced: HashSet<usize> = report.coerced_indices.iter().copied().collect();
            for (idx, m) in messages.iter().enumerate() {
                if !coerced.contains(&idx) {
                    prop_assert_eq!(m, &before[idx]);
                }
            }
        }
    }

    #[test]
    fn constants_are_the_expected_role_strings() {
        assert_eq!(SYSTEM_ROLE, "system");
        assert_eq!(USER_ROLE, "user");
    }
}
