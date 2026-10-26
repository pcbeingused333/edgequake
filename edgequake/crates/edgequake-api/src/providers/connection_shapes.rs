//! One add-provider preset per non-mock `edgequake_llm::ProviderType`.
//! Aliases stay accepted so existing rows can still be saved.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionShape {
    pub id: &'static str,
    pub label: &'static str,
    pub default_base_url: &'static str,
    pub hint: &'static str,
}

pub const CONNECTION_SHAPES: &[ConnectionShape] = &[
    ConnectionShape {
        id: "openai",
        label: "OpenAI",
        default_base_url: "https://api.openai.com/v1",
        hint: "API key required.",
    },
    ConnectionShape {
        id: "anthropic",
        label: "Anthropic",
        default_base_url: "https://api.anthropic.com",
        hint: "API key required. Messages API.",
    },
    ConnectionShape {
        id: "gemini",
        label: "Gemini",
        default_base_url: "https://generativelanguage.googleapis.com",
        hint: "Google AI Studio API key required.",
    },
    ConnectionShape {
        id: "vertexai",
        label: "Vertex AI",
        default_base_url: "https://aiplatform.googleapis.com",
        hint: "Uses Google Cloud credentials on the server. A saved key cannot build this client.",
    },
    ConnectionShape {
        id: "openrouter",
        label: "OpenRouter",
        default_base_url: "https://openrouter.ai/api/v1",
        hint: "API key required. OpenAI-compatible.",
    },
    ConnectionShape {
        id: "xai",
        label: "xAI",
        default_base_url: "https://api.x.ai/v1",
        hint: "API key required. OpenAI-compatible.",
    },
    ConnectionShape {
        id: "huggingface",
        label: "Hugging Face",
        default_base_url: "https://api-inference.huggingface.co/v1",
        hint: "API key required. OpenAI-compatible.",
    },
    ConnectionShape {
        id: "openai-compatible",
        label: "OpenAI-compatible",
        default_base_url: "https://api.openai.com/v1",
        hint: "Any server that speaks chat completions. Key optional.",
    },
    ConnectionShape {
        id: "ollama",
        label: "Ollama",
        default_base_url: "http://127.0.0.1:11434",
        hint: "Local Ollama. A key is optional.",
    },
    ConnectionShape {
        id: "lmstudio",
        label: "LM Studio",
        default_base_url: "http://127.0.0.1:1234/v1",
        hint: "Local LM Studio. A key is optional.",
    },
    ConnectionShape {
        id: "omlx",
        label: "oMLX",
        default_base_url: "http://127.0.0.1:9050/v1",
        hint: "Local oMLX. A key is optional.",
    },
    ConnectionShape {
        id: "mtplx",
        label: "MTPLX",
        default_base_url: "http://127.0.0.1:8080/v1",
        hint: "Local MTPLX. A key is optional.",
    },
    ConnectionShape {
        id: "llamacpp",
        label: "llama.cpp",
        default_base_url: "http://127.0.0.1:8080/v1",
        hint: "llama-server. A key is optional.",
    },
    ConnectionShape {
        id: "vllm-mlx",
        label: "vLLM-MLX",
        default_base_url: "http://127.0.0.1:8000/v1",
        hint: "Local vLLM-MLX. A key is optional.",
    },
    ConnectionShape {
        id: "mlx-lm",
        label: "MLX-LM",
        default_base_url: "http://127.0.0.1:8080/v1",
        hint: "mlx_lm.server. A key is optional.",
    },
    ConnectionShape {
        id: "vscode-copilot",
        label: "VS Code Copilot",
        default_base_url: "https://api.githubcopilot.com",
        hint: "Uses the Copilot proxy on the server. A saved key cannot build this client.",
    },
    ConnectionShape {
        id: "mistral",
        label: "Mistral",
        default_base_url: "https://api.mistral.ai/v1",
        hint: "API key required. OpenAI-compatible.",
    },
    ConnectionShape {
        id: "azure",
        label: "Azure OpenAI",
        default_base_url: "https://YOUR_RESOURCE.openai.azure.com",
        hint: "API key required. OpenAI-compatible endpoint.",
    },
    ConnectionShape {
        id: "nvidia",
        label: "NVIDIA",
        default_base_url: "https://integrate.api.nvidia.com/v1",
        hint: "API key required. OpenAI-compatible.",
    },
    ConnectionShape {
        id: "cohere",
        label: "Cohere",
        default_base_url: "https://api.cohere.com/v1",
        hint: "API key required. OpenAI-compatible.",
    },
    ConnectionShape {
        id: "bedrock",
        label: "AWS Bedrock",
        default_base_url: "https://bedrock-runtime.us-east-1.amazonaws.com",
        hint: "Uses AWS credentials on the server. A saved key cannot build this client.",
    },
];

const EXTRA_SHAPES: &[&str] = &[
    "openai_chat",
    "openai_responses",
    "openai_compatible",
    "anthropic_messages",
    "claude",
    "mlx_lm",
    "lm-studio",
    "lm_studio",
    "o-mlx",
    "o_mlx",
    "mtp-lx",
    "llama-server",
    "llama.cpp",
    "vllm_mlx",
    "vertex",
    "grok",
    "hf",
    "huggingface",
    "google",
    "azure-openai",
    "azure_openai",
    "nim",
    "copilot",
    "vscode",
];

/// Unknown shapes are rejected so a typo cannot be stored as a custom provider.
pub fn accept_api_shape(shape: &str) -> bool {
    let normalized = shape.trim().to_ascii_lowercase();
    CONNECTION_SHAPES
        .iter()
        .any(|preset| preset.id == normalized)
        || EXTRA_SHAPES.contains(&normalized.as_str())
}

/// Shapes that speak `{base}/v1/models` and chat completions.
pub fn is_openai_wire(shape: &str) -> bool {
    matches!(
        shape.trim().to_ascii_lowercase().as_str(),
        "openai"
            | "openai_chat"
            | "openai_responses"
            | "openai-compatible"
            | "openai_compatible"
            | "openrouter"
            | "mistral"
            | "xai"
            | "grok"
            | "nvidia"
            | "nim"
            | "cohere"
            | "huggingface"
            | "hf"
            | "azure"
            | "azure-openai"
            | "azure_openai"
            | "lmstudio"
            | "lm-studio"
            | "lm_studio"
            | "omlx"
            | "o-mlx"
            | "o_mlx"
            | "mtplx"
            | "mtp-lx"
            | "llamacpp"
            | "llama-server"
            | "llama.cpp"
            | "vllm-mlx"
            | "vllm_mlx"
            | "mlx-lm"
            | "mlx_lm"
    )
}

/// Saved URL and key cannot construct these clients.
pub fn requires_server_credentials(shape: &str) -> bool {
    matches!(
        shape.trim().to_ascii_lowercase().as_str(),
        "vertexai" | "vertex" | "bedrock" | "vscode-copilot" | "vscode" | "copilot"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_llm::ProviderType;

    #[test]
    fn presets_and_aliases_are_accepted_and_typos_are_not() {
        assert!(accept_api_shape("openai"));
        assert!(accept_api_shape("openai_chat"));
        assert!(accept_api_shape("Anthropic_Messages"));
        assert!(accept_api_shape("lmstudio"));
        assert!(accept_api_shape("openrouter"));
        assert!(accept_api_shape("mistral"));
        assert!(!accept_api_shape(""));
        assert!(!accept_api_shape("custom-http"));
        assert!(!accept_api_shape("openai_chat/../"));
        assert!(!accept_api_shape("mock"));
    }

    #[test]
    fn every_non_mock_provider_type_is_a_connection_shape() {
        let ids: Vec<_> = CONNECTION_SHAPES.iter().map(|shape| shape.id).collect();
        for kind in ProviderType::all() {
            if kind.canonical_id() == "mock" {
                continue;
            }
            assert!(
                ids.contains(&kind.canonical_id()),
                "missing shape {}",
                kind.canonical_id()
            );
            assert!(accept_api_shape(kind.canonical_id()));
        }
    }
}
