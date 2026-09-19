use serde::{Deserialize, Serialize};

use crate::llm_inference::LlmInference;
use crate::Result;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum OutputFormat {
    Text,
    Json,
    JsonSchema(String),
    Markdown,
}

pub struct StructuredOutput;

impl StructuredOutput {
    pub fn enforce_json(
        inference: &mut LlmInference,
        prompt: &str,
        max_tokens: Option<usize>,
    ) -> Result<String> {
        let json_prompt = format!(
            "{}\n\nRespond ONLY with valid JSON. No markdown, no explanation, no code fences. Just the raw JSON object.",
            prompt
        );
        let raw = inference.generate(&json_prompt, max_tokens)?;
        Ok(extract_json(&raw))
    }

    pub fn enforce_schema(
        inference: &mut LlmInference,
        prompt: &str,
        schema: &str,
        max_tokens: Option<usize>,
    ) -> Result<String> {
        let schema_prompt = format!(
            "{}\n\nRespond ONLY with valid JSON matching this schema:\n{}\n\nNo markdown, no explanation, no code fences. Just the raw JSON.",
            prompt, schema
        );
        let raw = inference.generate(&schema_prompt, max_tokens)?;
        Ok(extract_json(&raw))
    }

    pub fn generate_list(
        inference: &mut LlmInference,
        prompt: &str,
        max_tokens: Option<usize>,
    ) -> Result<Vec<String>> {
        let list_prompt = format!(
            "{}\n\nRespond with a JSON array of strings. Example: [\"item1\", \"item2\"]. No markdown, no explanation.",
            prompt
        );
        let raw = inference.generate(&list_prompt, max_tokens)?;
        let cleaned = extract_json(&raw);

        match serde_json::from_str::<Vec<String>>(&cleaned) {
            Ok(list) => Ok(list),
            Err(_) => {
                let lines: Vec<String> = cleaned
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty() && l != "-" && l != "*")
                    .map(|l| {
                        l.trim_start_matches("- ")
                            .trim_start_matches("* ")
                            .to_string()
                    })
                    .collect();
                Ok(lines)
            }
        }
    }

    pub fn generate_key_value(
        inference: &mut LlmInference,
        prompt: &str,
        max_tokens: Option<usize>,
    ) -> Result<std::collections::HashMap<String, String>> {
        let kv_prompt = format!(
            "{}\n\nRespond with a JSON object where keys and values are both strings. Example: {{\"key\": \"value\"}}. No markdown, no explanation.",
            prompt
        );
        let raw = inference.generate(&kv_prompt, max_tokens)?;
        let cleaned = extract_json(&raw);

        serde_json::from_str(&cleaned)
            .map_err(|e| crate::error::VortexAtomsError::Gguf(format!("Failed to parse JSON: {e}")))
    }

    pub fn with_retry(
        inference: &mut LlmInference,
        prompt: &str,
        max_tokens: Option<usize>,
        max_retries: usize,
    ) -> Result<String> {
        let mut last_error = String::new();

        for attempt in 0..=max_retries {
            let current_prompt = if attempt == 0 {
                format!("{}\n\nRespond ONLY with valid JSON.", prompt)
            } else {
                format!(
                    "{}\n\nYour previous response was not valid JSON. Error: {}\nPlease respond ONLY with valid JSON. No markdown, no explanation, no code fences.",
                    prompt, last_error
                )
            };

            match inference.generate(&current_prompt, max_tokens) {
                Ok(raw) => {
                    let cleaned = extract_json(&raw);
                    if serde_json::from_str::<serde_json::Value>(&cleaned).is_ok() {
                        return Ok(cleaned);
                    }
                    last_error = "Response was not valid JSON".to_string();
                }
                Err(e) => {
                    last_error = e.to_string();
                }
            }
        }

        Err(crate::error::VortexAtomsError::Gguf(format!(
            "Failed to generate valid JSON after {max_retries} retries: {last_error}"
        )))
    }
}

fn extract_json(text: &str) -> String {
    let trimmed = text.trim();

    if trimmed.starts_with("```json") {
        let start = 7;
        let end = trimmed.rfind("```").unwrap_or(trimmed.len());
        return trimmed[start..end].trim().to_string();
    }

    if trimmed.starts_with("```") {
        let start = 3;
        let end = trimmed.rfind("```").unwrap_or(trimmed.len());
        return trimmed[start..end].trim().to_string();
    }

    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            if end > start {
                return trimmed[start..=end].to_string();
            }
        }
    }

    if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            if end > start {
                return trimmed[start..=end].to_string();
            }
        }
    }

    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_from_code_fence() {
        let input = "```json\n{\"key\": \"value\"}\n```";
        assert_eq!(extract_json(input), "{\"key\": \"value\"}");
    }

    #[test]
    fn test_extract_json_bare() {
        let input = "Here is the result: {\"key\": \"value\"} hope that helps";
        assert_eq!(extract_json(input), "{\"key\": \"value\"}");
    }

    #[test]
    fn test_extract_json_array() {
        let input = "[\"a\", \"b\", \"c\"]";
        assert_eq!(extract_json(input), "[\"a\", \"b\", \"c\"]");
    }
}
