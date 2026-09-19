use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;
use tokio::sync::RwLock;

use crate::error::VortexAtomsError;
use crate::llm_tools::{ToolCall, ToolDefinition, ToolExecutor, ToolParameter};
use crate::Result;

pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn description(&self) -> &str;
    fn tools(&self) -> Vec<ToolDefinition>;
    fn execute(&self, call: &ToolCall) -> Result<String>;
    fn on_load(&mut self) -> Result<()> {
        Ok(())
    }
    fn on_unload(&mut self) -> Result<()> {
        Ok(())
    }
}

pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
    tool_map: HashMap<String, usize>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            tool_map: HashMap::new(),
        }
    }

    pub fn register(&mut self, mut plugin: Box<dyn Plugin>) -> Result<()> {
        plugin.on_load()?;
        let index = self.plugins.len();

        for tool in plugin.tools() {
            self.tool_map.insert(tool.name.clone(), index);
        }

        println!(
            "[PluginManager] Registered: {} v{}",
            plugin.name(),
            plugin.version()
        );
        self.plugins.push(plugin);
        Ok(())
    }

    pub fn unregister(&mut self, name: &str) -> Result<()> {
        if let Some(pos) = self.plugins.iter().position(|p| p.name() == name) {
            let mut plugin = self.plugins.remove(pos);

            for tool in plugin.tools() {
                self.tool_map.remove(&tool.name);
            }

            plugin.on_unload()?;
            println!("[PluginManager] Unregistered: {name}");

            self.tool_map.clear();
            for (i, plugin) in self.plugins.iter().enumerate() {
                for tool in plugin.tools() {
                    self.tool_map.insert(tool.name.clone(), i);
                }
            }

            Ok(())
        } else {
            Err(VortexAtomsError::Gguf(format!("Plugin not found: {name}")))
        }
    }

    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.plugins
            .iter()
            .map(|p| PluginInfo {
                name: p.name().to_string(),
                version: p.version().to_string(),
                description: p.description().to_string(),
                tool_count: p.tools().len(),
            })
            .collect()
    }

    pub fn get_tool(&self, tool_name: &str) -> Option<(&dyn Plugin, ToolDefinition)> {
        let plugin_idx = self.tool_map.get(tool_name)?;
        let plugin = &self.plugins[*plugin_idx];
        let tool_def = plugin.tools().into_iter().find(|t| t.name == tool_name)?;
        Some((plugin.as_ref(), tool_def))
    }

    pub fn execute_tool(&self, call: &ToolCall) -> Result<String> {
        let plugin_idx = self
            .tool_map
            .get(&call.name)
            .ok_or_else(|| VortexAtomsError::Gguf(format!("Unknown tool: {}", call.name)))?;

        self.plugins[*plugin_idx].execute(call)
    }

    pub fn all_tool_definitions(&self) -> Vec<ToolDefinition> {
        let mut defs = Vec::new();
        for plugin in &self.plugins {
            defs.extend(plugin.tools());
        }
        defs
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub tool_count: usize,
}

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn name(&self) -> &str {
        "core"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn description(&self) -> &str {
        "Core Vortex tools: system info, math, text processing"
    }

    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "system_info".to_string(),
                description: "Get system information (OS, architecture, CPU cores).".to_string(),
                parameters: vec![],
            },
            ToolDefinition {
                name: "math_eval".to_string(),
                description: "Evaluate a mathematical expression.".to_string(),
                parameters: vec![ToolParameter {
                    name: "expression".to_string(),
                    description: "The math expression to evaluate.".to_string(),
                    required: true,
                    param_type: "string".to_string(),
                }],
            },
            ToolDefinition {
                name: "text_stats".to_string(),
                description: "Get word count, character count, and line count of text.".to_string(),
                parameters: vec![ToolParameter {
                    name: "text".to_string(),
                    description: "The text to analyze.".to_string(),
                    required: true,
                    param_type: "string".to_string(),
                }],
            },
        ]
    }

    fn execute(&self, call: &ToolCall) -> Result<String> {
        match call.name.as_str() {
            "system_info" => {
                let info = serde_json::json!({
                    "os": std::env::consts::OS,
                    "arch": std::env::consts::ARCH,
                    "cores": std::thread::available_parallelism()
                        .map(|n| n.get()).unwrap_or(1),
                    "vortex_version": "0.1.0",
                });
                Ok(info.to_string())
            }
            "math_eval" => {
                let expr = call
                    .arguments
                    .get("expression")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0");
                Ok(format!("Expression result: {expr}"))
            }
            "text_stats" => {
                let text = call
                    .arguments
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let words = text.split_whitespace().count();
                let chars = text.len();
                let lines = text.lines().count();
                Ok(serde_json::json!({
                    "words": words,
                    "characters": chars,
                    "lines": lines,
                })
                .to_string())
            }
            _ => Err(VortexAtomsError::Gguf(format!(
                "Unknown tool: {}",
                call.name
            ))),
        }
    }
}

pub struct ToolingPluginExecutor {
    plugin_manager: Arc<RwLock<PluginManager>>,
}

impl ToolingPluginExecutor {
    pub fn new(plugin_manager: Arc<RwLock<PluginManager>>) -> Self {
        Self { plugin_manager }
    }
}

impl ToolExecutor for ToolingPluginExecutor {
    fn execute(&self, call: &ToolCall) -> Result<String> {
        let manager = self.plugin_manager.blocking_read();
        manager.execute_tool(call)
    }

    fn definitions(&self) -> Vec<ToolDefinition> {
        let manager = self.plugin_manager.blocking_read();
        manager.all_tool_definitions()
    }
}
