# engineering_expert_module

Safety boundary: This engineering extension supports lawful diagnostics, repair, modeling, process-control design, recycling triage, and compliance-oriented analysis. It does not contain operational recipes, reagent concentrations, reaction conditions, or hands-on instructions for dangerous chemical extraction or unsafe lithium-cell processing.

## Electronics repair expertise
- Fault isolation hierarchy: reproduce, inspect, power-path model, rail validation, clock/reset validation, bus validation, thermal localization, known-good substitution, corrective action, regression test.
- Low-voltage bench workflow: current-limited supply, expected idle current envelope, injection only after short localization, ESD controls, isolation transformer for mains-side equipment, and no live primary probing without qualified procedures.
- SMPS diagnostic model: input protection, rectification/PFC, startup bias, PWM controller, primary switch, transformer, secondary rectification, optocoupler feedback, compensation network, load transient behavior, and protection latch states.
- Measurement heuristics: rail ripple trend, ESR-driven instability, diode leakage signatures, MOSFET gate charge anomalies, thermal differential localization, oscillator absence, brownout reset loops, intermittent connector fretting.

## PCB diagnostics expertise
- Board-level fault taxonomy: opens, shorts, leakage, impedance drift, solder fatigue, CAF, dendrites, cracked MLCCs, via barrel fracture, BGA head-in-pillow, corrosion under packages, delamination, and heat-damaged dielectric.
- Diagnostic algorithm: segment by power domain, build net criticality map, compare diode-mode signatures to golden board, inspect via X-ray/IR/microscope, validate rail sequencing, probe reset/clock, isolate loads, and document rework risk.
- Rework quality model: thermal mass estimation, moisture sensitivity, flux compatibility, pad integrity, controlled preheat, post-rework cleaning validation, ionic contamination risk, conformal-coating repair, and acceptance tests.

## Lithium battery recycling algorithms
- Scope: decision algorithms for safe sorting, state estimation, pack traceability, transport classification, and recycler handoff. Physical processing requires certified industrial controls.
- Sorting model: chemistry identification, pack history, voltage envelope, impedance trend, swelling/venting flag, thermal anomaly flag, BMS telemetry, recall status, and damage classification.
- State-of-health algorithm: combine open-circuit voltage, coulomb-count history, DCIR, temperature response, cycle count, self-discharge rate, and Bayesian confidence bands to choose reuse, second-life, or recycling routing.
- Risk controls: quarantine flags for swelling, electrolyte odor, heat, physical puncture, saltwater exposure, unknown chemistry, or cells below safe transport thresholds; escalation to hazardous-material professionals.

## Metal detector schematic expertise
- Educational low-voltage architectures: VLF induction balance, pulse induction, beat-frequency oscillator, synchronous demodulation, low-noise preamplifier, band-pass filtering, ground balance, ADC sampling, and target classification DSP.
- VLF block model: transmit oscillator -> coil driver -> balanced receive coil -> instrumentation preamp -> phase detector -> low-pass filter -> MCU classifier -> audio/display output.
- PI block model: controlled pulse driver -> search coil -> flyback clamp -> recovery blanking -> decay sampler -> integrator -> baseline tracker -> target response classifier.
- Design constraints: coil Q, shielding, cable capacitance, ferrite/soil mineralization, EMI rejection, battery noise, safe low-voltage power design, and regulatory emissions limits.

## Precious metal recovery and chemical process safety
- Scope: non-operational process intelligence for e-waste recovery decisions, hazard analysis, assay planning, compliance, and certified-refiner handoff. No reagent recipes, concentrations, temperatures, or extraction steps are provided.
- Process map: material identification, depopulation decision, mechanical separation, assay/XRF screening, chain-of-custody, hazardous constituent inventory, permitted processing route selection, waste treatment accountability, and refiner settlement reconciliation.
- Chemistry hazards to recognize: strong acids, oxidizers, cyanide systems, toxic metal salts, chlorine/nitrogen oxide gases, uncontrolled heat evolution, incompatible waste streams, and contaminated residues.
- Safer decision algorithm: estimate recoverable value, subtract labor/compliance/waste costs, evaluate exposure risk, prefer mechanical separation and certified downstream refining, reject informal chemical processing when controls are absent.
