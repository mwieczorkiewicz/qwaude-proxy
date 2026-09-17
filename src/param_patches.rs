//! Optional rewriting of top-level chat-completion JSON fields before upstream vLLM.

use serde_json::{Map, Value};

/// How patch keys are merged into the client request body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchMode {
    /// Set a field only when the client omitted it or sent JSON `null`.
    IfAbsent,
    /// Always overwrite (or insert) patch keys — mirrors forced server defaults.
    Always,
}

/// Resolved patch set from env (`REQUEST_PARAM_PATCHES_*`).
#[derive(Debug, Clone, PartialEq)]
pub struct RequestParamPatches {
    pub mode: PatchMode,
    pub fields: Map<String, Value>,
}

impl RequestParamPatches {
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }
}

/// Demo Room `thinking-mode-sampling` vLLM defaults (see demo-room-infra GitOps).
pub fn thinking_mode_sampling_preset() -> Map<String, Value> {
    Map::from_iter([
        ("temperature".to_string(), Value::from(0.6)),
        ("top_p".to_string(), Value::from(0.95)),
        ("top_k".to_string(), Value::from(20)),
        ("min_p".to_string(), Value::from(0.0)),
        ("presence_penalty".to_string(), Value::from(0.0)),
        ("repetition_penalty".to_string(), Value::from(1.0)),
    ])
}

/// Apply configured patches to a chat-completion request object. Returns the
/// number of fields written.
pub fn apply_request_param_patches(
    obj: &mut Map<String, Value>,
    patches: &RequestParamPatches,
) -> u32 {
    let mut written = 0u32;
    for (key, patch_value) in &patches.fields {
        let should_write = match patches.mode {
            PatchMode::Always => true,
            PatchMode::IfAbsent => !obj.contains_key(key) || obj.get(key) == Some(&Value::Null),
        };
        if should_write {
            obj.insert(key.clone(), patch_value.clone());
            written += 1;
        }
    }
    written
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn always_mode_overwrites_client_temperature() {
        let patches = RequestParamPatches {
            mode: PatchMode::Always,
            fields: thinking_mode_sampling_preset(),
        };
        let mut obj = Map::from_iter([("temperature".to_string(), json!(0.1))]);
        let n = apply_request_param_patches(&mut obj, &patches);
        assert!(n >= 1);
        assert_eq!(obj["temperature"], json!(0.6));
    }

    #[test]
    fn if_absent_leaves_client_temperature() {
        let patches = RequestParamPatches {
            mode: PatchMode::IfAbsent,
            fields: thinking_mode_sampling_preset(),
        };
        let mut obj = Map::from_iter([("temperature".to_string(), json!(0.1))]);
        apply_request_param_patches(&mut obj, &patches);
        assert_eq!(obj["temperature"], json!(0.1));
    }

    #[test]
    fn if_absent_fills_missing_top_p() {
        let patches = RequestParamPatches {
            mode: PatchMode::IfAbsent,
            fields: thinking_mode_sampling_preset(),
        };
        let mut obj = Map::new();
        apply_request_param_patches(&mut obj, &patches);
        assert_eq!(obj["top_p"], json!(0.95));
    }
}
