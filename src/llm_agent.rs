use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::llm_inference::LlmInference;
use crate::llm_stream::StreamEvent;
use crate::llm_tools::{
    format_tool_results, parse_tool_calls, tool_definitions_prompt, ToolExecutor,
};
use crate::Result;
use tokio::sync::mpsc;

const DEFAULT_MAX_STEPS: usize = 10;
const DEFAULT_MAX_TOKENS_PER_STEP: usize = 1024;

pub struct AgentLoop {
    max_steps: usize,
    max_tokens_per_step: usize,
    cancel_flag: Arc<AtomicBool>,
    step_counter: Arc<AtomicUsize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentStep {
    pub step_number: usize,
    pub thought: String,
    pub tool_calls: Vec<ToolCallInfo>,
    pub tool_results: Vec<String>,
    pub output: Option<String>,
    pub duration_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentStepResult {
    pub steps: Vec<AgentStep>,
    pub final_output: String,
    pub total_tool_calls: usize,
    pub total_duration_ms: u64,
    pub completed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCallInfo {
    pub name: String,
    pub arguments: serde_json::Value,
    pub result: String,
}

impl AgentLoop {
    pub fn new() -> Self {
        Self {
            max_steps: DEFAULT_MAX_STEPS,
            max_tokens_per_step: DEFAULT_MAX_TOKENS_PER_STEP,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            step_counter: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn with_limits(max_steps: usize, max_tokens_per_step: usize) -> Self {
        Self {
            max_steps,
            max_tokens_per_step,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            step_counter: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }

    pub fn current_step(&self) -> usize {
        self.step_counter.load(Ordering::Relaxed)
    }

    pub fn run(
        &self,
        engine: &mut LlmInference,
        executor: &dyn ToolExecutor,
        task: &str,
    ) -> Result<AgentStepResult> {
        self.cancel_flag.store(false, Ordering::Relaxed);
        self.step_counter.store(0, Ordering::Relaxed);

        let tool_defs = executor.definitions();
        let tool_prompt = tool_definitions_prompt(&tool_defs);
        let start = Instant::now();
        let mut steps = Vec::new();
        let mut total_tool_calls = 0usize;

        let system_context = format!(
            "You are an intelligent agent that can reason step-by-step and use tools to accomplish tasks.\n\
             Think carefully about what needs to be done, then either use a tool or provide a final answer.\n\
             {}",
            tool_prompt
        );

        let mut current_context = format!(
            "Task: {}\n\nThink step by step. If you need to use a tool, output a <tool_call> block. \
             When you have the final answer, output it directly.",
            task
        );

        for step_num in 0..self.max_steps {
            if self.is_cancelled() {
                break;
            }

            self.step_counter.store(step_num + 1, Ordering::Relaxed);
            let step_start = Instant::now();

            engine.clear_history();

            let full_prompt = format!("{}\n\n{}", system_context, current_context);
            let response = engine.generate(&full_prompt, Some(self.max_tokens_per_step))?;

            let tool_calls = parse_tool_calls(&response);

            if tool_calls.is_empty() {
                steps.push(AgentStep {
                    step_number: step_num + 1,
                    thought: response.clone(),
                    tool_calls: vec![],
                    tool_results: vec![],
                    output: Some(response.clone()),
                    duration_ms: step_start.elapsed().as_millis() as u64,
                });

                return Ok(AgentStepResult {
                    steps,
                    final_output: response,
                    total_tool_calls,
                    total_duration_ms: start.elapsed().as_millis() as u64,
                    completed: true,
                });
            }

            let tool_call_infos: Vec<ToolCallInfo> = tool_calls
                .iter()
                .map(|tc| {
                    let result = executor
                        .execute(tc)
                        .unwrap_or_else(|e| format!("Error: {e}"));
                    total_tool_calls += 1;
                    ToolCallInfo {
                        name: tc.name.clone(),
                        arguments: tc.arguments.clone(),
                        result: result.clone(),
                    }
                })
                .collect();

            let results_text = format_tool_results(&tool_calls, executor);

            steps.push(AgentStep {
                step_number: step_num + 1,
                thought: response,
                tool_calls: tool_call_infos
                    .iter()
                    .map(|tc| ToolCallInfo {
                        name: tc.name.clone(),
                        arguments: tc.arguments.clone(),
                        result: String::new(),
                    })
                    .collect(),
                tool_results: tool_call_infos.iter().map(|tc| tc.result.clone()).collect(),
                output: None,
                duration_ms: step_start.elapsed().as_millis() as u64,
            });

            current_context = format!(
                "Task: {}\n\nPrevious reasoning:\n{}\n\nTool results:\n{}\n\nNow continue thinking step by step. \
                 Use another tool if needed, or provide the final answer.",
                task,
                steps.iter().map(|s| format!("Step {}: {}", s.step_number, s.thought)).collect::<Vec<_>>().join("\n"),
                results_text,
            );
        }

        let final_output = steps
            .last()
            .and_then(|s| s.output.clone())
            .unwrap_or_else(|| {
                let last_thought = steps.last().map(|s| s.thought.clone()).unwrap_or_default();
                format!(
                    "Agent reached maximum steps ({}) without a final answer. Last thought: {}",
                    self.max_steps, last_thought
                )
            });

        Ok(AgentStepResult {
            steps,
            final_output,
            total_tool_calls,
            total_duration_ms: start.elapsed().as_millis() as u64,
            completed: false,
        })
    }

    pub fn run_streaming(
        &self,
        engine: &mut LlmInference,
        executor: &dyn ToolExecutor,
        task: &str,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<AgentStepResult> {
        self.cancel_flag.store(false, Ordering::Relaxed);
        self.step_counter.store(0, Ordering::Relaxed);

        let tool_defs = executor.definitions();
        let tool_prompt = tool_definitions_prompt(&tool_defs);
        let start = Instant::now();
        let mut steps = Vec::new();
        let mut total_tool_calls = 0usize;

        let system_context = format!(
            "You are an intelligent agent that can reason step-by-step and use tools to accomplish tasks.\n\
             {}",
            tool_prompt
        );

        let mut current_context = format!(
            "Task: {}\n\nThink step by step. Use tools or provide a final answer.",
            task
        );

        for step_num in 0..self.max_steps {
            if self.is_cancelled() {
                break;
            }

            self.step_counter.store(step_num + 1, Ordering::Relaxed);
            let step_start = Instant::now();

            engine.clear_history();
            let full_prompt = format!("{}\n\n{}", system_context, current_context);

            let _ = tx.blocking_send(StreamEvent::Token {
                token_id: 0,
                text: format!("\n--- Step {} ---\n", step_num + 1),
                pos: step_num,
            });

            let response_tx = tx.clone();
            engine.generate_streaming(&full_prompt, Some(self.max_tokens_per_step), response_tx)?;

            let response = engine.generate(&full_prompt, Some(self.max_tokens_per_step))?;
            let tool_calls = parse_tool_calls(&response);

            if tool_calls.is_empty() {
                steps.push(AgentStep {
                    step_number: step_num + 1,
                    thought: response.clone(),
                    tool_calls: vec![],
                    tool_results: vec![],
                    output: Some(response.clone()),
                    duration_ms: step_start.elapsed().as_millis() as u64,
                });

                return Ok(AgentStepResult {
                    steps,
                    final_output: response,
                    total_tool_calls,
                    total_duration_ms: start.elapsed().as_millis() as u64,
                    completed: true,
                });
            }

            let mut tool_call_infos = Vec::new();
            for tc in &tool_calls {
                let result = executor
                    .execute(tc)
                    .unwrap_or_else(|e| format!("Error: {e}"));
                total_tool_calls += 1;
                tool_call_infos.push(ToolCallInfo {
                    name: tc.name.clone(),
                    arguments: tc.arguments.clone(),
                    result: result.clone(),
                });
            }

            let results_text = format_tool_results(&tool_calls, executor);

            steps.push(AgentStep {
                step_number: step_num + 1,
                thought: response,
                tool_calls: tool_call_infos
                    .iter()
                    .map(|tc| ToolCallInfo {
                        name: tc.name.clone(),
                        arguments: tc.arguments.clone(),
                        result: String::new(),
                    })
                    .collect(),
                tool_results: tool_call_infos.iter().map(|tc| tc.result.clone()).collect(),
                output: None,
                duration_ms: step_start.elapsed().as_millis() as u64,
            });

            current_context = format!(
                "Task: {}\n\nPrevious reasoning:\n{}\n\nTool results:\n{}\n\nContinue or give the final answer.",
                task,
                steps.iter().map(|s| format!("Step {}: {}", s.step_number, s.thought)).collect::<Vec<_>>().join("\n"),
                results_text,
            );
        }

        let final_output = steps
            .last()
            .and_then(|s| s.output.clone())
            .unwrap_or_else(|| "Agent reached maximum steps.".to_string());

        Ok(AgentStepResult {
            steps,
            final_output,
            total_tool_calls,
            total_duration_ms: start.elapsed().as_millis() as u64,
            completed: false,
        })
    }
}

impl Default for AgentLoop {
    fn default() -> Self {
        Self::new()
    }
}
