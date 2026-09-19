use memmap2::{Mmap, MmapOptions};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::{Path, PathBuf};

use crate::error::{Result, VortexAtomsError};
use crate::knowledge_orchestrator::KnowledgeFragmentDescriptor;

pub const FINANCE_MATH_MODULE_ID: &str = "finance_math_module";
pub const FINANCE_MATH_MODULE_PATH: &str = "knowledge/finance_math_module.tcz";
pub const FINANCE_MATH_MAX_MAPPED_BYTES: u64 = 50 * 1024 * 1024;

/// Returns the default Kernel_02 registration descriptor for the mmap-backed
/// finance, economics, logistics, risk, and statistics extension.
pub fn finance_math_descriptor() -> KnowledgeFragmentDescriptor {
    KnowledgeFragmentDescriptor::new(
        FINANCE_MATH_MODULE_ID,
        "code_logic",
        "macro micro economics corporate finance factory management workflow logistics risk assessment statistics forecasting monte carlo optimization strategic planning",
        FINANCE_MATH_MODULE_PATH,
    )
}

/// Memory-mapped finance and mathematical strategy extension.
///
/// The compiled TCZ file is bounded before mapping, opened read-only, queried for
/// one Kernel_03 request, and explicitly dropped after use to avoid resident
/// runtime leaks.
pub struct FinanceMathModule {
    path: PathBuf,
    mmap: Mmap,
}

impl FinanceMathModule {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let metadata = file.metadata()?;

        if metadata.len() == 0 {
            return Err(VortexAtomsError::InvalidKnowledgeExtension(format!(
                "finance math module is empty: {}",
                path.display()
            )));
        }

        if metadata.len() > FINANCE_MATH_MAX_MAPPED_BYTES {
            return Err(VortexAtomsError::InvalidKnowledgeExtension(format!(
                "finance math module exceeds mapping budget: {} bytes at {}",
                metadata.len(),
                path.display()
            )));
        }

        // SAFETY:
        // - File is opened read-only and mapped as immutable `Mmap`.
        // - File size is bounded before mapping.
        // - UTF-8 is validated before semantic access.
        // - The mmap is owned by this struct and released when dropped.
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        Ok(Self { path, mmap })
    }

    pub fn load_default() -> Result<Self> {
        Self::load(FINANCE_MATH_MODULE_PATH)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn mapped_bytes(&self) -> usize {
        self.mmap.len()
    }

    pub fn sections(&self) -> Result<Vec<FinanceKnowledgeSection>> {
        let text = std::str::from_utf8(&self.mmap[..]).map_err(|error| {
            VortexAtomsError::InvalidKnowledgeExtension(format!(
                "{} is not valid UTF-8 TCZ text: {error}",
                self.path.display()
            ))
        })?;

        parse_tcz_sections(text)
    }

    pub fn query(&self, prompt: &str) -> Result<FinanceMathAnswer> {
        let prompt_lower = prompt.to_ascii_lowercase();
        let mut scored = self
            .sections()?
            .into_iter()
            .map(|section| (score_section(&prompt_lower, &section), section))
            .collect::<Vec<_>>();

        scored.sort_by_key(|entry| std::cmp::Reverse(entry.0));
        let matched_sections = scored
            .into_iter()
            .filter(|(score, _)| *score > 0)
            .take(6)
            .map(|(_, section)| section)
            .collect::<Vec<_>>();

        Ok(FinanceMathAnswer {
            module_id: FINANCE_MATH_MODULE_ID.to_string(),
            source_path: self.path.display().to_string(),
            mapped_bytes: self.mapped_bytes(),
            mapped_under_budget: self.mapped_bytes() as u64 <= FINANCE_MATH_MAX_MAPPED_BYTES,
            matched_sections,
            strategic_notice: strategic_notice(),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct FinanceKnowledgeSection {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct FinanceMathAnswer {
    pub module_id: String,
    pub source_path: String,
    pub mapped_bytes: usize,
    pub mapped_under_budget: bool,
    pub matched_sections: Vec<FinanceKnowledgeSection>,
    pub strategic_notice: String,
}

impl FinanceMathAnswer {
    pub fn to_kernel_summary(&self) -> String {
        let mut output = format!(
            "{module_id} mmap-loaded from {source_path} ({bytes} mapped bytes; under_budget={under}); matched_sections={count}",
            module_id = self.module_id,
            source_path = self.source_path,
            bytes = self.mapped_bytes,
            under = self.mapped_under_budget,
            count = self.matched_sections.len(),
        );

        output.push('\n');
        output.push_str(&self.strategic_notice);

        for section in &self.matched_sections {
            output.push_str("\n\n## ");
            output.push_str(&section.title);
            output.push('\n');
            output.push_str(&section.content);
        }

        output
    }
}

/// Kernel_03 entry point: load the extension through mmap, query it, and drop it.
pub fn run_finance_math_module(prompt: &str) -> Result<String> {
    let module = FinanceMathModule::load_default()?;
    let answer = module.query(prompt)?;
    let summary = answer.to_kernel_summary();

    // Explicitly release the read-only mmap after the strategy request completes.
    std::mem::drop(module);

    Ok(summary)
}

fn parse_tcz_sections(text: &str) -> Result<Vec<FinanceKnowledgeSection>> {
    let mut sections = Vec::new();
    let mut current_id = String::new();
    let mut current_title = String::new();
    let mut current_tags = Vec::<String>::new();
    let mut current_content = Vec::<String>::new();
    let mut in_section = false;

    for line in text.lines() {
        if let Some(value) = line.strip_prefix("@@section ") {
            if in_section {
                sections.push(FinanceKnowledgeSection {
                    id: current_id.clone(),
                    title: current_title.clone(),
                    tags: current_tags.clone(),
                    content: current_content.join("\n"),
                });
                current_content.clear();
                current_tags.clear();
            }
            current_id = value.trim().to_string();
            current_title = value.trim().replace('_', " ");
            in_section = true;
        } else if let Some(value) = line.strip_prefix("@@title ") {
            current_title = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("@@tags ") {
            current_tags = value
                .split(',')
                .map(|tag| tag.trim().to_ascii_lowercase())
                .filter(|tag| !tag.is_empty())
                .collect();
        } else if line == "@@end" {
            if in_section {
                sections.push(FinanceKnowledgeSection {
                    id: current_id.clone(),
                    title: current_title.clone(),
                    tags: current_tags.clone(),
                    content: current_content.join("\n"),
                });
                current_id.clear();
                current_title.clear();
                current_tags.clear();
                current_content.clear();
                in_section = false;
            }
        } else if in_section && !line.starts_with("VAI_TCZ") {
            current_content.push(line.to_string());
        }
    }

    if in_section {
        sections.push(FinanceKnowledgeSection {
            id: current_id,
            title: current_title,
            tags: current_tags,
            content: current_content.join("\n"),
        });
    }

    if sections.is_empty() {
        Err(VortexAtomsError::InvalidKnowledgeExtension(
            "finance math module contains no TCZ sections".to_string(),
        ))
    } else {
        Ok(sections)
    }
}

fn score_section(prompt_lower: &str, section: &FinanceKnowledgeSection) -> usize {
    let mut score = 0usize;

    for tag in &section.tags {
        if prompt_lower.contains(tag) {
            score = score.saturating_add(8);
        }
    }

    for token in section.title.split_whitespace() {
        let token = token.to_ascii_lowercase();
        if token.len() > 2 && prompt_lower.contains(&token) {
            score = score.saturating_add(3);
        }
    }

    let content_lower = section.content.to_ascii_lowercase();
    for token in prompt_lower.split_whitespace() {
        if token.len() > 2 && content_lower.contains(token) {
            score = score.saturating_add(1);
        }
    }

    score
}

fn strategic_notice() -> String {
    "Strategic boundary: this module provides quantitative decision support, scenario planning, statistical reasoning, and management workflow analysis. It is not personalized investment, legal, tax, accounting, or fiduciary advice; production use requires validated data, controls, and qualified review.".to_string()
}
