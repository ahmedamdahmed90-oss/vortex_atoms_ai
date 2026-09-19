use std::collections::VecDeque;

use crate::llm_inference::LlmInference;
use crate::llm_prompt::{ChatMessage, MessageRole};
use crate::Result;

pub struct MemoryConsolidator {
    threshold_turns: usize,
    max_summary_tokens: usize,
}

#[derive(Clone, Debug)]
pub struct ConversationSummary {
    pub summary: String,
    pub turn_count: usize,
    pub key_topics: Vec<String>,
}

impl MemoryConsolidator {
    pub fn new(threshold_turns: usize, max_summary_tokens: usize) -> Self {
        Self {
            threshold_turns,
            max_summary_tokens,
        }
    }

    pub fn default_config() -> Self {
        Self {
            threshold_turns: 20,
            max_summary_tokens: 256,
        }
    }

    pub fn should_consolidate(&self, history_len: usize) -> bool {
        history_len >= self.threshold_turns * 2
    }

    pub fn consolidate(
        &self,
        engine: &mut LlmInference,
        history: &[ChatMessage],
    ) -> Result<ConsolidationResult> {
        if history.is_empty() {
            return Ok(ConsolidationResult {
                summary: None,
                messages_removed: 0,
                topics: vec![],
            });
        }

        let conversation_text: Vec<String> = history
            .iter()
            .map(|m| {
                let role = match m.role {
                    MessageRole::User => "User",
                    MessageRole::Assistant => "Assistant",
                    MessageRole::System => "System",
                };
                format!("{role}: {}", m.content)
            })
            .collect();

        let full_conversation = conversation_text.join("\n");

        let summary_prompt = format!(
            "Summarize the following conversation in 2-4 sentences. \
             Identify the key topics discussed. Respond in this exact JSON format:\n\
             {{\"summary\": \"your summary here\", \"topics\": [\"topic1\", \"topic2\"]}}\n\n\
             Conversation:\n{}",
            full_conversation
        );

        let raw_response = engine.generate(&summary_prompt, Some(self.max_summary_tokens))?;

        let parsed = parse_summary_response(&raw_response);

        let topics = parsed.1.clone();

        Ok(ConsolidationResult {
            summary: Some(ConversationSummary {
                summary: parsed.0,
                turn_count: history.len() / 2,
                key_topics: parsed.1,
            }),
            messages_removed: history.len(),
            topics,
        })
    }

    pub fn build_consolidated_context(
        &self,
        summaries: &[ConversationSummary],
        recent_history: &[ChatMessage],
    ) -> Vec<ChatMessage> {
        let mut messages = Vec::new();

        if !summaries.is_empty() {
            let summary_text: Vec<String> = summaries
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    format!(
                        "Previous conversation {} ({} turns, topics: {}): {}",
                        i + 1,
                        s.turn_count,
                        s.key_topics.join(", "),
                        s.summary
                    )
                })
                .collect();

            messages.push(ChatMessage {
                role: MessageRole::System,
                content: format!(
                    "Context from previous conversations:\n{}",
                    summary_text.join("\n\n")
                ),
            });
        }

        for msg in recent_history {
            messages.push(msg.clone());
        }

        messages
    }
}

impl Default for MemoryConsolidator {
    fn default() -> Self {
        Self::default_config()
    }
}

#[derive(Clone, Debug)]
pub struct ConsolidationResult {
    pub summary: Option<ConversationSummary>,
    pub messages_removed: usize,
    pub topics: Vec<String>,
}

pub struct ConversationMemory {
    history: VecDeque<ChatMessage>,
    summaries: Vec<ConversationSummary>,
    consolidator: MemoryConsolidator,
    max_recent: usize,
}

impl ConversationMemory {
    pub fn new(consolidator: MemoryConsolidator, max_recent: usize) -> Self {
        Self {
            history: VecDeque::new(),
            summaries: Vec::new(),
            consolidator,
            max_recent,
        }
    }

    pub fn push(&mut self, message: ChatMessage) {
        self.history.push_back(message);
    }

    pub fn get_context_messages(&self) -> Vec<ChatMessage> {
        let recent: Vec<ChatMessage> = self
            .history
            .iter()
            .rev()
            .take(self.max_recent * 2)
            .rev()
            .cloned()
            .collect();

        self.consolidator
            .build_consolidated_context(&self.summaries, &recent)
    }

    pub fn try_consolidate(&mut self, engine: &mut LlmInference) -> Result<bool> {
        if !self.consolidator.should_consolidate(self.history.len()) {
            return Ok(false);
        }

        let history_vec: Vec<ChatMessage> = self.history.iter().cloned().collect();
        let result = self.consolidator.consolidate(engine, &history_vec)?;

        if let Some(summary) = result.summary {
            self.summaries.push(summary);

            let keep_count = self.max_recent * 2;
            while self.history.len() > keep_count {
                self.history.pop_front();
            }

            return Ok(true);
        }

        Ok(false)
    }

    pub fn clear(&mut self) {
        self.history.clear();
        self.summaries.clear();
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    pub fn summary_count(&self) -> usize {
        self.summaries.len()
    }

    pub fn summaries(&self) -> &[ConversationSummary] {
        &self.summaries
    }
}

fn parse_summary_response(response: &str) -> (String, Vec<String>) {
    let cleaned = if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            &response[start..=end]
        } else {
            response
        }
    } else {
        response
    };

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(cleaned) {
        let summary = value
            .get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let topics = value
            .get("topics")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        return (summary, topics);
    }

    (response.to_string(), vec![])
}
