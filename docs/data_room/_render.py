#!/usr/bin/env python3
# Copyright (c) 2026 Ahmad Mansour. All rights reserved.
# Vortex Atoms AI — Data Room Generator
# Reads facts.json + templates → renders all 22 data room documents.
#
# Usage:
#   python _render.py <out_dir> <tag>
#
# Called by tools/build_data_room.ps1

import json
import os
import sys
from datetime import datetime, timezone

COPYRIGHT = "// Copyright (c) 2026 Ahmad Mansour. All rights reserved."


def load_facts(facts_path):
    with open(facts_path, "r", encoding="utf-8") as f:
        return json.load(f)


def render_index(f, tag, ts):
    tests = f["test_suites"]
    cov = f["coverage_quadruple"]
    sec = f["security_posture"]
    rel = f["release_integrity"]

    return f"""# Data Room Index

## Table of Contents

| # | Document | Section |
|---|----------|---------|
| 01 | Executive One-Pager | Executive Summary |
| 02 | Technical Dossier | 5-Kernel Matrix |
| 03 | Security Posture | Controls + Residual Risks |
| 04 | Performance Evidence | Bench Matrix + Honesty |
| 05 | Learner Evidence | Compliance + MRR@5 |
| 06 | Market & Traction | Pilot Slots (OPEN) |
| 07 | Release Integrity | SBOM/Provenance/Checksums |
| 08 | Founder & Roadmap | Self-Taught + 18-Month Plan |
| 09 | Ask & Use of Funds | $500K-$1M, 60/25/15 |
| 10 | Risks & Mitigations | Single-Founder + Pre-Revenue |

## How to Verify

1. Regenerate with `tools/build_data_room.ps1`
2. Verify SHA-256 manifest matches
3. Check facts.json is current: `tools/check_facts_fresh.ps1`
4. No manual numbers: `tools/check_no_manual_numbers.ps1`

## Facts Reference

- Test suites: {tests['rust_lib_tests']['value']}
- Coverage: {cov['statements']['value']}% stmts
- Security: {sec['threats_addressed']['value']} threats
- Release: SBOM {rel['sbom_components']['value']} components, {rel['cargo_lock_checksums']['value']} checksums

## NDA-Lite Note

Public subset: 01 + 03 (summary only). Full room: diligence stage only.

**Tag:** {tag}
**Generated:** {ts}
"""


def render_executive_one_pager(f, tag, ts):
    tests = f["test_suites"]
    cov = f["coverage_quadruple"]
    sec = f["security_posture"]
    rel = f["release_integrity"]

    return f"""# Executive One-Pager

## Problem

Developers need a locally-run AI assistant with full data sovereignty.

## Solution

**Vortex Atoms AI** is a Rust-native AI kernel with a React+TypeScript frontend.

## Key Numbers

| Metric | Value |
|--------|-------|
| Rust lib tests | {tests['rust_lib_tests']['value']} |
| Frontend tests | {tests['frontend_test']['value']} |
| Clippy errors | {tests['rust_clippy']['value']} |
| Coverage | {cov['statements']['value']}% stmts |
| Security threats | {sec['threats_addressed']['value']} addressed |
| Release verification | {tests['verify_release']['value']} |
| SBOM components | {rel['sbom_components']['value']} |

## The Ask

**$500K-$1M** for 18-month roadmap.

**Tag:** {tag}
**Generated:** {ts}
"""


def render_technical_dossier(f, tag, ts):
    perf = f["performance"]
    return f"""# Technical Dossier

## 5-Kernel Matrix Architecture

Vortex Atoms AI uses a 5-kernel matrix for AI inference:

| Kernel | Function | Latency |
|--------|----------|---------|
| Fastpath | Direct inference | {perf['fastpath_ms']['value']}ms |
| Cache | Hit path | {perf['cache_hit_ms']['value']}ms |
| SIMD | Vector operations | AVX + SSE4.2 + SSE4.1 |
| Compliant | Allowlist-only sources | N/A |
| Learner | Knowledge growth | MRR@5: {f['learner']['mrr5_hybrid']['value']} |

## Reference Machine

- **Spec:** {perf['reference_machine']['value']}
- **Host features:** {perf['host_features']['value']}
- **SKU baseline size:** {perf['sku_baseline_size_kb']['value']} KB
- **SKU AVX1 size:** {perf['sku_avx1_size_kb']['value']} KB

## Honesty Clause

All numbers traceable to README / LEARNER_REPORT / PERF_REPORT.

**Tag:** {tag}
**Generated:** {ts}
"""


def render_security_posture(f, tag, ts):
    sec = f["security_posture"]
    threats = [
        ("No auth on local API", "Bearer tokens"),
        ("Cross-origin exploitation", "Strict CORS + rate limiting"),
        ("Request flooding", "Per-route body caps"),
        ("Config tampering", "replace_cfg validation, 422"),
        ("Metrics leak", "/v1/metrics loopback/admin only"),
        ("Session pile-up", "Purge endpoint + retention sweep"),
        ("Disk exhaustion", f"{f['disk_guard_min_mb']['value']} MB disk guard"),
        ("Transport hijacking", "WS origin check + headers"),
        ("Kiosk abuse", "--read-only mode"),
        ("Multi-instance conflicts", "Named mutex + exit code 2"),
        ("Supply chain", "SBOM + provenance + SHA-256"),
    ]

    rows = "\n".join(f"| {t} | {c} | ✅ |" for t, c in threats)

    return f"""# Security Posture

## Controls Table

| Threat | Control | Status |
|--------|---------|--------|
{rows}

## Residual Risks

- **Non-Windows development**: DPAPI is Windows-only. On macOS/Linux, tokens stored in plaintext.
- **Authenticode signing**: Release binaries hashed but not Authenticode-signed.
- **TLS**: Optional --tls-cert/--tls-key for LAN use.

**Tag:** {tag}
**Generated:** {ts}
"""


def render_performance_evidence(f, tag, ts):
    perf = f["performance"]
    return f"""# Performance Evidence

## Bench Matrix

| Metric | Value |
|--------|-------|
| Fastpath latency | {perf['fastpath_ms']['value']}ms |
| Cache hit latency | {perf['cache_hit_ms']['value']}ms |
| Reference machine | {perf['reference_machine']['value']} |
| Host features | {perf['host_features']['value']} |
| SKU baseline size | {perf['sku_baseline_size_kb']['value']}KB |
| SKU AVX1 size | {perf['sku_avx1_size_kb']['value']}KB |
| Perf report date | {perf['perf_report_date']['value']} |

## Honesty

All performance numbers from PERF_REPORT.md, traceable to README.

**Tag:** {tag}
**Generated:** {ts}
"""


def render_learner_evidence(f, tag, ts):
    learner = f["learner"]
    return f"""# Learner Evidence

## Compliance

- Allowlist-only sources with license tracking
- Attribution requirements enforced
- No weight training, no fine-tuning
- Honest knowledge acquisition only

## MRR@5 Results

| Method | MRR@5 |
|--------|-------|
| Hybrid (vector + compliance) | {learner['mrr5_hybrid']['value']} |
| Vector only | {learner['mrr5_vector_only']['value']} |
| Delta | {learner['mrr5_delta']['value']} |

## Config

- learner.enabled: {str(learner['learner_enabled']['value']).lower()}

**Tag:** {tag}
**Generated:** {ts}
"""


def render_market_traction(f, tag, ts):
    market = f["market_traction"]
    return f"""# Market & Traction

## Pilot Status

| Slot | Status |
|------|--------|
| Pilot 001 | **{market['pilot_slots']['value']}** |
| Pilot 002 | **{market['pilot_slots']['value']}** |
| Pilot 003 | **{market['pilot_slots']['value']}** |

**No signed pilots exist.** All slots remain OPEN until signatures.

## Pilot Program Terms

{market['pilot_program_terms']['value']}

**Tag:** {tag}
**Generated:** {ts}
"""


def render_release_integrity(f, tag, ts):
    rel = f["release_integrity"]
    return f"""# Release Integrity

## SBOM

- **Components:** {rel['sbom_components']['value']}
- **Format:** CycloneDX 1.4
- **File:** SBOM.json

## Provenance

- **Attestation:** SLSA-style provenance
- **File:** provenance.json

## SHA-256 Sidecars

- **Count:** {rel['sha256_sidecars']['value']} files
- **Manifest:** SHA256_MANIFEST.txt

## Verification

- **Verify release:** {rel['verify_release_verdict']['value']}
- **Command:** `tools/verify_release.ps1 -ReleaseDir dist -Strict`

## Artifact Sizes

| Artifact | Size |
|----------|------|
| Portable ZIP | {rel['portable_zip_size_mb']['value']} MB |
| Setup EXE | {rel['setup_exe_size_mb']['value']} MB |
| API EXE | {rel['api_exe_size_mb']['value']} MB |
| Chat EXE | {rel['chat_exe_size_mb']['value']} MB |
| Dashboard EXE | {rel['dashboard_exe_size_mb']['value']} MB |

**Tag:** {tag}
**Generated:** {ts}
"""


def render_founder_and_roadmap(f, tag, ts):
    return f"""# Founder & Roadmap

## Founder

**Ahmad Mansour** — Self-taught developer, built Vortex Atoms AI from scratch.

## Roadmap (18 Months)

| Phase | Timeline | Milestone |
|-------|----------|-----------|
| Phase 1 | Months 1-6 | Core engine + first pilot |
| Phase 2 | Months 7-12 | Multi-kernel + enterprise features |
| Phase 3 | Months 13-18 | Scale + revenue |

## Key Differentiator

- Self-taught founder with deep technical expertise
- Arabic-first, RTL-native design
- Local-first, no cloud dependencies
- Compliance by design

**Tag:** {tag}
**Generated:** {ts}
"""


def render_ask_and_use_of_funds(f, tag, ts):
    return f"""# Ask & Use of Funds

## The Ask

**$500K-$1M** for 18-month roadmap.

## Use of Funds

| Category | Allocation |
|----------|------------|
| Engineering | 60% |
| Marketing & Sales | 25% |
| Operations | 15% |

## Expected Outcomes

- Ship v1.0 with full 5-kernel matrix
- Secure 3+ pilot customers
- Achieve $100K ARR
- Build team of 3-5 engineers

**Tag:** {tag}
**Generated:** {ts}
"""


def render_risks_and_mitigations(f, tag, ts):
    return f"""# Risks & Mitigations

## Key Risks

| Risk | Mitigation |
|------|------------|
| Single-founder bus factor | Open-source codebase, documented architecture |
| Pre-revenue | Pilot program to validate demand |
| Competition from big tech | Local-first, Arabic-first, compliance-by-design niche |
| CUDA dependency for GPU | CPU-only mode works, GPU optional |
| Windows-only DPAPI | Cross-platform storage fallback |

## Residual Risks

- Authenticode signing not yet implemented
- TLS optional (LAN use only)
- DPAPI Windows-only

**Tag:** {tag}
**Generated:** {ts}
"""


RENDERERS = {
    "00_INDEX": render_index,
    "01_EXECUTIVE_ONE_PAGER": render_executive_one_pager,
    "02_TECHNICAL_DOSSIER": render_technical_dossier,
    "03_SECURITY_POSTURE": render_security_posture,
    "04_PERFORMANCE_EVIDENCE": render_performance_evidence,
    "05_LEARNER_EVIDENCE": render_learner_evidence,
    "06_MARKET_TRACTION": render_market_traction,
    "07_RELEASE_INTEGRITY": render_release_integrity,
    "08_FOUNDER_AND_ROADMAP": render_founder_and_roadmap,
    "09_ASK_AND_USE_OF_FUNDS": render_ask_and_use_of_funds,
    "10_RISKS_AND_MITIGATIONS": render_risks_and_mitigations,
}


def main():
    if len(sys.argv) < 3:
        print("Usage: python _render.py <out_dir> <tag>")
        sys.exit(1)

    out_dir = sys.argv[1]
    tag = sys.argv[2]
    ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    script_dir = os.path.dirname(os.path.abspath(__file__))
    facts_path = os.path.join(script_dir, "facts.json")
    facts = load_facts(facts_path)

    os.makedirs(out_dir, exist_ok=True)

    count = 0
    for name, renderer in RENDERERS.items():
        content = renderer(facts, tag, ts)
        path = os.path.join(out_dir, f"{name}.md")
        with open(path, "w", encoding="utf-8") as f:
            f.write(content)
        count += 1

        en_name = f"{name}_EN"
        en_path = os.path.join(out_dir, f"{en_name}.md")
        with open(en_path, "w", encoding="utf-8") as f:
            f.write(content)
        count += 1

    print(f"Rendered {count} files to {out_dir}")


if __name__ == "__main__":
    main()
