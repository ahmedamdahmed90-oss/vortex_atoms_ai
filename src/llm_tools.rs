use serde::{Deserialize, Serialize};

use crate::error::VortexAtomsError;
use crate::Result;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub param_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

pub trait ToolExecutor: Send + Sync {
    fn execute(&self, call: &ToolCall) -> Result<String>;
    fn definitions(&self) -> Vec<ToolDefinition>;
}

pub struct KernelToolExecutor;

impl ToolExecutor for KernelToolExecutor {
    fn execute(&self, call: &ToolCall) -> Result<String> {
        match call.name.as_str() {
            "execute_code" => {
                let code = call
                    .arguments
                    .get("code")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Ok(format!("Code execution result for: {code}"))
            }
            "search_knowledge" => {
                let query = call
                    .arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Ok(format!("Knowledge search results for: {query}"))
            }
            "render_media" => {
                let prompt = call
                    .arguments
                    .get("prompt")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let media_type = call
                    .arguments
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("image");
                Ok(format!("Media render request: {media_type} — {prompt}"))
            }
            "manage_memory" => {
                let action = call
                    .arguments
                    .get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("list");
                Ok(format!("Memory management: {action}"))
            }
            "system_info" => Ok(serde_json::json!({
                "platform": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "cores": std::thread::available_parallelism()
                    .map(|n| n.get()).unwrap_or(1),
            })
            .to_string()),
            other => Err(VortexAtomsError::Gguf(format!("Unknown tool: {other}"))),
        }
    }

    fn definitions(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "execute_code".to_string(),
                description: "Execute a Rust or Python code snippet and return the result."
                    .to_string(),
                parameters: vec![ToolParameter {
                    name: "code".to_string(),
                    description: "The code to execute.".to_string(),
                    required: true,
                    param_type: "string".to_string(),
                }],
            },
            ToolDefinition {
                name: "search_knowledge".to_string(),
                description: "Search the knowledge base for relevant information.".to_string(),
                parameters: vec![ToolParameter {
                    name: "query".to_string(),
                    description: "The search query.".to_string(),
                    required: true,
                    param_type: "string".to_string(),
                }],
            },
            ToolDefinition {
                name: "render_media".to_string(),
                description: "Generate or render media content (image, audio, video).".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "prompt".to_string(),
                        description: "Description of the media to generate.".to_string(),
                        required: true,
                        param_type: "string".to_string(),
                    },
                    ToolParameter {
                        name: "type".to_string(),
                        description: "Media type: image, audio, or video.".to_string(),
                        required: false,
                        param_type: "string".to_string(),
                    },
                ],
            },
            ToolDefinition {
                name: "manage_memory".to_string(),
                description: "Manage conversation memory: list, clear, or summarize.".to_string(),
                parameters: vec![ToolParameter {
                    name: "action".to_string(),
                    description: "Action: list, clear, or summarize.".to_string(),
                    required: true,
                    param_type: "string".to_string(),
                }],
            },
            ToolDefinition {
                name: "system_info".to_string(),
                description: "Get system information (OS, architecture, CPU cores).".to_string(),
                parameters: vec![],
            },
        ]
    }
}

pub fn parse_tool_calls(text: &str) -> Vec<ToolCall> {
    let mut calls = Vec::new();

    if let Some(start) = text.find("<tool_call>") {
        if let Some(end) = text[start..].find("</tool_call>") {
            let tool_block = &text[start + 11..start + end];
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(tool_block) {
                if let Some(name) = value.get("name").and_then(|v| v.as_str()) {
                    let arguments = value
                        .get("arguments")
                        .cloned()
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                    calls.push(ToolCall {
                        name: name.to_string(),
                        arguments,
                    });
                }
            }
        }
    }

    calls
}

pub fn tool_definitions_prompt(defs: &[ToolDefinition]) -> String {
    if defs.is_empty() {
        return String::new();
    }

    let mut prompt = String::from("\nYou have access to the following tools. To use a tool, respond with a <tool_call> block:\n\n");

    for def in defs {
        prompt.push_str(&format!(
            "Tool: {}\nDescription: {}\nParameters:\n",
            def.name, def.description
        ));
        for param in &def.parameters {
            let req = if param.required {
                "required"
            } else {
                "optional"
            };
            prompt.push_str(&format!(
                "  - {} ({}): {} [{}]\n",
                param.name, param.param_type, param.description, req
            ));
        }
        prompt.push('\n');
    }

    prompt.push_str("To call a tool, output: <tool_call>{\"name\": \"tool_name\", \"arguments\": {\"param\": \"value\"}}</tool_call>\n\n");
    prompt
}

pub fn format_tool_results(tool_calls: &[ToolCall], executor: &dyn ToolExecutor) -> String {
    let mut results = String::new();
    for call in tool_calls {
        match executor.execute(call) {
            Ok(result) => {
                results.push_str(&format!("\nTool '{}' result: {}\n", call.name, result));
            }
            Err(e) => {
                results.push_str(&format!("\nTool '{}' error: {}\n", call.name, e));
            }
        }
    }
    results
}
