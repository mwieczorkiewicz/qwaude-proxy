//! Env parsing for [`param_patches::RequestParamPatches`].

use crate::config::ConfigError;
use crate::param_patches::{thinking_mode_sampling_preset, PatchMode, RequestParamPatches};
use serde_json::{Map, Value};

pub const ENV_REQUEST_PARAM_PATCHES_ENABLED: &str = "REQUEST_PARAM_PATCHES_ENABLED";
pub const ENV_REQUEST_PARAM_PATCHES_MODE: &str = "REQUEST_PARAM_PATCHES_MODE";
pub const ENV_REQUEST_PARAM_PATCHES_PRESET: &str = "REQUEST_PARAM_PATCHES_PRESET";
pub const ENV_REQUEST_PARAM_PATCHES_JSON: &str = "REQUEST_PARAM_PATCHES_JSON";

pub fn parse_request_param_patches(
    enabled_raw: Option<String>,
    mode_raw: Option<String>,
    preset_raw: Option<String>,
    json_raw: Option<String>,
) -> Result<Option<RequestParamPatches>, ConfigError> {
    let enabled = parse_bool(enabled_raw, ENV_REQUEST_PARAM_PATCHES_ENABLED, false)?;
    if !enabled {
        return Ok(None);
    }

    let mode = parse_patch_mode(mode_raw)?;
    let fields = resolve_patch_fields(preset_raw, json_raw)?;
    if fields.is_empty() {
        return Err(ConfigError::invalid(
            ENV_REQUEST_PARAM_PATCHES_ENABLED,
            "true".to_string(),
            "enabled but no patch fields (set PRESET or JSON)",
        ));
    }

    Ok(Some(RequestParamPatches { mode, fields }))
}

fn parse_bool(raw: Option<String>, var: &'static str, default: bool) -> Result<bool, ConfigError> {
    match raw {
        None => Ok(default),
        Some(value) => match value.as_str() {
            "true" | "1" | "yes" => Ok(true),
            "false" | "0" | "no" => Ok(false),
            _ => Err(ConfigError::invalid(
                var,
                value,
                "must be `true` or `false`",
            )),
        },
    }
}

fn parse_patch_mode(raw: Option<String>) -> Result<PatchMode, ConfigError> {
    match raw {
        None => Ok(PatchMode::IfAbsent),
        Some(value) => match value.as_str() {
            "if_absent" | "missing_only" => Ok(PatchMode::IfAbsent),
            "always" | "force" => Ok(PatchMode::Always),
            _ => Err(ConfigError::invalid(
                ENV_REQUEST_PARAM_PATCHES_MODE,
                value,
                "must be `if_absent` or `always`",
            )),
        },
    }
}

fn resolve_patch_fields(
    preset_raw: Option<String>,
    json_raw: Option<String>,
) -> Result<Map<String, Value>, ConfigError> {
    let mut fields = Map::new();
    if let Some(preset) = preset_raw {
        let preset_fields = match preset.as_str() {
            "thinking-mode" | "thinking-mode-sampling" => thinking_mode_sampling_preset(),
            "" => Map::new(),
            other => {
                return Err(ConfigError::invalid(
                    ENV_REQUEST_PARAM_PATCHES_PRESET,
                    other.to_string(),
                    "unknown preset (supported: thinking-mode)",
                ));
            }
        };
        fields.extend(preset_fields);
    }
    if let Some(json) = json_raw {
        if json.trim().is_empty() {
            return Ok(fields);
        }
        let raw = json;
        let value: Value = serde_json::from_str(&raw).map_err(|_| {
            ConfigError::invalid(
                ENV_REQUEST_PARAM_PATCHES_JSON,
                raw.clone(),
                "must be valid JSON",
            )
        })?;
        let obj = value.as_object().ok_or(ConfigError::invalid(
            ENV_REQUEST_PARAM_PATCHES_JSON,
            raw,
            "must be a JSON object",
        ))?;
        fields.extend(obj.clone());
    }
    Ok(fields)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn disabled_by_default() {
        let p = parse_request_param_patches(None, None, None, None).unwrap();
        assert!(p.is_none());
    }

    #[test]
    fn thinking_mode_preset_always() {
        let p = parse_request_param_patches(
            Some("true".to_string()),
            Some("always".to_string()),
            Some("thinking-mode".to_string()),
            None,
        )
        .unwrap()
        .unwrap();
        assert_eq!(p.mode, PatchMode::Always);
        assert_eq!(p.fields["temperature"], json!(0.6));
    }

    #[test]
    fn json_merges_with_preset() {
        let p = parse_request_param_patches(
            Some("true".to_string()),
            None,
            Some("thinking-mode".to_string()),
            Some(r#"{"temperature":0.7}"#.to_string()),
        )
        .unwrap()
        .unwrap();
        assert_eq!(p.fields["temperature"], json!(0.7));
        assert_eq!(p.fields["top_p"], json!(0.95));
    }
}
