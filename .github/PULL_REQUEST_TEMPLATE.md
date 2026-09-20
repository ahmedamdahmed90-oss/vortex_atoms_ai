---
name: Pull Request
about: Submit a pull request to Vortex Atoms AI
title: ''
labels: needs-review
assignees: ''

---

## Description
<!-- Provide a brief description of the changes in this PR -->

Related Issue: #(issue number)

## Type of Change
<!-- Mark relevant options with [x] -->
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Code refactoring
- [ ] Test addition/update

## Changes Made
<!-- List the main changes made in this PR -->
1. Change 1
2. Change 2
3. Change 3

## CI Gate Checklist
<!-- ALL items must pass before merge. Paste CI run URL. -->

### Backend
- [ ] `cargo fmt -- --check` clean
- [ ] `cargo clippy --all-targets --features learner,tray -- -D warnings` clean
- [ ] `cargo test --features learner --lib` — 156+ passed, 0 failed
- [ ] `cargo test --features learner,tray` — passed

### Frontend
- [ ] `npm run lint:strict` — 0 errors, 0 warnings
- [ ] `npm run test` — 356+ passed
- [ ] `npm run build` — success

### Data Room
- [ ] `check_facts_fresh.ps1` — green
- [ ] `check_no_manual_numbers.ps1` — green
- [ ] `check_repo_refs.ps1` — green

### Documentation
- [ ] README updated (if user-facing changes)
- [ ] API_REFERENCE.md updated (if API changes)
- [ ] CHANGELOG entry added

## Screenshots (if applicable)
<!-- Add screenshots to demonstrate the changes -->

## Breaking Changes
<!-- List any breaking changes and migration steps -->
None

## Additional Notes
<!-- Any additional information that reviewers should know -->
