use memmap2::{Mmap, MmapOptions};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::{Path, PathBuf};

use crate::error::{Result, VortexAtomsError};
use crate::knowledge_orchestrator::KnowledgeFragmentDescriptor;

pub const BIO_MEDICAL_MODULE_ID: &str = "bio_medical_module";
pub const BIO_MEDICAL_MODULE_PATH: &str = "knowledge/bio_medical_module.tcz";
pub const BIO_MEDICAL_MAX_MAPPED_BYTES: u64 = 50 * 1024 * 1024;

/// Returns the default Kernel_02 registration descriptor for the mmap-backed
/// biomedical and avian-genetics extension.
pub fn bio_medical_descriptor() -> KnowledgeFragmentDescriptor {
    KnowledgeFragmentDescriptor::new(
        BIO_MEDICAL_MODULE_ID,
        "code_logic",
        "human general medicine clinical pharmacology veterinary science avian genetics budgie finch inheritance probability hagoromo blackwing opaline rainbow mutation",
        BIO_MEDICAL_MODULE_PATH,
    )
}

/// Memory-mapped biomedical and avian genetics extension.
///
/// The compiled TCZ file is bounded to less than 50 MiB before mapping. It is
/// opened read-only, queried for a single Kernel_03 request, and explicitly
/// dropped after use.
pub struct BioMedicalModule {
    path: PathBuf,
    mmap: Mmap,
}

impl BioMedicalModule {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let metadata = file.metadata()?;

        if metadata.len() == 0 {
            return Err(VortexAtomsError::InvalidKnowledgeExtension(format!(
                "bio medical module is empty: {}",
                path.display()
            )));
        }

        if metadata.len() > BIO_MEDICAL_MAX_MAPPED_BYTES {
            return Err(VortexAtomsError::InvalidKnowledgeExtension(format!(
                "bio medical module exceeds 50 MiB mapping budget: {} bytes at {}",
                metadata.len(),
                path.display()
            )));
        }

        // SAFETY:
        // - Opened read-only and mapped as immutable `Mmap`.
        // - File size is bounded below 50 MiB before mapping.
        // - The parser validates UTF-8 before semantic access.
        // - The mmap is owned by this struct and released when the struct drops.
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        Ok(Self { path, mmap })
    }

    pub fn load_default() -> Result<Self> {
        Self::load(BIO_MEDICAL_MODULE_PATH)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn mapped_bytes(&self) -> usize {
        self.mmap.len()
    }

    pub fn sections(&self) -> Result<Vec<BioMedicalKnowledgeSection>> {
        let text = std::str::from_utf8(&self.mmap[..]).map_err(|error| {
            VortexAtomsError::InvalidKnowledgeExtension(format!(
                "{} is not valid UTF-8 TCZ text: {error}",
                self.path.display()
            ))
        })?;

        parse_tcz_sections(text)
    }

    pub fn query(&self, prompt: &str) -> Result<BioMedicalAnswer> {
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
            .take(5)
            .map(|(_, section)| section)
            .collect::<Vec<_>>();

        let clinical_safety_bounded = contains_clinical_request(&prompt_lower);
        let veterinary_safety_bounded = contains_veterinary_request(&prompt_lower);

        Ok(BioMedicalAnswer {
            module_id: BIO_MEDICAL_MODULE_ID.to_string(),
            source_path: self.path.display().to_string(),
            mapped_bytes: self.mapped_bytes(),
            mapped_under_50mb: self.mapped_bytes() as u64 <= BIO_MEDICAL_MAX_MAPPED_BYTES,
            matched_sections,
            clinical_safety_bounded,
            veterinary_safety_bounded,
            safety_notice: safety_notice(clinical_safety_bounded, veterinary_safety_bounded),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct BioMedicalKnowledgeSection {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct BioMedicalAnswer {
    pub module_id: String,
    pub source_path: String,
    pub mapped_bytes: usize,
    pub mapped_under_50mb: bool,
    pub matched_sections: Vec<BioMedicalKnowledgeSection>,
    pub clinical_safety_bounded: bool,
    pub veterinary_safety_bounded: bool,
    pub safety_notice: String,
}

impl BioMedicalAnswer {
    pub fn to_kernel_summary(&self) -> String {
        let mut output = format!(
            "{module_id} mmap-loaded from {source_path} ({bytes} mapped bytes; under_50mb={under}); matched_sections={count}",
            module_id = self.module_id,
            source_path = self.source_path,
            bytes = self.mapped_bytes,
            under = self.mapped_under_50mb,
            count = self.matched_sections.len(),
        );

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
pub fn run_bio_medical_module(prompt: &str) -> Result<String> {
    let module = BioMedicalModule::load_default()?;
    let answer = module.query(prompt)?;
    let summary = answer.to_kernel_summary();

    // Explicitly release the read-only mmap as soon as Kernel_03 finishes this
    // biomedical or avian-genetics request.
    std::mem::drop(module);

    Ok(summary)
}

fn parse_tcz_sections(text: &str) -> Result<Vec<BioMedicalKnowledgeSection>> {
    let mut sections = Vec::new();
    let mut current_id = String::new();
    let mut current_title = String::new();
    let mut current_tags = Vec::<String>::new();
    let mut current_content = Vec::<String>::new();
    let mut in_section = false;

    for line in text.lines() {
        if let Some(value) = line.strip_prefix("@@section ") {
            if in_section {
                sections.push(BioMedicalKnowledgeSection {
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
                sections.push(BioMedicalKnowledgeSection {
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
        sections.push(BioMedicalKnowledgeSection {
            id: current_id,
            title: current_title,
            tags: current_tags,
            content: current_content.join("\n"),
        });
    }

    if sections.is_empty() {
        Err(VortexAtomsError::InvalidKnowledgeExtension(
            "bio medical module contains no TCZ sections".to_string(),
        ))
    } else {
        Ok(sections)
    }
}

fn score_section(prompt_lower: &str, section: &BioMedicalKnowledgeSection) -> usize {
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

fn contains_clinical_request(prompt_lower: &str) -> bool {
    [
        "medicine",
        "clinical",
        "pharmacology",
        "drug",
        "dose",
        "diagnosis",
        "symptom",
        "patient",
        "human",
        "treatment",
    ]
    .iter()
    .any(|needle| prompt_lower.contains(needle))
}

fn contains_veterinary_request(prompt_lower: &str) -> bool {
    [
        "veterinary",
        "avian",
        "bird",
        "budgie",
        "finch",
        "mutation",
        "genetics",
        "inheritance",
        "probability",
    ]
    .iter()
    .any(|needle| prompt_lower.contains(needle))
}

fn safety_notice(clinical: bool, veterinary: bool) -> String {
    match (clinical, veterinary) {
        (true, true) => "Safety boundary: this module provides educational biomedical, pharmacology, veterinary, and avian-genetics decision support. It does not diagnose, prescribe, replace licensed clinicians/veterinarians, or provide patient-specific dosing. Urgent human or animal symptoms require qualified professional care.".to_string(),
        (true, false) => "Safety boundary: human medicine and pharmacology content is educational decision support only. It does not diagnose, prescribe, or provide patient-specific treatment or dosing.".to_string(),
        (false, true) => "Safety boundary: veterinary and avian-genetics content is educational breeding, triage, and husbandry support only. Sick or injured animals require a qualified veterinarian.".to_string(),
        (false, false) => "Safety boundary: module output is educational biomedical and genetics reference material only.".to_string(),
    }
}
