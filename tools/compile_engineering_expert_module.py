#!/usr/bin/env python3
"""Compile the Vortex Atoms AI engineering expert source into compact TCZ form."""
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "knowledge" / "engineering_expert_module.source.md"
OUT = ROOT / "knowledge" / "engineering_expert_module.tcz"

SECTIONS = [
    (
        "electronics_repair",
        "Electronics Repair Expert System",
        "electronics,repair,smps,diagnostics,current limited supply,rail,ripple,mosfet,diode,capacitor",
        "Fault isolation hierarchy: reproduce the symptom, inspect mechanically, model the power path, validate rails, validate clock/reset, validate buses, localize thermally, substitute known-good assemblies, apply corrective action, and regression test. Low-voltage bench workflow uses current limiting, ESD controls, expected idle-current envelopes, isolation for mains-side equipment, and qualified procedures for hazardous primary circuits. SMPS reasoning covers input protection, rectification/PFC, startup bias, PWM control, primary switch behavior, transformer coupling, secondary rectification, optocoupler feedback, compensation stability, transient response, and protection latch states. Measurement heuristics include ripple trend, ESR instability, diode leakage, MOSFET gate anomalies, thermal differential localization, absent oscillators, brownout reset loops, and connector intermittency.",
    ),
    (
        "pcb_diagnostics",
        "PCB Diagnostics and Rework Intelligence",
        "pcb,printed circuit board,diagnostics,bga,via,short,open,corrosion,rework,diode mode,xray,ir",
        "Board-level taxonomy: opens, shorts, leakage, impedance drift, solder fatigue, conductive anodic filament growth, dendrites, cracked MLCCs, via barrel fracture, BGA head-in-pillow, corrosion under packages, delamination, and heat-damaged dielectric. Diagnostic algorithm: segment by power domain, build a net criticality map, compare diode-mode signatures to a golden board, inspect by microscope/X-ray/IR, validate rail sequencing, probe reset and clock, isolate downstream loads, and document rework risk. Rework quality model includes thermal mass estimation, moisture sensitivity, flux compatibility, pad integrity, controlled preheat, post-rework cleaning validation, ionic contamination risk, conformal-coating repair, and acceptance testing.",
    ),
    (
        "lithium_recycling_algorithms",
        "Lithium Battery Recycling Algorithms",
        "lithium,battery recycling,18650,pouch cell,bms,state of health,soh,sorting,black mass,quarantine",
        "Decision algorithms support safe sorting, state estimation, pack traceability, transport classification, and certified recycler handoff. Sorting model: chemistry identification, pack history, voltage envelope, impedance trend, swelling or venting flag, thermal anomaly flag, BMS telemetry, recall status, and damage classification. State-of-health fusion combines open-circuit voltage, coulomb-count history, DC internal resistance, temperature response, cycle count, self-discharge rate, and Bayesian confidence bands to route packs toward reuse, second-life evaluation, or recycling. Risk controls flag swelling, electrolyte odor, heat, puncture, saltwater exposure, unknown chemistry, or unsafe transport thresholds for hazardous-material escalation.",
    ),
    (
        "metal_detector_schematics",
        "Metal Detector Schematics and Signal Models",
        "metal detector,schematic,vlf,pulse induction,pi,bfo,coil,ground balance,dsp,oscillator",
        "Educational low-voltage architectures: VLF induction balance, pulse induction, beat-frequency oscillator, synchronous demodulation, low-noise preamplification, band-pass filtering, ground balance, ADC sampling, and target classification DSP. VLF block model: transmit oscillator to coil driver to balanced receive coil to instrumentation preamp to phase detector to low-pass filter to MCU classifier to audio/display output. PI block model: controlled pulse driver to search coil to flyback clamp to recovery blanking to decay sampler to integrator to baseline tracker to target response classifier. Design constraints include coil Q, shielding, cable capacitance, ferrite and soil mineralization, EMI rejection, battery noise, low-voltage power safety, and regulatory emissions limits.",
    ),
    (
        "precious_metal_process_safety",
        "Precious Metal Recovery and Chemical Process Safety",
        "precious metal,gold extraction,silver,palladium,chemical process,acid,cyanide,aqua regia,refiner,assay,xrf,e waste",
        "Non-operational process intelligence for e-waste recovery decisions, hazard analysis, assay planning, compliance, and certified-refiner handoff. The module intentionally omits reagent recipes, concentrations, reaction conditions, and extraction steps. Process map: material identification, depopulation decision, mechanical separation, assay/XRF screening, chain-of-custody, hazardous constituent inventory, permitted processing route selection, waste-treatment accountability, and refiner settlement reconciliation. Hazards to recognize include strong acids, oxidizers, cyanide systems, toxic metal salts, chlorine or nitrogen oxide gases, uncontrolled heat evolution, incompatible waste streams, and contaminated residues. Safer decision algorithm estimates recoverable value, subtracts labor/compliance/waste costs, evaluates exposure risk, prefers mechanical separation and certified downstream refining, and rejects informal chemical processing when controls are absent.",
    ),
]


def main() -> None:
    compact = [
        "VAI_TCZ_V1 module=engineering_expert_module identity=vortex_atoms_ai safety_bounded=true"
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
    print(f"compiled {OUT} ({OUT.stat().st_size} bytes) from {SRC}")


if __name__ == "__main__":
    main()
