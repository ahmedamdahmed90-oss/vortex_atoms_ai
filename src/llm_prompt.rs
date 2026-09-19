#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChatTemplate {
    ChatML,
    Llama,
    Phi3,
    Gemma,
}

impl ChatTemplate {
    pub fn default_for_arch(arch: &str) -> Self {
        match arch {
            "qwen2" | "qwen3" => Self::ChatML,
            "llama" => Self::Llama,
            "phi3" => Self::Phi3,
            "gemma3" => Self::Gemma,
            _ => Self::ChatML,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

pub fn format_chat_prompt(
    template: &ChatTemplate,
    system_prompt: &str,
    user_message: &str,
    assistant_prefix: Option<&str>,
) -> String {
    let messages = vec![
        ChatMessage {
            role: MessageRole::System,
            content: system_prompt.to_string(),
        },
        ChatMessage {
            role: MessageRole::User,
            content: user_message.to_string(),
        },
    ];
    format_chat_history(template, &messages, assistant_prefix)
}

pub fn format_chat_history(
    template: &ChatTemplate,
    messages: &[ChatMessage],
    assistant_prefix: Option<&str>,
) -> String {
    match template {
        ChatTemplate::ChatML => {
            let mut p = String::new();
            for msg in messages {
                let role_str = match msg.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                };
                p.push_str(&format!("<|im_start|>{}\n{}\n", role_str, msg.content));
            }
            p.push_str("<|im_start|>assistant\n");
            if let Some(prefix) = assistant_prefix {
                p.push_str(prefix);
            }
            p
        }
        ChatTemplate::Llama => {
            let mut p = String::new();
            p.push_str("<s>");
            for (i, msg) in messages.iter().enumerate() {
                match msg.role {
                    MessageRole::System => {
                        p.push_str(&format!("[INST] <<SYS>>\n{}\n<</SYS>>\n\n", msg.content));
                    }
                    MessageRole::User => {
                        if i > 0 && messages[i - 1].role == MessageRole::System {
                            p.push_str(&format!("{} [/INST] ", msg.content));
                        } else {
                            p.push_str(&format!("[INST] {} [/INST] ", msg.content));
                        }
                    }
                    MessageRole::Assistant => {
                        p.push_str(&format!("{} ", msg.content));
                    }
                }
            }
            if let Some(prefix) = assistant_prefix {
                p.push_str(prefix);
            }
            p
        }
        ChatTemplate::Phi3 => {
            let mut p = String::new();
            for msg in messages {
                let role_str = match msg.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                };
                p.push_str(&format!("<|{}|>\n{}\n", role_str, msg.content));
            }
            p.push_str("<|assistant|>\n");
            if let Some(prefix) = assistant_prefix {
                p.push_str(prefix);
            }
            p
        }
        ChatTemplate::Gemma => {
            let mut p = String::new();
            p.push_str("<bos>");
            for msg in messages {
                let (role_str, other_content) = match msg.role {
                    MessageRole::System => ("user", Some(&msg.content)),
                    MessageRole::User => ("user", None),
                    MessageRole::Assistant => ("model", None),
                };
                p.push_str(&format!("<start_of_turn>{}\n", role_str));
                if let Some(content) = other_content {
                    p.push_str(content);
                    p.push('\n');
                    p.push_str(&msg.content);
                } else {
                    p.push_str(&msg.content);
                }
                p.push_str("<end_of_turn>\n");
            }
            p.push_str("<start_of_turn>model\n");
            if let Some(prefix) = assistant_prefix {
                p.push_str(prefix);
            }
            p
        }
    }
}

pub fn count_tokens_approx(text: &str) -> usize {
    text.len() / 4
}
