use memmap2::{Mmap, MmapOptions};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::{Path, PathBuf};

use crate::error::{Result, VortexAtomsError};
use crate::knowledge_orchestrator::KnowledgeFragmentDescriptor;

pub const GLOBAL_HUMANITIES_MODULE_ID: &str = "global_humanities_module";
pub const GLOBAL_HUMANITIES_MODULE_PATH: &str = "knowledge/global_humanities_module.bin";
pub const GLOBAL_HUMANITIES_MAX_MAPPED_BYTES: u64 = 50 * 1024 * 1024;
const MAGIC: &[u8; 8] = b"VAI_GH01";

/// Returns the default Kernel_02 registration descriptor for the mmap-backed
/// global humanities, language, history, geography, and GIS extension.
pub fn global_humanities_descriptor() -> KnowledgeFragmentDescriptor {
    KnowledgeFragmentDescriptor::new(
        GLOBAL_HUMANITIES_MODULE_ID,
        "code_logic",
        "global humanities grammar language translation dictionary world languages history timeline geography gis mapping coordinates projection spatial index",
        GLOBAL_HUMANITIES_MODULE_PATH,
    )
}

/// Memory-mapped high-density binary humanities extension.
///
/// The compiled binary uses compact length-prefixed sections instead of JSON or
/// TOML. It is mapped read-only, parsed directly from the mapped byte slice, and
/// explicitly dropped after a Kernel_03 request.
pub struct GlobalHumanitiesModule {
    path: PathBuf,
    mmap: Mmap,
}

impl GlobalHumanitiesModule {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let metadata = file.metadata()?;

        if metadata.len() == 0 {
            return Err(VortexAtomsError::InvalidKnowledgeExtension(format!(
                "global humanities module is empty: {}",
                path.display()
            )));
        }

        if metadata.len() > GLOBAL_HUMANITIES_MAX_MAPPED_BYTES {
            return Err(VortexAtomsError::InvalidKnowledgeExtension(format!(
                "global humanities module exceeds mapping budget: {} bytes at {}",
                metadata.len(),
                path.display()
            )));
        }

        // SAFETY:
        // - File is opened read-only and mapped as immutable `Mmap`.
        // - File size is bounded before mapping.
        // - The binary parser performs checked length reads before slicing.
        // - The mmap is owned by this struct and released when dropped.
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        Ok(Self { path, mmap })
    }

    pub fn load_default() -> Result<Self> {
        Self::load(GLOBAL_HUMANITIES_MODULE_PATH)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn mapped_bytes(&self) -> usize {
        self.mmap.len()
    }

    pub fn sections(&self) -> Result<Vec<GlobalHumanitiesSection>> {
        parse_binary_sections(&self.mmap[..])
    }

    pub fn query(&self, prompt: &str) -> Result<GlobalHumanitiesAnswer> {
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
            .take(7)
            .map(|(_, section)| section)
            .collect::<Vec<_>>();

        Ok(GlobalHumanitiesAnswer {
            module_id: GLOBAL_HUMANITIES_MODULE_ID.to_string(),
            source_path: self.path.display().to_string(),
            mapped_bytes: self.mapped_bytes(),
            mapped_under_budget: self.mapped_bytes() as u64 <= GLOBAL_HUMANITIES_MAX_MAPPED_BYTES,
            matched_sections,
            operational_notice: operational_notice(),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct GlobalHumanitiesSection {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct GlobalHumanitiesAnswer {
    pub module_id: String,
    pub source_path: String,
    pub mapped_bytes: usize,
    pub mapped_under_budget: bool,
    pub matched_sections: Vec<GlobalHumanitiesSection>,
    pub operational_notice: String,
}

impl GlobalHumanitiesAnswer {
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
        output.push_str(&self.operational_notice);

        for section in &self.matched_sections {
            output.push_str("\n\n## ");
            output.push_str(&section.title);
            output.push('\n');
            output.push_str(&section.content);
        }

        output
    }
}

/// Kernel_03 entry point: mmap, query, then explicitly drop the mapping.
pub fn run_global_humanities_module(prompt: &str) -> Result<String> {
    let module = GlobalHumanitiesModule::load_default()?;
    let answer = module.query(prompt)?;
    let summary = answer.to_kernel_summary();

    // Explicit release point for the read-only mmap.
    std::mem::drop(module);

    Ok(summary)
}

fn parse_binary_sections(bytes: &[u8]) -> Result<Vec<GlobalHumanitiesSection>> {
    let mut cursor = Cursor::new(bytes);
    let magic = cursor.read_exact(MAGIC.len())?;
    if magic != &MAGIC[..] {
        return Err(VortexAtomsError::InvalidKnowledgeExtension(
            "global humanities module has invalid binary magic".to_string(),
        ));
    }

    let section_count = cursor.read_u32_le()? as usize;
    let mut sections = Vec::with_capacity(section_count);

    for _ in 0..section_count {
        let id = cursor.read_string_u16()?;
        let title = cursor.read_string_u16()?;
        let tag_count = cursor.read_u16_le()? as usize;
        let mut tags = Vec::with_capacity(tag_count);
        for _ in 0..tag_count {
            tags.push(cursor.read_string_u16()?.to_ascii_lowercase());
        }
        let content = cursor.read_string_u32()?;
        sections.push(GlobalHumanitiesSection {
            id,
            title,
            tags,
            content,
        });
    }

    if sections.is_empty() {
        Err(VortexAtomsError::InvalidKnowledgeExtension(
            "global humanities module contains no binary sections".to_string(),
        ))
    } else {
        Ok(sections)
    }
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(VortexAtomsError::Overflow(
                "global humanities binary cursor",
            ))?;
        let slice = self.bytes.get(self.offset..end).ok_or_else(|| {
            VortexAtomsError::InvalidKnowledgeExtension(format!(
                "global humanities binary section truncated at offset {} len {}",
                self.offset, len
            ))
        })?;
        self.offset = end;
        Ok(slice)
    }

    fn read_u16_le(&mut self) -> Result<u16> {
        let bytes = self.read_exact(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32_le(&mut self) -> Result<u32> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_string_u16(&mut self) -> Result<String> {
        let len = self.read_u16_le()? as usize;
        self.read_string(len)
    }

    fn read_string_u32(&mut self) -> Result<String> {
        let len = self.read_u32_le()? as usize;
        self.read_string(len)
    }

    fn read_string(&mut self, len: usize) -> Result<String> {
        let bytes = self.read_exact(len)?;
        std::str::from_utf8(bytes)
            .map(|value| value.to_string())
            .map_err(|error| {
                VortexAtomsError::InvalidKnowledgeExtension(format!(
                    "global humanities binary UTF-8 decode failed: {error}"
                ))
            })
    }
}

fn score_section(prompt_lower: &str, section: &GlobalHumanitiesSection) -> usize {
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

fn operational_notice() -> String {
    "Operational boundary: this module provides language, grammar, translation, history, geography, and GIS reference logic for strategic reasoning. Real-time production translation and GIS operations require validated locale data, current geodata, licensing review, and domain-specific QA.".to_string()
}
