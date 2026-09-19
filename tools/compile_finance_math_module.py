#!/usr/bin/env python3
"""Compile the Vortex Atoms AI finance/math source into compact TCZ form."""
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "knowledge" / "finance_math_module.source.md"
OUT = ROOT / "knowledge" / "finance_math_module.tcz"
MAX_BYTES = 50 * 1024 * 1024

SECTIONS = [
    (
        "macro_models",
        "Advanced Macroeconomic Models",
        "macro,macroeconomics,gdp,inflation,labor,monetary policy,yield curve,fiscal,exchange rate,scenario,stagflation,output gap",
        "National accounts include GDP by expenditure, income, and production; inflation decomposition; labor-market slack; fiscal impulse; current account; capital flows; productivity; and output gap. Monetary transmission tracks policy-rate path, yield curve, bank-credit channel, expectations channel, exchange-rate channel, liquidity preference, term premium, real rate, and financial conditions index. Macro scenario engine supports baseline, upside, downside, stagflation, hard landing, soft landing, supply shock, demand shock, debt sustainability, external funding stress, and currency shock.",
    ),
    (
        "micro_market_design",
        "Microeconomic and Market Design Models",
        "micro,microeconomics,elasticity,marginal cost,game theory,nash,stackelberg,bertrand,cournot,auction,market power",
        "Consumer and producer theory covers elasticity, substitution, income effects, marginal cost, marginal revenue, contribution margin, economies of scale, learning curves, and price discrimination. Game-theory layer includes Nash equilibrium, Stackelberg competition, Bertrand and Cournot structures, repeated games, adverse selection, moral hazard, mechanism design, auction logic, and incentive compatibility. Industrial-organization model tracks market power, entry barriers, switching costs, network effects, platform economics, vertical integration, supplier bargaining power, and regulatory constraints.",
    ),
    (
        "corporate_strategy_finance",
        "Corporate Finance and Strategic Planning Engine",
        "corporate finance,strategy,npv,irr,wacc,valuation,free cash flow,working capital,scenario planning,kpi,capital allocation",
        "Capital allocation model includes NPV, IRR, payback, EVA, WACC, hurdle rates, scenario-weighted valuation, real options, capital rationing, portfolio optimization, and post-investment review. Financial-statement model maps revenue drivers, margin bridge, working capital, capex, depreciation, debt schedule, interest coverage, covenant headroom, free cash flow, and sensitivity tables. Strategic planning loop diagnoses position, sets constraints, models scenarios, ranks initiatives, stress tests, chooses portfolio, assigns owners, defines KPIs, and implements feedback cadence.",
    ),
    (
        "factory_workflow_logistics",
        "Corporate and Factory Workflow Logistics",
        "factory,operations,workflow,logistics,takt time,cycle time,throughput,wip,little's law,oee,bottleneck,supply chain,mrp,safety stock",
        "Operations models include takt time, cycle time, throughput, WIP, Little's Law, bottleneck analysis, OEE, downtime taxonomy, changeover reduction, SMED, line balancing, and capacity planning. Supply-chain engine uses demand forecasting, safety stock, reorder point, EOQ, MRP, supplier risk, lead-time variability, bullwhip effect, lot sizing, transport constraints, and service-level optimization. Factory workflow architecture includes value-stream mapping, constraint identification, dispatching rules, finite-capacity scheduling, kanban sizing, quality gates, rework-loop isolation, and maintenance planning.",
    ),
    (
        "risk_assessment_frameworks",
        "Risk Assessment Frameworks",
        "risk,risk assessment,enterprise risk,var,cvar,stress testing,monte carlo,bayesian,fmea,bow tie,expected loss,control effectiveness",
        "Enterprise risk taxonomy covers strategic, financial, operational, compliance, cyber, supplier, market, liquidity, credit, model, geopolitical, and climate risk. Quantitative risk layer includes probability-impact matrices, expected loss, VaR, CVaR, stress testing, scenario analysis, fault trees, bow-tie analysis, FMEA, Monte Carlo, Bayesian updating, and control effectiveness. Governance layer defines risk appetite, key risk indicators, escalation thresholds, control ownership, residual risk, audit trail, incident review, and management action tracking.",
    ),
    (
        "statistics_algorithms",
        "Statistical Algorithms and Forecasting",
        "statistics,statistical algorithms,regression,glm,time series,arima,kalman,bootstrap,hypothesis testing,forecasting,optimization,linear programming",
        "Statistical toolkit includes descriptive statistics, robust estimators, correlation, regression, GLM, time-series decomposition, ARIMA concepts, state-space models, Kalman filtering, survival analysis, bootstrapping, and hypothesis testing. Forecasting controls include backtesting, cross-validation, leakage control, forecast bias, prediction intervals, hierarchical forecasting, intermittent demand, ensemble averaging, and regime detection. Optimization toolkit includes linear programming, integer programming, stochastic programming, dynamic programming, queuing models, simulation optimization, gradient methods, and multi-objective Pareto tradeoffs.",
    ),
    (
        "data_driven_strategy_architecture",
        "Data-Driven Strategy and Vector DB Architecture",
        "data driven strategy,vector db,qdrant,semantic intent,token cache,mmap,leak prevention,decision log,model governance",
        "Data pipeline uses source validation, schema contracts, feature registry, quality checks, lineage, drift detection, versioned assumptions, model cards, scenario store, and decision logs. Active Vector DB interface maps semantic finance, economics, risk, logistics, and statistics intents to the finance_math_module descriptor; qdrant-compatible point snapshots preserve vector-layer compatibility; hot token cache serves repeated strategy fragments without disk reads. Leak-prevention design uses read-only mmap per request, bounded file size, no global mutable module singleton, supervisor token-cache eviction, and explicit mmap drop after Kernel_03 completion.",
    ),
]


def main() -> None:
    compact = [
        "VAI_TCZ_V1 module=finance_math_module identity=vortex_atoms_ai max_map_bytes=52428800 vector_db=active safety_bounded=true"
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


if __name__ == "__main__":
    main()
