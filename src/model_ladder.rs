//! Model ladder: an ordered, hardware-aware hierarchy of model variants.
//!
//! Where `route_model_hint` only fires on an *explicit* client hint, the
//! ladder layers an automatic selection on top: given the detected CPU tier
//! and the incoming prompt, it chooses the highest rung the hardware can
//! afford within the configured context budget — without ever requiring the
//! client to name a model.
//!
//! Security posture is inherited from the existing routing gate and stays
//! **fail-closed**: when `security.allow_model_routing` is disabled the
//! ladder is inert (`select_rung` returns `None`, the caller keeps the
//! currently-loaded engine), and every auto-selection is audit-logged by
//! the caller just like a manual swap would be.

use crate::llm_download::{ModelDefinition, MODELS_1_5B, MODELS_3B, MODEL_ECO, MODEL_Q4_0};
use crate::perf_topology::CpuTier;

/// A single rung on the model ladder.
#[derive(Clone, Copy)]
pub struct LadderRung {
    /// Stable short id (`eco`, `q4_0`, `b1_5`, `b3`).
    pub id: &'static str,
    /// Human label shown in `/v1/models`.
    pub label: &'static str,
    /// Pointer into the download allowlist / matrix.
    pub def: &'static ModelDefinition,
    /// Highest CPU tier this rung may be auto-selected on. A `Low`-tier host
    /// is never laddered past the economy slot regardless of prompt size.
    pub max_tier: CpuTier,
    /// Minimum prompt length (in UTF-16 chars) before this rung becomes the
    /// preferred choice. Below the threshold a cheaper rung wins for latency.
    pub min_prompt_chars: usize,
}

/// Ordered ladder, cheapest rung first. Walk it in reverse (largest → smallest)
/// when auto-selecting so the largest affordable slot wins on long prompts.
pub static LADDER: &[LadderRung] = &[
    LadderRung {
        id: "eco",
        label: "SmolLM2-135M economy (Q4_0)",
        def: &MODEL_ECO,
        max_tier: CpuTier::Standard,
        min_prompt_chars: 0,
    },
    LadderRung {
        id: "q4_0",
        label: "Qwen2.5-0.5B quality (Q4_0)",
        def: &MODEL_Q4_0,
        max_tier: CpuTier::Standard,
        min_prompt_chars: 300,
    },
    LadderRung {
        id: "b1_5",
        label: "Qwen2.5-1.5B (Q4_K_M)",
        def: &MODELS_1_5B[0],
        max_tier: CpuTier::Standard,
        min_prompt_chars: 1500,
    },
    LadderRung {
        id: "b3",
        label: "Qwen2.5-3B (Q4_K_M)",
        def: &MODELS_3B[0],
        max_tier: CpuTier::Standard,
        min_prompt_chars: 3200,
    },
];

/// Resolve a rung by stable id. `None` for unknown names (fail-closed: the
/// caller keeps whatever model is currently loaded).
pub fn resolve_rung(id: &str) -> Option<&'static LadderRung> {
    LADDER.iter().find(|r| r.id == id)
}

/// Map an existing economy/quality matrix name (`eco`/`q4_0`) onto its ladder
/// rung. Unknown names yield `None` (the existing route-model semantics keep
/// returning a 400 from the caller).
pub fn rung_for_matrix_name(name: &str) -> Option<&'static LadderRung> {
    match name {
        "eco" => resolve_rung("eco"),
        "q4_0" => resolve_rung("q4_0"),
        _ => None,
    }
}

/// Auto-select the best rung for this host + prompt, or `None`.
///
/// Rules (deterministic, fail-closed):
/// 1. Routing disabled → always `None` (engine keeps its current model). This
///    mirrors the manual hint gate: no routing without `security.allow_model_routing`.
/// 2. `prompt_chars > context_ceiling` → `None` (cannot fit within the policy
///    context budget; caller rejects instead of silently truncating).
/// 3. `CpuTier::Low` host → pinned to the economy rung regardless of prompt
///    length (weak hardware cannot afford the larger matrix entries on CPU).
/// 4. Otherwise walk the ladder largest → smallest; the first rung whose
///    `min_prompt_chars <= prompt_chars` and whose `max_tier >= tier` wins.
///
/// Callers must audit-log every non-`None` auto-selection (same surface as a
/// manual swap) so model changes never happen silently.
pub fn select_rung(
    tier: CpuTier,
    prompt_chars: usize,
    routing_enabled: bool,
    context_ceiling: usize,
) -> Option<&'static LadderRung> {
    if !routing_enabled || prompt_chars > context_ceiling {
        return None;
    }
    if tier == CpuTier::Low {
        return resolve_rung("eco");
    }
    LADDER
        .iter()
        .rev()
        .find(|r| prompt_chars >= r.min_prompt_chars && tier >= r.max_tier)
}

/// Cheap UTF-16 code-unit count, the conservative proxy the ladder uses for
/// "prompt size". Iterates `encode_utf16` (no allocation, no grapheme cost).
pub fn prompt_chars(prompt: &str) -> usize {
    prompt.encode_utf16().count()
}

/// Serializable snapshot of the ladder for `/v1/models`.
#[derive(serde::Serialize)]
pub struct LadderRungJson {
    pub id: &'static str,
    pub label: &'static str,
    pub repo: &'static str,
    pub file: &'static str,
    pub architecture: &'static str,
    pub size_params: u64,
    pub quant: &'static str,
    pub max_seq_len: usize,
    pub min_prompt_chars: usize,
}

impl From<&'static LadderRung> for LadderRungJson {
    fn from(r: &'static LadderRung) -> Self {
        Self {
            id: r.id,
            label: r.label,
            repo: r.def.repo,
            file: r.def.gguf_file,
            architecture: r.def.arch,
            size_params: r.def.size_params,
            quant: r.def.quant,
            max_seq_len: r.def.max_seq_len,
            min_prompt_chars: r.min_prompt_chars,
        }
    }
}

/// JSON view of the whole ladder. Empty when routing is disabled (fail-closed
/// visibility: clients must not be told laddered slots exist if they cannot
/// be reached).
pub fn ladder_json(routing_enabled: bool) -> Vec<LadderRungJson> {
    if !routing_enabled {
        return Vec::new();
    }
    LADDER.iter().map(LadderRungJson::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ladder_is_ordered_smallest_to_largest() {
        assert_eq!(LADDER.len(), 4, "expected 4 rungs: eco/q4_0/b1_5/b3");
        let mut prev = LadderRung {
            id: "",
            label: "",
            def: &MODEL_ECO,
            max_tier: CpuTier::Low,
            min_prompt_chars: 0,
        };
        for r in LADDER {
            assert!(
                r.min_prompt_chars >= prev.min_prompt_chars,
                "ladder must be ascending by min_prompt_chars"
            );
            prev = *r;
        }
        assert_eq!(LADDER[0].id, "eco");
        assert_eq!(LADDER[3].id, "b3");
    }

    #[test]
    fn resolve_known_and_unknown() {
        assert_eq!(resolve_rung("eco").map(|r| r.id), Some("eco"));
        assert_eq!(resolve_rung("b3").map(|r| r.id), Some("b3"));
        assert_eq!(resolve_rung("nope").map(|r| r.id), None);
    }

    #[test]
    fn matrix_names_resolve_to_rungs() {
        assert_eq!(rung_for_matrix_name("eco").map(|r| r.id), Some("eco"));
        assert_eq!(rung_for_matrix_name("q4_0").map(|r| r.id), Some("q4_0"));
        assert_eq!(rung_for_matrix_name("bogus").map(|r| r.id), None);
    }

    #[test]
    fn routing_disabled_is_fail_closed() {
        assert_eq!(
            select_rung(CpuTier::Standard, 9_000, false, 4096).map(|r| r.id),
            None
        );
    }

    #[test]
    fn oversize_prompt_is_rejected() {
        assert_eq!(
            select_rung(CpuTier::Standard, 10_000, true, 4096).map(|r| r.id),
            None
        );
    }

    #[test]
    fn short_prompt_prefers_eco() {
        assert_eq!(
            select_rung(CpuTier::Standard, 42, true, 4096).map(|r| r.id),
            Some("eco")
        );
    }

    #[test]
    fn long_prompt_climbs_ladder() {
        assert_eq!(
            select_rung(CpuTier::Standard, 1700, true, 4096).map(|r| r.id),
            Some("b1_5")
        );
        assert_eq!(
            select_rung(CpuTier::Standard, 4000, true, 8192).map(|r| r.id),
            Some("b3")
        );
    }

    #[test]
    fn low_tier_is_pinned_to_eco() {
        assert_eq!(
            select_rung(CpuTier::Low, 7_000, true, 8192).map(|r| r.id),
            Some("eco")
        );
    }

    #[test]
    fn ladder_json_is_empty_when_routing_disabled() {
        assert!(ladder_json(false).is_empty());
        assert_eq!(ladder_json(true).len(), LADDER.len());
    }
}
