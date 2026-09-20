# 07 Release Integrity

// Copyright (c) 2026 Ahmad Mansour. All rights reserved.

## Public Release URL

- **Release:** https://github.com/ahmedamdahmed90-oss/vortex_atoms_ai/releases/tag/v0.3.0
- **Commit SHA:** c80ebf63b2bf150affd3f86a05d913553ceec86c
- **Tag:** v0.3.0

## Attached Artifacts (small files only)

| Artifact | Description | SHA-256 |
|----------|-------------|---------|
| SHA256_MANIFEST.txt | Checksums for all release artifacts | See file |
| SBOM.json | CycloneDX 1.4, 715 components | See file |
| provenance.json | Build provenance with SHA-256 digest | See file |
| SEC01_LANDING.md | Security landing report | See file |

## Binary Distribution

Executables and installers ship via the website, NOT as GitHub release assets.
This keeps the repo lightweight (<100 MB tracked).

### Verification Steps

1. Download binary from website
2. Download SHA256_MANIFEST.txt from release page
3. Verify: `Get-FileHash -Algorithm SHA256 vortex_api.exe`
4. Compare with manifest value

## Supply Chain

- **SBOM:** 715 components, CycloneDX 1.4 format
- **Provenance:** SLSA build provenance with SHA-256 digest
- **Checksums:** SHA-256 sidecars for all release artifacts
- **CI:** `cargo audit` + `npm audit` on every push
- **Reproducibility:** Knowledge artifacts verified in CI (`git diff --exit-code`)
