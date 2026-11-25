#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum AiProviderType {
    OpenAi,     // 0
    Anthropic,  // 1
    DeepSeek,   // 2
    Ollama,     // 3 本地
    OpenRouter, // 4 聚合
    Moonshot,   // 5
    Qwen,       // 6 阿里云百炼 / DashScope
    Zhipu,      // 7 智谱 AI / Z.ai
    Xai,        // 8 xAI Grok
    Custom,     // 20 OpenAI 兼容中转（始终最后，编码留空档给后续厂商）
}

impl AiProviderType {
    pub const ALL: [Self; 10] = [Self::OpenAi, Self::Anthropic, Self::DeepSeek, Self::Ollama, Self::OpenRouter, Self::Moonshot, Self::Qwen, Self::Zhipu, Self::Xai, Self::Custom];

    pub fn from_i16(code: i16) -> Option<Self> {
        match code {
            0 => Some(Self::OpenAi),
            1 => Some(Self::Anthropic),
            2 => Some(Self::DeepSeek),
            3 => Some(Self::Ollama),
            4 => Some(Self::OpenRouter),
            5 => Some(Self::Moonshot),
            6 => Some(Self::Qwen),
            7 => Some(Self::Zhipu),
            8 => Some(Self::Xai),
            20 => Some(Self::Custom),
            _ => None,
        }
    }

    pub fn as_i16(self) -> i16 {
        match self {
            Self::OpenAi => 0,
            Self::Anthropic => 1,
            Self::DeepSeek => 2,
            Self::Ollama => 3,
            Self::OpenRouter => 4,
            Self::Moonshot => 5,
            Self::Qwen => 6,
            Self::Zhipu => 7,
            Self::Xai => 8,
            Self::Custom => 20,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::OpenAi => "OpenAI",
            Self::Anthropic => "Anthropic",
            Self::DeepSeek => "DeepSeek",
            Self::Ollama => "Ollama",
            Self::OpenRouter => "OpenRouter",
            Self::Moonshot => "Moonshot Kimi",
            Self::Qwen => "阿里千问 (百炼)",
            Self::Zhipu => "智谱 AI (Z.ai)",
            Self::Xai => "xAI Grok",
            Self::Custom => "自定义中转 (OpenAI 兼容)",
        }
    }

    pub fn default_url(self) -> Option<&'static str> {
        match self {
            Self::OpenAi => Some("https://api.openai.com/v1"),
            Self::Anthropic => Some("https://api.anthropic.com"),
            Self::DeepSeek => Some("https://api.deepseek.com"),
            Self::Ollama => Some("http://localhost:11434/v1"),
            Self::OpenRouter => Some("https://openrouter.ai/api/v1"),
            Self::Moonshot => Some("https://api.moonshot.cn/v1"),
            Self::Qwen => Some("https://dashscope.aliyuncs.com/compatible-mode/v1"),
            Self::Zhipu => Some("https://api.z.ai/api/paas/v4"),
            Self::Xai => Some("https://api.x.ai/v1"),
            Self::Custom => None,
        }
    }
}
