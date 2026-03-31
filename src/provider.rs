#[derive(Debug, Clone)]
pub struct ProviderDefinition {
    pub id: &'static str,
    pub display_name: &'static str,
    pub base_url: &'static str,
    pub auth_header: &'static str,
    pub auth_scheme: AuthScheme,
    pub api_style: ProviderApiStyle,
    pub extra_headers: &'static [(&'static str, &'static str)],
}

#[derive(Debug, Clone, Copy)]
pub enum AuthScheme {
    Bearer,
    Raw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderApiStyle {
    ChatCompletions,
    AnthropicMessages,
}

#[derive(Debug, Clone)]
pub struct ProviderRegistry {
    providers: Vec<ProviderDefinition>,
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self {
            providers: vec![
                ProviderDefinition {
                    id: "openai",
                    display_name: "OpenAI",
                    base_url: "https://api.openai.com",
                    auth_header: "authorization",
                    auth_scheme: AuthScheme::Bearer,
                    api_style: ProviderApiStyle::ChatCompletions,
                    extra_headers: &[],
                },
                ProviderDefinition {
                    id: "claude",
                    display_name: "Claude (Anthropic)",
                    base_url: "https://api.anthropic.com",
                    auth_header: "x-api-key",
                    auth_scheme: AuthScheme::Raw,
                    api_style: ProviderApiStyle::AnthropicMessages,
                    extra_headers: &[("anthropic-version", "2023-06-01")],
                },
                ProviderDefinition {
                    id: "deepseek",
                    display_name: "DeepSeek",
                    base_url: "https://api.deepseek.com",
                    auth_header: "authorization",
                    auth_scheme: AuthScheme::Bearer,
                    api_style: ProviderApiStyle::ChatCompletions,
                    extra_headers: &[],
                },
                ProviderDefinition {
                    id: "nvidia",
                    display_name: "NVIDIA Build",
                    base_url: "https://integrate.api.nvidia.com",
                    auth_header: "authorization",
                    auth_scheme: AuthScheme::Bearer,
                    api_style: ProviderApiStyle::ChatCompletions,
                    extra_headers: &[],
                },
            ],
        }
    }
}

impl ProviderRegistry {
    pub fn get(&self, id: &str) -> Option<&ProviderDefinition> {
        self.providers.iter().find(|provider| provider.id == id)
    }

    pub fn all(&self) -> &[ProviderDefinition] {
        &self.providers
    }
}
