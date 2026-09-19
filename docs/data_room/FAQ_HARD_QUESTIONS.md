// Copyright (c) 2026 Ahmad Mansour — Vortex Atoms AI
# FAQ: Hard Questions for Investors

## Q1: "Why is generation slow on old CPUs?"

**AR:** 🖥️ **السرعة على المعالجات القديمة**
**EN:** 🖥️ **Why slow on old CPUs?**

**Answer (AR):**
آلة المرجعية لدينا من 2011 بدون AVX2. السرعة متوسطة لأن المعالج قديم. لكن SKU (sse41/avx1) تُحسّن الأداء. TK/s غير مُقاسة بعد لأن لا يوجد llama-cpp backend.

**Answer (EN):**
Our reference machine is from 2011 without AVX2. Generation speed is moderate because the CPU is old. But SKU compilation (sse41/avx1) optimizes performance. TK/s not yet measured (no llama-cpp backend).

**Proof:** {{performance_reference_machine_value}} | Fastpath: {{performance_fastpath_ms_value}}ms

---

## Q2: "Why not just use llama.cpp today?"

**AR:** ❓ **لماذا لا تستخدمون llama.cpp؟**
**EN:** ❓ **Why not llama.cpp?**

**Answer (AR):**
نحن نبني المحرك الخاص بنا (Candle) للتحكم الكامل في الاستدلال. llama.cpp خيار إضافي مستقبلي. حالياً Candle يوفر استدلال محلي بدون اعتماد خارجي.

**Answer (EN):**
We're building our own inference engine (Candle) for full control. llama.cpp is an optional future backend. Currently Candle provides local inference with no external dependencies.

**Proof:** Cargo.toml dependencies: candle-core, candle-nn, candle-transformers

---

## Q3: "What stops OpenAI from crushing you?"

**AR:** 🛡️ **ما الذي يمنع OpenAI من القضاء عليكم؟**
**EN:** 🛡️ **What stops OpenAI from crushing you?**

**Answer (AR):**
OpenAI يحتاج سحابة وإنترنت. نحن نعمل محلياً بدون اتصال. الخصوصية المحلية هي الميزة الأساسية — لا أحد يريد بياناته على سيرفرات شركة أمريكية.

**Answer (EN):**
OpenAI requires cloud and internet. We run locally, offline. Local data sovereignty is our core moat — nobody wants their data on US corporate servers.

**Proof:** {{security_posture_threats_addressed_value}} threats addressed locally

---

## Q4: "Revenue model?"

**AR:** 💰 **نموذج الإيرادات**
**EN:** 💰 **Revenue model?**

**Answer (AR):**
برنامج بيتا: 90 يوم مجاني + 40% خصم السنة الأولى. بعدها اشتراك شهري/سنوي. لا يوجد إيرادات حالياً — مرحلة قبل الإيرادات.

**Answer (EN):**
Beta program: 90 days free + 40% year-1 discount. Then monthly/annual subscription. No revenue yet — pre-revenue stage.

**Proof:** {{market_traction_pilot_program_terms_value}}

---

## Q5: "Why should we bet on a self-taught founder?"

**AR:** 👤 **لماذا نعتقد بمؤسس متعلم ذاتياً؟**
**EN:** 👤 **Why bet on a self-taught founder?**

**Answer (AR):**
بنيت كل شيء من الصفر: محرك Rust، واجهة React، 5 نوى. بدون شهادة CS — بدون أكاديميا. النتيجة: منتج يعمل 156 اختبار و0 خطأ clippy. التعلم الذاتي يعني المرونة.

**Answer (EN):**
Built everything from scratch: Rust engine, React UI, 5 kernels. No CS degree — no academia. Result: 156 tests pass, 0 clippy errors. Self-taught means adaptable.

**Proof:** {{test_suites_rust_lib_tests_value}}, {{test_suites_rust_clippy_value}}

---

## Q6: "What if pilots don't convert?"

**AR:** 🚪 **ماذا لو لم تحول المقاعد؟**
**EN:** 🚪 **What if pilots don't convert?**

**Answer (AR):**
المقاعد مفتوحة ولا شيء مُلزم. إذا لم يتحولوا، نستمر في التحسين ونبحث عن عملاء آخرين. لدينا 90 يوم مجاني — المخاطر منخفضة للمساهمين.

**Answer (EN):**
Slots are open, nothing committed. If pilots don't convert, we keep iterating and find other customers. 90-day free program means low risk for investors.

**Proof:** Pilot slots: OPEN | {{market_traction_pilot_program_terms_value}}

---

## Q7: "How does the 5-Kernel Matrix work?"

**AR:** 🧩 **كيف تعمل مصفوفة 5 النوى؟**
**EN:** 🧩 **How does the 5-Kernel Matrix work?**

**Answer (AR):**
5 أنماط مستقلة: K01 واجهة، K02 توجيه متجهي، K03 تنفيذ منطقي، K04 وسائط متعددة، K05 مراقبة ذاكرة. كل نمط يعمل بشكل مستقل ويتواصل عبر IKC (Inter-Kernel Communication).

**Answer (EN):**
5 independent kernels: K01 UI, K02 vector routing, K03 code logic, K04 multimedia, K05 memory watchdog. Each operates independently, communicating via IKC (Inter-Kernel Communication).

**Proof:** docs/data_room/02_TECHNICAL_DOSSIER.md

---

## Q8: "What about memory safety?"

**AR:** 🛡️ **السلامة الذاكرية**
**EN:** 🛡️ **What about memory safety?**

**Answer (AR):**
Rust يضمن السلامة الذاكرية بدون garbage collector. لا يوجد segfaults، لا يوجد data races. هذا ميزة تنافسية مقابل Python/Node.js.

**Answer (EN):**
Rust guarantees memory safety without garbage collector. No segfaults, no data races. Competitive advantage over Python/Node.js.

**Proof:** Rust language, cargo clippy --all-targets --features learner,tray -- -D warnings = 0 errors

---

## Q9: "What's the biggest risk?"**

**AR:** ⚠️ **ما هو أكبر خطر؟**
**EN:** ⚠️ **What's the biggest risk?**

**Answer (AR):**
مؤسس واحد بدون شريك. إذا مرضت أو احتجت راحة، المشروع يتوقف. التخفيف: تعيين شريك مع توزيع أسهم في أول 6 أشهر.

**Answer (EN):**
Single founder without co-founder. If I get sick or need rest, the project stops. Mitigation: hire co-founder with vesting in first 6 months.

**Proof:** docs/data_room/10_RISKS_AND_MITIGATIONS.md

---

## Q10: "What about competition from established AI companies?"

**AR:** 🏢 **المنافسة من الشركات الكبرى**
**EN:** 🏢 **Competition from established AI companies?**

**Answer (AR):**
OpenAI و Google يركزون على السحابة. نحن نركز على المحلي والخصوصية. سوقنا المستهدف هو المطورين العرب وفرق البيانات التي تريد التحكم الكامل في بياناتها.

**Answer (EN):**
OpenAI and Google focus on cloud. We focus on local and privacy. Our target market is Arabic developers and data teams who want full control over their data.

**Proof:** {{security_posture_threats_addressed_value}} security threats addressed locally

---

## Q11: "What's the timeline to revenue?"

**AR:** 📅 **جدول زمني للإيرادات**
**EN:** 📅 **Timeline to revenue?**

**Answer (AR):**
بيتا بعد 3-6 أشهر. إيرادات بعد 6-12 شهر مع برنامج اشتراك. 18 شهر للوهم للإيرادات المستدامة.

**Answer (EN):**
Beta in 3-6 months. Revenue in 6-12 months with subscription program. 18 months to sustainable revenue.

**Proof:** 18-month roadmap in docs/data_room/08_FOUNDER_AND_ROADMAP.md

---

## Q12: "How do you verify the software is secure?"

**AR:** 🔒 **كيف تتحققون من أمان البرنامج؟**
**EN:** 🔒 **How do you verify software security?**

**Answer (AR):**
156 اختبار أمان، clippy 0 أخطاء، SBOM مع 715 مكون، SHA-256 sidecars، verify_release.ps1. كل شيء مُوثق ومُتحقق.

**Answer (EN):**
156 security tests, clippy 0 errors, SBOM with 715 components, SHA-256 sidecars, verify_release.ps1. Everything documented and verified.

**Proof:** {{release_integrity_verify_release_verdict_value}}, {{release_integrity_sbom_components_value}} components

---

## Rehearsal Mode Summary

Each answer: ≤90 words + one number + one proof link.

| Q | Number | Proof Link |
|---|--------|------------|
| Q1 | {{performance_reference_machine_value}} | perf_report |
| Q2 | candle-core dependency | Cargo.toml |
| Q3 | {{security_posture_threats_addressed_value}} threats | SECURITY.md |
| Q4 | {{market_traction_pilot_program_terms_value}} | MARKET_REPORT.md |
| Q5 | {{test_suites_rust_lib_tests_value}} tests | README.md |
| Q6 | Pilot slots: OPEN | MARKET_REPORT.md |
| Q7 | 5 kernels | 02_TECHNICAL_DOSSIER.md |
| Q8 | Rust memory safety | cargo clippy 0 errors |
| Q9 | Single founder risk | 10_RISKS_AND_MITIGATIONS.md |
| Q10 | {{security_posture_threats_addressed_value}} threats | SECURITY.md |
| Q11 | 18-month roadmap | 08_FOUNDER_AND_ROADMAP.md |
| Q12 | {{release_integrity_verify_release_verdict_value}} | verify_release.ps1 |
