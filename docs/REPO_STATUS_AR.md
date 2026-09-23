---
title: حالة المستودع
dir: rtl
lang: ar
---

# REPO-01 — حالة المستودع

> **حقوق النشر (c) 2026 أحمد منصور.** جميع الحقوق محفوظة.

**English:** [REPO_STATUS.md](./REPO_STATUS.md)

## ما هو هذا المستودع؟

هذا المستودع العام هو **مرآة إصدار (release mirror)** — يحتوي على لقطات مضغوطة (squashed snapshots) تبدأ من الإصدار v0.2.0. سجل التطوير الكامل خاص.

## لماذا يوجد commit (2) فقط؟

تم ضغط الدفع الأولي إلى commit (2) للحفاظ على سجل عام نظيف:

1. **Initial commit** — لقطة كاملة لقاعدة الشيفرة
2. **CI/CD fixes** — جميع تصحيحات خط CI

سيكون كل إصدار مستقبلي commit مضغوط واحد مع tag.

## كيف يوثّق CI الأرقام؟

كل دفع يُشغّل بوابة CI الكاملة:

- **Backend**: `cargo test` (225 tests)، `cargo clippy` (0 warnings)
- **Frontend**: `vitest` (356 tests)، `e2e` (54 tests)
- **Lint**: `oxlint` + `lint:strict` (0 errors)
- **Docker**: Multi-stage build passes

**Green CI = certified numbers.** تستشهد غرفة البيانات (data room) وREADME بقيم موثّقة فقط من CI.

## عملية الإصدار (Release Process)

1. تمرّ البوابات المحلية (local gates): lib 225 / vitest 356 / e2e 54 / lint:strict 0-0
2. يُنشأ tag (`git tag v0.3.0`)
3. `release.yml` يبني المخرجات + يشغّل `verify_release.ps1`
4. المخرجات الصغيرة (SBOM, provenance, manifest) تُرفق بإصدار GitHub
5. الملفات الثنائية (binaries) تُوزَّع عبر الموقع مع تحقق SHA-256

## التحقق (Verification)

```powershell
# Verify release integrity
./tools/verify_release.ps1 -Version "0.3.0"

# Verify facts are fresh
./tools/check_facts_fresh.ps1

# Verify no manual numbers
./tools/check_no_manual_numbers.ps1
```
