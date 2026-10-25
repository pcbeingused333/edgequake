//! Presets for a custom provider. OpenAI-shaped and Anthropic-shaped servers are
//! first-class. Aliases stay accepted so existing rows can still be saved.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionShape {
    pub id: &'static str,
    pub label: &'static str,
    pub default_base_url: &'static str,
    pub hint: &'static str,
}

pub const CONNECTION_SHAPES: &[ConnectionShape] = &[
    ConnectionShape {
        id: "openai_chat",
        label: "OpenAI-compatible",
        default_base_url: "https://api.openai.com/v1",
        hint: "OpenAI, and any server that speaks the chat completions API.",
    },
    ConnectionShape {
        id: "anthropic_messages",
        label: "Anthropic-compatible",
        default_base_url: "https://api.anthropic.com",
        hint: "Anthropic, and any server that speaks the Messages API.",
    },
    ConnectionShape {
        id: "ollama",
        label: "Ollama",
        default_base_url: "http://127.0.0.1:11434",
        hint: "Local Ollama. A key is optional.",
    },
];

const EXTRA_SHAPES: &[&str] = &[
    "openai",
    "openai_responses",
    "openai-compatible",
    "openai_compatible",
    "anthropic",
    "claude",
    "omlx",
    "mlx-lm",
    "mlx_lm",
    "llamacpp",
    "vllm-mlx",
    "lmstudio",
    "mtplx",
];

/// Unknown shapes are rejected so a typo cannot be stored as a custom provider.
pub fn accept_api_shape(shape: &str) -> bool {
    let normalized = shape.trim().to_ascii_lowercase();
    CONNECTION_SHAPES
        .iter()
        .any(|preset| preset.id == normalized)
        || EXTRA_SHAPES.contains(&normalized.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_and_aliases_are_accepted_and_typos_are_not() {
        assert!(accept_api_shape("openai_chat"));
        assert!(accept_api_shape("Anthropic_Messages"));
        assert!(accept_api_shape("lmstudio"));
        assert!(!accept_api_shape(""));
        assert!(!accept_api_shape("custom-http"));
        assert!(!accept_api_shape("openai_chat/../"));
    }
}
