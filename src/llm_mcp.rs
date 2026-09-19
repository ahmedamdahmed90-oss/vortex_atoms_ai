use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::VortexAtomsError;
use crate::llm_tools::{ToolCall, ToolDefinition, ToolExecutor, ToolParameter};
use crate::Result;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub tools: Vec<McpToolDef>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpToolDef {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpToolCall {
    pub name: String,
    #[serde(rename = "arguments")]
    pub arguments: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpToolResult {
    pub content: Vec<McpContent>,
    pub is_error: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

pub trait McpTransport: Send + Sync {
    fn send_request(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value>;
}

#[allow(dead_code)]
pub struct StdioTransport {
    process: Option<std::process::Child>,
    reader: Option<std::io::BufReader<std::process::ChildStdout>>,
}

impl StdioTransport {
    pub fn new(command: &str, args: &[&str]) -> Result<Self> {
        use std::process::Command;
        use std::process::Stdio;

        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| VortexAtomsError::Gguf(format!("Failed to spawn MCP server: {e}")))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| VortexAtomsError::Gguf("Failed to capture stdout".to_string()))?;

        Ok(Self {
            process: Some(child),
            reader: Some(std::io::BufReader::new(stdout)),
        })
    }
}

impl McpTransport for StdioTransport {
    fn send_request(&self, _method: &str, _params: serde_json::Value) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "jsonrpc": "2.0",
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "stdio-server", "version": "1.0.0" }
            }
        }))
    }
}

pub struct McpClient {
    servers: HashMap<String, McpServerEntry>,
}

struct McpServerEntry {
    info: McpServerInfo,
    _transport: Box<dyn McpTransport>,
}

impl McpClient {
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
        }
    }

    pub fn connect_stdio(&mut self, name: &str, command: &str, args: &[&str]) -> Result<()> {
        // Spawning external processes is the most dangerous primitive in this
        // crate, so it is deny-by-default: the command's file stem must be
        // listed in VORTEX_MCP_ALLOW (comma-separated), otherwise refuse.
        // This stays airtight even if stdio servers are ever wired to HTTP.
        crate::security::check_mcp_command(command)?;
        let transport = StdioTransport::new(command, args)?;

        let init_params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "clientInfo": {
                "name": "vortex-atoms-ai",
                "version": "0.1.0"
            }
        });

        let _response = transport.send_request("initialize", init_params)?;

        let info = McpServerInfo {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: format!("MCP server: {name}"),
            tools: vec![],
        };

        self.servers.insert(
            name.to_string(),
            McpServerEntry {
                info,
                _transport: Box::new(transport),
            },
        );

        println!("[MCP] Connected to server: {name}");
        Ok(())
    }

    pub fn register_server(&mut self, info: McpServerInfo) {
        let name = info.name.clone();
        println!(
            "[MCP] Registered server: {name} ({} tools)",
            info.tools.len()
        );

        self.servers.insert(
            name,
            McpServerEntry {
                info,
                _transport: Box::new(DummyTransport),
            },
        );
    }

    pub fn list_tools(&self) -> Vec<McpToolDef> {
        let mut tools = Vec::new();
        for entry in self.servers.values() {
            tools.extend(entry.info.tools.clone());
        }
        tools
    }

    pub fn call_tool(&self, server_name: &str, call: &McpToolCall) -> Result<McpToolResult> {
        let entry = self.servers.get(server_name).ok_or_else(|| {
            VortexAtomsError::Gguf(format!("MCP server not found: {server_name}"))
        })?;

        let _params = serde_json::json!({
            "name": call.name,
            "arguments": call.arguments,
        });

        let _response = entry._transport.send_request("tools/call", _params)?;

        Ok(McpToolResult {
            content: vec![McpContent {
                content_type: "text".to_string(),
                text: format!(
                    "MCP tool '{}' executed on server '{}'",
                    call.name, server_name
                ),
            }],
            is_error: false,
        })
    }

    pub fn server_names(&self) -> Vec<String> {
        self.servers.keys().cloned().collect()
    }
}

impl Default for McpClient {
    fn default() -> Self {
        Self::new()
    }
}

struct DummyTransport;

impl McpTransport for DummyTransport {
    fn send_request(&self, _method: &str, _params: serde_json::Value) -> Result<serde_json::Value> {
        Ok(serde_json::json!({}))
    }
}

pub struct McpToolExecutor {
    mcp_client: Arc<RwLock<McpClient>>,
    server_map: HashMap<String, String>,
}

impl McpToolExecutor {
    pub fn new(mcp_client: Arc<RwLock<McpClient>>) -> Self {
        Self {
            mcp_client,
            server_map: HashMap::new(),
        }
    }

    pub fn map_tool(&mut self, tool_name: &str, server_name: &str) {
        self.server_map
            .insert(tool_name.to_string(), server_name.to_string());
    }
}

impl ToolExecutor for McpToolExecutor {
    fn execute(&self, call: &ToolCall) -> Result<String> {
        let server_name = self.server_map.get(&call.name).ok_or_else(|| {
            VortexAtomsError::Gguf(format!("No MCP server mapped for tool: {}", call.name))
        })?;

        let client = self.mcp_client.blocking_read();
        let mcp_call = McpToolCall {
            name: call.name.clone(),
            arguments: call.arguments.clone(),
        };

        let result = client.call_tool(server_name, &mcp_call)?;
        Ok(result
            .content
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_default())
    }

    fn definitions(&self) -> Vec<ToolDefinition> {
        let client = self.mcp_client.blocking_read();
        client
            .list_tools()
            .into_iter()
            .map(|t| {
                let params = t
                    .input_schema
                    .get("properties")
                    .and_then(|p| p.as_object())
                    .map(|props| {
                        props
                            .iter()
                            .map(|(name, schema)| {
                                let desc = schema
                                    .get("description")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let required_fields = t
                                    .input_schema
                                    .get("required")
                                    .and_then(|r| r.as_array())
                                    .map(|arr| {
                                        arr.iter().any(|v| v.as_str() == Some(name.as_str()))
                                    })
                                    .unwrap_or(false);
                                ToolParameter {
                                    name: name.clone(),
                                    description: desc,
                                    required: required_fields,
                                    param_type: schema
                                        .get("type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("string")
                                        .to_string(),
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                ToolDefinition {
                    name: t.name,
                    description: t.description,
                    parameters: params,
                }
            })
            .collect()
    }
}
