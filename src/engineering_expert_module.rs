use memmap2::{Mmap, MmapOptions};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::{Path, PathBuf};

use crate::error::{Result, VortexAtomsError};
use crate::knowledge_orchestrator::KnowledgeFragmentDescriptor;

pub const ENGINEERING_EXPERT_MODULE_ID: &str = "engineering_expert_module";
pub const ENGINEERING_EXPERT_MODULE_PATH: &str = "knowledge/engineering_expert_module.tcz";

/// Returns the default Kernel_02 registration descriptor for the mmap-backed
/// engineering extension. Kernel_03 is the only kernel that executes the module;
/// Kernel_02 uses this descriptor only for semantic detection/routing.
pub fn engineering_expert_descriptor() -> KnowledgeFragmentDescriptor {
    KnowledgeFragmentDescriptor::new(
        ENGINEERING_EXPERT_MODULE_ID,
        "code_logic",
        "electronics repair pcb diagnostics lithium battery recycling algorithms metal detector schematics precious metal recovery chemical process safety",
        ENGINEERING_EXPERT_MODULE_PATH,
    )
}

/// Memory-mapped engineering expert extension.
///
/// The extension is mapped read-only on demand and dropped immediately after the
/// triggering Kernel_03 request is answered. This keeps the compressed knowledge
/// pack out of persistent heap state.
pub struct EngineeringExpertModule {
    path: PathBuf,
    mmap: Mmap,
}

impl EngineeringExpertModule {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let metadata = file.metadata()?;

        if metadata.len() == 0 {
            return Err(VortexAtomsError::InvalidKnowledgeExtension(format!(
                "engineering expert module is empty: {}",
                path.display()
            )));
        }

        // SAFETY:
        // - Opened read-only and mapped as immutable `Mmap`.
        // - The parser validates UTF-8 before reading semantic sections.
        // - The mmap is owned by this struct and released when the struct drops.
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        Ok(Self { path, mmap })
    }

    pub fn load_default() -> Result<Self> {
        Self::load(ENGINEERING_EXPERT_MODULE_PATH)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn compressed_bytes(&self) -> usize {
        self.mmap.len()
    }

    pub fn sections(&self) -> Result<Vec<EngineeringKnowledgeSection>> {
        let text = std::str::from_utf8(&self.mmap[..]).map_err(|error| {
            VortexAtomsError::InvalidKnowledgeExtension(format!(
                "{} is not valid UTF-8 TCZ text: {error}",
                self.path.display()
            ))
        })?;

        parse_tcz_sections(text)
    }

    pub fn query(&self, prompt: &str) -> Result<EngineeringExpertAnswer> {
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
            .take(4)
            .map(|(_, section)| section)
            .collect::<Vec<_>>();

        let safety_bounded = contains_hazardous_chemistry_request(&prompt_lower)
            || contains_lithium_handling_request(&prompt_lower);

        Ok(EngineeringExpertAnswer {
            module_id: ENGINEERING_EXPERT_MODULE_ID.to_string(),
            source_path: self.path.display().to_string(),
            compressed_bytes: self.compressed_bytes(),
            matched_sections,
            safety_bounded,
            safety_notice: safety_notice(safety_bounded),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct EngineeringKnowledgeSection {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct EngineeringExpertAnswer {
    pub module_id: String,
    pub source_path: String,
    pub compressed_bytes: usize,
    pub matched_sections: Vec<EngineeringKnowledgeSection>,
    pub safety_bounded: bool,
    pub safety_notice: String,
}

impl EngineeringExpertAnswer {
    pub fn to_kernel_summary(&self) -> String {
        let mut output = format!(
            "{module_id} mmap-loaded from {source_path} ({bytes} compressed bytes); matched_sections={count}",
            module_id = self.module_id,
            source_path = self.source_path,
            bytes = self.compressed_bytes,
            count = self.matched_sections.len(),
        );

        if self.safety_bounded {
            output.push_str("; safety_bounded=true");
        }

        output.push('\n');
        output.push_str(&self.safety_notice);

        for section in &self.matched_sections {
            output.push_str("\n\n## ");
            output.push_str(&section.title);
            output.push('\n');
            output.push_str(&section.content);
        }

        output
    }
}

/// Kernel_03 entry point: load the extension via mmap, query it, and drop it.
pub fn run_engineering_expert_module(prompt: &str) -> Result<String> {
    let module = EngineeringExpertModule::load_default()?;
    let answer = module.query(prompt)?;
    let summary = answer.to_kernel_summary();

    // Explicitly release the read-only mmap as soon as Kernel_03 finishes this
    // engineering request.
    std::mem::drop(module);

    Ok(summary)
}

fn parse_tcz_sections(text: &str) -> Result<Vec<EngineeringKnowledgeSection>> {
    let mut sections = Vec::new();
    let mut current_id = String::new();
    let mut current_title = String::new();
    let mut current_tags = Vec::<String>::new();
    let mut current_content = Vec::<String>::new();
    let mut in_section = false;

    for line in text.lines() {
        if let Some(value) = line.strip_prefix("@@section ") {
            if in_section {
                sections.push(EngineeringKnowledgeSection {
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
                sections.push(EngineeringKnowledgeSection {
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
        sections.push(EngineeringKnowledgeSection {
            id: current_id,
            title: current_title,
            tags: current_tags,
            content: current_content.join("\n"),
        });
    }

    if sections.is_empty() {
        Err(VortexAtomsError::InvalidKnowledgeExtension(
            "engineering expert module contains no TCZ sections".to_string(),
        ))
    } else {
        Ok(sections)
    }
}

fn score_section(prompt_lower: &str, section: &EngineeringKnowledgeSection) -> usize {
    let mut score = 0usize;

    for tag in &section.tags {
        if prompt_lower.contains(tag) {
            score = score.saturating_add(8);
        }
    }

    for token in section.title.split_whitespace() {
        if prompt_lower.contains(&token.to_ascii_lowercase()) {
            score = score.saturating_add(3);
        }
    }

    for token in prompt_lower.split_whitespace() {
        if section.content.to_ascii_lowercase().contains(token) {
            score = score.saturating_add(1);
        }
    }

    score
}

fn contains_hazardous_chemistry_request(prompt_lower: &str) -> bool {
    [
        "cyanide",
        "aqua regia",
        "nitric acid",
        "hydrochloric",
        "leach",
        "gold extraction",
        "precious metal extraction",
        "chemical process",
        "dissolve gold",
    ]
    .iter()
    .any(|needle| prompt_lower.contains(needle))
}

fn contains_lithium_handling_request(prompt_lower: &str) -> bool {
    [
        "lithium",
        "battery recycling",
        "18650",
        "pouch cell",
        "battery pack",
        "black mass",
    ]
    .iter()
    .any(|needle| prompt_lower.contains(needle))
}

fn safety_notice(safety_bounded: bool) -> String {
    if safety_bounded {
        "Safety boundary: this module provides diagnostics, process-control logic, hazard recognition, and compliance-oriented design guidance. It intentionally does not provide operational recipes, reagent concentrations, reaction conditions, or hands-on instructions for dangerous chemical extraction or unsafe lithium-cell processing.".to_string()
    } else {
        "Safety boundary: engineering guidance is limited to lawful, low-voltage, repair, diagnostic, modeling, and compliance-oriented workflows.".to_string()
    }
}
