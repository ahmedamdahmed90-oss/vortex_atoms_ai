#!/usr/bin/env python3
"""Compile the Vortex Atoms AI biomedical source into compact TCZ form.

Also emits the embedded avian-genetics fastpath manifest
(knowledge/avian_genetics_fastpath.json) consumed by src/fastpath at Rust
compile time. Pass --no-fastpath to skip the manifest.
"""
import argparse
import json
import os
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "knowledge" / "bio_medical_module.source.md"
OUT = ROOT / "knowledge" / "bio_medical_module.tcz"
FASTPATH_OUT = ROOT / "knowledge" / "avian_genetics_fastpath.json"
MAX_BYTES = 50 * 1024 * 1024
FASTPATH_MAX_BYTES = 1 * 1024 * 1024
FASTPATH_SCHEMA = "vortex_atoms_ai_avian_fastpath"
FASTPATH_VERSION = 1

SECTIONS = [
    (
        "human_general_medicine",
        "Human General Medicine Reasoning Map",
        "human,medicine,general medicine,clinical,differential diagnosis,red flag,pathophysiology,patient,symptom,treatment",
        "Systems framework spans cardiovascular, respiratory, renal, hepatic, endocrine, neurologic, gastrointestinal, hematologic, infectious disease, dermatologic, musculoskeletal, psychiatric, reproductive, and immunologic reasoning. Clinical logic uses problem representation, differential ranking, red-flag recognition, pre-test probability, sensitivity and specificity, likelihood ratios, contraindication checks, follow-up thresholds, and escalation logic. Emergency warning classes include chest pain, stroke signs, severe dyspnea, anaphylaxis, sepsis signs, suicidal ideation, severe dehydration, altered mental status, major trauma, and rapidly worsening symptoms. This is educational support, not diagnosis or treatment.",
    ),
    (
        "clinical_pharmacology",
        "Clinical Pharmacology and Medication Safety Map",
        "clinical pharmacology,pharmacokinetics,pharmacodynamics,drug,medication,dose,interaction,cyp,renal,hepatic,contraindication",
        "Pharmacokinetics: absorption, distribution, metabolism, elimination, half-life, steady state, loading concepts, renal impairment, hepatic impairment, protein binding, therapeutic index, and concentration-response uncertainty. Pharmacodynamics: agonism, antagonism, partial agonism, inverse agonism, receptor reserve, tolerance, tachyphylaxis, additive toxicity, synergistic toxicity, and pharmacogenomic variability. Medication safety map includes allergies, contraindications, pregnancy and lactation constraints, age extremes, QT prolongation risk, serotonin toxicity risk, CNS depression stacking, anticoagulation bleeding risk, nephrotoxicity, hepatotoxicity, and cytochrome P450 interactions. The module does not generate patient-specific dosing.",
    ),
    (
        "veterinary_avian_science",
        "Veterinary Science and Avian Medicine Database",
        "veterinary,avian,bird,budgie,finch,exotic,husbandry,triage,egg binding,respiratory,quarantine",
        "Species-aware veterinary reasoning tracks metabolic rate, thermoregulation, stress physiology, analgesic tolerance, fluid needs, handling risk, and species-specific contraindications. Avian medicine logic emphasizes that birds mask illness; urgent flags include open-mouth breathing, tail bobbing, fluffed posture, anorexia, weight loss, neurologic signs, trauma, egg-binding signs, bleeding, collapse, or heat stress. Husbandry variables include diet, calcium and vitamin D balance, UV exposure, ventilation, enclosure hygiene, enrichment, quarantine, parasite monitoring, and disease transmission controls. Veterinary outputs are educational and require qualified veterinary confirmation for sick animals.",
    ),
    (
        "avian_genetics_engine",
        "Avian Genetics Probability Engine",
        "avian genetics,bird genetics,inheritance,probability,punnett,sex-linked,autosomal recessive,autosomal dominant,mutation,carrier,split",
        "Core symbols: autosomal dominant A/a, autosomal recessive r/r, incomplete or co-dominant I/i with dosage effects, sex-linked recessive Z^m/Z or Z^m/W because male birds are ZZ and female birds are ZW, polygenic modifiers Pn, and epistatic masks E. Probability algorithm: enumerate gametes and allele frequencies, combine Punnett products, apply sex chromosome rules, evaluate penetrance, apply epistasis and masking order, then map genotype to phenotype. Autosomal recessive split x split gives 25% visual, 50% split, 25% normal. Visual recessive x normal gives 100% split. Visual recessive x split gives 50% visual and 50% split. Sex-linked recessive visual male x normal female gives all sons split and all daughters visual. Normal male split for a sex-linked mutation x normal female gives 50% split sons and 50% visual daughters. Females cannot be split for Z-linked recessive traits because they have only one Z chromosome.",
    ),
    (
        "budgie_mutation_database",
        "Budgie Mutation Database: Hagoromo Blackwing Opaline Rainbow",
        "budgie,hagoromo,blackwing,opaline,rainbow,clearwing,greywing,yellowface,blue series,sex-linked,frill,crest",
        "Hagoromo is represented as a Japanese frill phenotype with crest and frill expression; model it as an autosomal crest/frill complex with variable expressivity, breeder-line modifiers, viability scoring, feather-quality scoring, and line-breeding risk. Blackwing is represented as a wing melanin distribution phenotype; model it as a line-specific modifier interacting with normal, clearwing, and greywing-like wing dilution pathways, tracking wing-body contrast, melanin retention, and uncertainty. Opaline is a classic sex-linked recessive budgie mutation: males are visual when both Z chromosomes carry opaline; females are visual when their single Z carries opaline; split males transmit opaline without showing it; females cannot be split. Rainbow is a composite phenotype assembly: blue-series base plus opaline plus clearwing or fullbody greywing family plus yellowface or goldenface expression depending on standard. Rainbow probability is not a single locus; evaluate base color first, then sex-linked opaline, then wing dilution family, then yellowface or goldenface, then violet, dark, grey, spangle, dominant pied, or recessive pied modifiers.",
    ),
    (
        "finch_genetics_database",
        "Finch and Small-Bird Mutation Genetics Database",
        "finch,zebra finch,gouldian finch,canary,mutation,sex-linked,autosomal,head color,breast color,body color",
        "Finch mutation logic is species-namespaced because zebra finches, society finches, Gouldian finches, canaries, and other passerines can use different inheritance models for similar visual names. Zebra finch models track sex-linked traits with ZZ/ZW logic and autosomal recessive traits with carrier tracking; orange, brown, black breast, cheek, and dilution phenotypes can combine with modifiers and selection lines. Gouldian finch models separate head color, breast color, and body color loci before phenotype naming because sex-linked, autosomal, and modifier interactions may coexist. Breeding ethics engine flags high inbreeding coefficients, lethal-factor risk, deformity risk, poor feathering, immune weakness, and pairings that amplify welfare problems.",
    ),
    (
        "avian_probability_recipes",
        "Bird Mutation Probability Recipes and Cross Templates",
        "bird mutation probability,budgie probability,finch probability,genotype,phenotype,cross template,inheritance logic",
        "Template autosomal recessive: N/split x N/split -> 25% normal non-carrier, 50% normal split, 25% visual. Visual x normal non-carrier -> 100% split. Visual x split -> 50% visual, 50% split. Visual x visual -> 100% visual. Template autosomal dominant: single-factor visual x normal -> 50% visual single-factor and 50% normal; double-factor visual x normal -> 100% visual single-factor if viable; single-factor x single-factor -> 25% normal, 50% single-factor, 25% double-factor. Template sex-linked recessive in birds: visual male x normal female -> sons split, daughters visual; split male x normal female -> 25% normal sons, 25% split sons, 25% normal daughters, 25% visual daughters; normal male x visual female -> sons split, daughters normal; visual male x visual female -> all visual. Composite phenotype template such as Rainbow: multiply probabilities of required component phenotypes after accounting for linkage assumptions and unavailable genotype uncertainty.",
    ),
]


def build_fastpath_manifest() -> dict:
    """Single source of truth for the avian-genetics fastpath tables.

    The deterministic, no-disk-I/O fastpath resolves ONLY canonical single-locus
    crosses; composite or ambiguous queries are routed to the full corpus by the
    Rust fallback. Tables mirror the published bird-breeding probability recipes.
    """
    autosomal_recessive = [
        {
            "id": "ar_split_x_split",
            "left": "split",
            "right": "split",
            "outcome": "25% normal non-carrier, 50% split, 25% visual",
        },
        {
            "id": "ar_visual_x_normal",
            "left": "visual",
            "right": "normal",
            "outcome": "100% split",
        },
        {
            "id": "ar_visual_x_split",
            "left": "visual",
            "right": "split",
            "outcome": "50% visual, 50% split",
        },
        {
            "id": "ar_visual_x_visual",
            "left": "visual",
            "right": "visual",
            "outcome": "100% visual",
        },
        {
            "id": "ar_normal_x_normal",
            "left": "normal",
            "right": "normal",
            "outcome": "100% normal non-carrier",
        },
        {
            "id": "ar_split_x_normal",
            "left": "split",
            "right": "normal",
            "outcome": "50% split, 50% normal non-carrier",
        },
    ]
    autosomal_dominant = [
        {
            "id": "ad_single_x_normal",
            "left": "single_factor",
            "right": "normal",
            "outcome": "50% single-factor visual, 50% normal",
        },
        {
            "id": "ad_double_x_normal",
            "left": "double_factor",
            "right": "normal",
            "outcome": "100% single-factor visual",
        },
        {
            "id": "ad_single_x_single",
            "left": "single_factor",
            "right": "single_factor",
            "outcome": "25% normal, 50% single-factor, 25% double-factor",
        },
        {
            "id": "ad_double_x_single",
            "left": "double_factor",
            "right": "single_factor",
            "outcome": "50% double-factor, 50% single-factor",
        },
        {
            "id": "ad_double_x_double",
            "left": "double_factor",
            "right": "double_factor",
            "outcome": "100% double-factor (subject to viability checks)",
        },
        {
            "id": "ad_normal_x_normal",
            "left": "normal",
            "right": "normal",
            "outcome": "100% normal",
        },
    ]
    sex_linked_recessive = [
        {
            "id": "sl_visual_male_x_normal_female",
            "left": "visual_male",
            "right": "normal_female",
            "outcome": "sons split, daughters visual",
        },
        {
            "id": "sl_split_male_x_normal_female",
            "left": "split_male",
            "right": "normal_female",
            "outcome": "25% normal son, 25% split son, 25% normal daughter, 25% visual daughter",
        },
        {
            "id": "sl_normal_male_x_visual_female",
            "left": "normal_male",
            "right": "visual_female",
            "outcome": "sons split, daughters normal",
        },
        {
            "id": "sl_split_male_x_visual_female",
            "left": "split_male",
            "right": "visual_female",
            "outcome": "25% visual male, 25% split male, 25% visual female, 25% normal female",
        },
        {
            "id": "sl_visual_male_x_visual_female",
            "left": "visual_male",
            "right": "visual_female",
            "outcome": "all visual (males and females)",
        },
        {
            "id": "sl_normal_male_x_normal_female",
            "left": "normal_male",
            "right": "normal_female",
            "outcome": "all normal",
        },
    ]
    mutations = [
        {
            "key": "opaline",
            "name": "Opaline",
            "species": ["budgie"],
            "mode": "sex_linked_recessive",
            "note": "Classic sex-linked recessive budgie mutation: males are ZZ and visual when both Z chromosomes carry opaline; females are ZW and visual when their single Z carries it; split males transmit without showing it; females cannot be split.",
        },
        {
            "key": "blackwing",
            "name": "Blackwing",
            "species": ["budgie"],
            "mode": "composite_phenotype",
            "note": "Wing melanin distribution acting as a line-specific modifier that interacts with normal, clearwing, and greywing-like wing dilution pathways. Not a single-locus fastpath answer.",
        },
        {
            "key": "hagoromo",
            "name": "Hagoromo",
            "species": ["budgie"],
            "mode": "composite_phenotype",
            "note": "Japanese frill crest/frill complex with variable expressivity and breeder-line modifiers. Not a single-locus fastpath answer.",
        },
        {
            "key": "rainbow",
            "name": "Rainbow",
            "species": ["budgie"],
            "mode": "composite_phenotype",
            "note": "Composite phenotype assembly: blue-series base plus sex-linked opaline plus a wing dilution family plus yellowface/goldenface expression, then violet/dark/grey and pattern modifiers. Evaluate loci in assembly order; not a single-locus fastpath answer.",
        },
        {
            "key": "clearwing",
            "name": "Clearwing",
            "species": ["budgie"],
            "mode": "composite_phenotype",
            "note": "Wing dilution family member acting as a phenotype modifier; evaluate with the full assembly order. Not a single-locus fastpath answer.",
        },
        {
            "key": "greywing",
            "name": "Greywing",
            "species": ["budgie"],
            "mode": "composite_phenotype",
            "note": "Wing dilution family member acting as a phenotype modifier; evaluate with the full assembly order. Not a single-locus fastpath answer.",
        },
    ]
    return {
        "schema": FASTPATH_SCHEMA,
        "version": FASTPATH_VERSION,
        "generated_by": "tools/compile_bio_medical_module.py --fastpath",
        "keywords_avian": [
            "avian",
            "bird",
            "budgie",
            "finch",
            "opaline",
            "hagoromo",
            "blackwing",
            "rainbow",
            "clearwing",
            "greywing",
            "yellowface",
            "sex-linked",
            "sex linked",
            "punnett",
            "split",
            "carrier",
        ],
        "keywords_probability": [
            "genetics",
            "inheritance",
            "probability",
            "punnett",
            "genotype",
            "phenotype",
            "allele",
            "offspring",
            "expected",
            "chance",
            "odds",
            "percent",
            "ratio",
        ],
        "mutations": mutations,
        "templates": {
            "autosomal_recessive": autosomal_recessive,
            "autosomal_dominant": autosomal_dominant,
            "sex_linked_recessive": sex_linked_recessive,
        },
    }


def emit_fastpath_manifest() -> None:
    manifest = build_fastpath_manifest()
    text = json.dumps(
        manifest, indent=1, ensure_ascii=True, separators=(",", ": "), sort_keys=False
    ) + "\n"
    encoded = text.encode("utf-8")
    if len(encoded) >= FASTPATH_MAX_BYTES:
        raise SystemExit(
            f"fastpath manifest exceeds {FASTPATH_MAX_BYTES} budget: {len(encoded)} bytes"
        )
    temp = FASTPATH_OUT.with_suffix(".json.tmp")
    temp.write_bytes(encoded)
    os.replace(temp, FASTPATH_OUT)
    parsed = json.loads(FASTPATH_OUT.read_text(encoding="utf-8"))
    if parsed["schema"] != FASTPATH_SCHEMA or parsed["version"] != FASTPATH_VERSION:
        raise SystemExit(f"fastpath manifest self-check failed for {FASTPATH_OUT}")
    print(
        f"compiled {FASTPATH_OUT} ({len(encoded)} bytes, schema={parsed['schema']} v{parsed['version']})"
    )


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Compile the biomedical TCZ and the avian-genetics fastpath manifest."
    )
    parser.add_argument(
        "--no-fastpath",
        action="store_true",
        help="do not regenerate knowledge/avian_genetics_fastpath.json",
    )
    args = parser.parse_args()

    compact = [
        "VAI_TCZ_V1 module=bio_medical_module identity=vortex_atoms_ai max_map_bytes=52428800 safety_bounded=true"
    ]
    for section_id, title, tags, content in SECTIONS:
        compact.extend(
            [
                f"@@section {section_id}",
                f"@@title {title}",
                f"@@tags {tags}",
                " ".join(content.split()),
                "@@end",
            ]
        )
    OUT.write_text("\n".join(compact) + "\n", encoding="utf-8")
    size = OUT.stat().st_size
    if size >= MAX_BYTES:
        raise SystemExit(f"compiled module exceeds 50 MiB mapping budget: {size} bytes")
    print(f"compiled {OUT} ({size} bytes, {size / MAX_BYTES:.6%} of 50 MiB budget) from {SRC}")
    if not args.no_fastpath:
        emit_fastpath_manifest()


if __name__ == "__main__":
    main()
