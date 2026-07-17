# Fraud Operations Cost Model: Why RiskFabric Exists

RiskFabric is a personal engineering project built to demonstrate this approach end to end — from synthetic data generation through model training to an analyst-facing case management tool — not a deployed commercial product.

## The Problem That Keeps Fraud Teams Awake

Every Indian financial institution runs the same math every morning and loses.

A fraud detection system flags 1,500 transactions. A team of analysts investigates every single one — pulling data from four different systems, cross-referencing device fingerprints against location history, checking whether a ₹5,000 UPI transfer at 3 AM is suspicious or just a college student ordering late-night food. After 30 minutes per alert, they close 92% of them as legitimate. The remaining 8% — roughly 120 transactions — are actual fraud.

The team just burned ₹45 lakh worth of analyst salary this month investigating innocent customers. The real fraud that slipped through? That costs another ₹4.64 for every rupee lost, once you add chargebacks, legal fees, regulatory penalties, and the customers who leave because their legitimate transaction got blocked.

This is not a hypothetical. It is the operational reality of fraud detection in Indian banking, and it is the problem RiskFabric was built to solve.

## The Three-Layered Cost Problem

### Layer 1: The False Positive Tax

Industry consensus — [repeated across AML compliance literature since 2018](https://www.flagright.com/post/understanding-false-positives-in-transaction-monitoring) — puts transaction monitoring false positive rates at **90–95%**. For every 100 alerts a bank investigates, 90 to 95 are legitimate customer activity. The investigation still happens. The analyst still spends 30 minutes. The salary still gets paid.

For a mid-sized Indian bank processing 1 million transactions per day:

- 1,500 alerts generated daily at a 0.15% alert rate
- 1,380 of those are false positives
- 750 analyst-hours wasted every single day
- 94 analysts needed just to keep up with the backlog
- **₹5.4 crore per year in analyst salary** — 92% of which investigates nothing

**Source caveat**: The 90–95% figure is widely cited but traces through a chain (commonly attributed to PwC, circa 2017-18) whose original publication remains unverified. Treat as industry directional consensus, not an audited PwC statistic.

### Layer 2: The Investigation Time Sink

The per-alert cost isn't just labor hours — it's the compounding effect of scattered data. A [published case study from Parkar](https://www.parkar.in/insights/blogs/when-fraud-monitoring-becomes-manual-triage) (March 2026) describes an unnamed mid-sized regional bank where average investigation time stretched to **48 hours per alert**, not because cases were complex, but because the data needed to resolve each alert sat across four different systems. Twelve analysts could not keep up with 900 alerts per day.

[Unit21, a fraud detection vendor](https://www.unit21.ai/blog/ai-agents-for-fraud-detection-and-investigation-how-they-work-and-what-to-evaluate), reports that teams deploying AI-assisted investigation tools see investigation times drop from **30+ minutes to under 5 minutes per alert**, with 40–60% reductions in false positives. Unit21's own post caveats these numbers: "verify against your own data, not just a vendor's marketing page." The directional signal is consistent across multiple independent sources — investigation time is dominated by data gathering, not decision-making, and structured case management with model explainability cuts through it.

**Source caveat**: Unit21 numbers are vendor-reported and self-caveated. The Parkar case study is real but does not specify the bank's location — do not cite as Indian-specific without independent confirmation.

### Layer 3: The ₹4.64 Multiplier

[LexisNexis Risk Solutions commissioned Forrester Consulting](https://risk.lexisnexis.com/global/en/about-us/press-room/press-release/20240429-tcof-india) to survey 382 fraud management decision-makers across APAC (79 in India) for their 2023 True Cost of Fraud Study. The finding: **Indian financial institutions incur ₹4.64 in total costs for every ₹1 lost to fraud**. This includes internal labor, external investigation costs, legal fees, recovery costs, and the expense of replacing or redistributing lost assets. For retailers, the multiplier is ₹3.07.

In 2024, [cyber fraud cost Indians **₹22,845 crore**](https://www.angelone.in/news/market-updates/cyber-fraud-costs-indians-22-845-crore-in-2024-government) — a 206% increase from 2023, disclosed by the Ministry of Home Affairs to the Lok Sabha. At the ₹4.64 multiplier, the total operational burden on Indian financial institutions from fraud-related activity is orders of magnitude larger than the stolen amount alone.

Every rupee of fraud that gets through a detection system costs ₹4.64 to clean up. But here is the trap: over-investing in detection to catch that one rupee generates a flood of false positives that each cost ₹200 to investigate. The system burns money at both ends — missed fraud and false alerts.

### Verified Sources

| Claim | Source | Status |
|---|---|---|
| ₹4.64 fraud cost multiplier for Indian FIs | [LexisNexis True Cost of Fraud Study, APAC](https://risk.lexisnexis.com/global/en/about-us/press-room/press-release/20240429-tcof-india) (Forrester-commissioned, Aug 2023, 79 Indian respondents) | Solid |
| ₹22,845 crore cyber fraud losses, India 2024 | [Ministry of Home Affairs, Lok Sabha disclosure](https://www.angelone.in/news/market-updates/cyber-fraud-costs-indians-22-845-crore-in-2024-government) | Solid |
| 90–95% false positive rate | [Widely cited since ~2018](https://www.flagright.com/post/understanding-false-positives-in-transaction-monitoring), original PwC chain unverified | Industry directional consensus |
| 30 min → 5 min investigation time drop | [Unit21 vendor blog, 2025](https://www.unit21.ai/blog/ai-agents-for-fraud-detection-and-investigation-how-they-work-and-what-to-evaluate) (self-caveated) | Vendor-reported |
| 900 alerts/day, 12 analysts, 48-hour investigation | [Parkar blog, March 2026](https://www.parkar.in/insights/blogs/when-fraud-monitoring-becomes-manual-triage) (unnamed regional bank) | Real case, not confirmed Indian |
| ₹444,300 avg fraud analyst salary, India | [WorldSalaries 2026](https://worldsalaries.com/average-fraud-analyst-salary-in-india/) (corroborated by Indeed, PayScale, AmbitionBox) | Solid |

## The Indian-Specific Amplifier: Scale and Compliance

### UPI Makes This Existential

UPI processed [228 billion transactions worth ₹300 trillion in 2025](https://bureau.id/resources/blog/fraud-detection-software-for-fintech). A 0.15% alert rate on 228 billion transactions is 342 million alerts per year. At a 92% false positive rate, 314 million of those are wasted investigations. Even at an optimistic ₹150 cost per alert, that is ₹4,700 crore burned across the ecosystem each year investigating legitimate transactions. A 1% reduction in the false positive rate across UPI alone saves approximately ₹50 crore annually — just on analyst labor.

### The Regulatory Timeline Is Not Waiting

In [July 2024, the RBI issued revised Master Directions](https://hyperverge.co/blog/what-is-financial-fraud-detection/) requiring banks, NBFCs, and cooperative banks to strengthen fraud detection governance with data analytics, early warning systems, and enhanced due diligence. Compliance is not optional.

Simultaneously, the Digital Personal Data Protection Act 2023 restricts how financial institutions can share and process real customer data. Banks cannot pool transaction records to build better fraud models. They cannot send customer PII to third-party fraud vendors without explicit consent frameworks that do not yet exist at scale.

This creates a compliance paradox: the regulator demands better fraud detection, but the law restricts the data needed to build it.

## How RiskFabric Breaks the Cycle

RiskFabric is a synthetic data sandbox. It generates realistic financial transaction streams — complete with customers, accounts, cards, merchant geolocation, device fingerprints, and temporal patterns — and injects known fraud patterns with known ground truth. The output trains XGBoost models that catch fraud on behavioral signals (spatial velocity, device-switching, rapid-fire transactions, escalating amounts) rather than brittle rules. Every component of the pipeline addresses a specific cost driver.

### Cost Driver 1: False Positive Rate (92% → Target 75-80%)

The root cause of false positives is models trained on noisy operational labels and thresholds set by intuition rather than evidence.

**What RiskFabric does differently:**

The generator (`src/generators/fraud.rs`) maintains two labels per transaction: `fraud_target` (ground truth — the fraud we actually injected) and `is_fraud` (operational label — what a rules engine would flag, including false positives and missed fraud). This separation means the model learns from clean truth, not from a system that is wrong 92% of the time. Training on operational labels creates models that replicate the same mistakes. Training on ground truth creates models that improve on them.

The ML pipeline calibrates every score via Platt scaling (`src/ml/calibrate_model.py`) so a score of 0.85 actually means 85% probability of fraud. Thresholds are tuned against precision-recall curves (`src/ml/compute_thresholds.py`) rather than arbitrary cutoffs. SHAP explanations (`src/ml/shap_analysis.py`) surface which features drive false positives, enabling iterative model refinement. Drift detection (`src/ml/drift_simulation.py`) catches distribution shifts before they silently degrade model quality in production.

### Cost Driver 2: Investigation Time (30 min → Under 5 min)

The Parkar case study identified the real bottleneck: data scattered across four systems. Analysts spend 90% of their investigation time gathering context, not making decisions.

**What RiskFabric does differently:**

Every scored transaction lands in a structured Django case management UI (`case_admin/`) with the score, SHAP feature-level explanation, and full transaction context. An analyst sees not just "transaction flagged" but "flagged because the device switched from a Xiaomi in Mumbai to an iPhone in Gurugram within 12 minutes, the amount is 4.2× the customer's 90-day moving average, and the merchant is 47 km from the customer's home H3 cell." The decision becomes a review, not a scavenger hunt.

A Grafana operational dashboard gives fraud ops managers aggregate visibility into false positive rates, score distributions, and model health — so they detect alert quality degradation before the analyst team wastes a week on it.

### Cost Driver 3: Fraud Catch Rate (Behavioral Signals Beyond Rules)

Rules-based systems catch known patterns and miss everything else. LexisNexis identified synthetic identities at account creation as the highest-loss stage of the customer journey for APAC financial institutions (46% of FI fraud losses). UPI scams, CNP fraud, and account takeovers exploit velocity and behavioral anomalies that static rules cannot capture.

**What RiskFabric does differently:**

The generator injects fraud via behavioral profiles (`src/generators/fraud.rs`): spatial velocity anomalies, rapid-fire transaction sequences, device-switching patterns, escalating amounts, and coordinated campaigns spanning multiple customer accounts. The feature engineering pipeline (`src/etl/features/`) pre-builds these behavioral signals — merchant proximity via H3 hexagonal indexing, temporal velocity, device/IP switching, amount deviation from customer baselines. The resulting XGBoost model detects patterns that rules miss.

Leakage verification (`src/ml/verify_leakage.py`) ensures the model's performance is real, not inflated by features that accidentally encode the target variable. A model that looks good in evaluation because of data leakage fails silently in production — generating false positives, missing real fraud, and compounding the ₹4.64 multiplier effect on every missed case.

### Cost Driver 4: The Compliance Paradox

Indian banks cannot pool real customer data to build better models. They cannot send PII to external vendors without DPDP Act consent frameworks. They must comply with RBI data analytics requirements.

**What RiskFabric does differently:**

The entire pipeline operates on synthetic data. Banks can benchmark fraud model performance, tune detection thresholds, and stress-test at UPI scale without exposing a single real customer record. The deployment architecture (`documentation/decisions/deployment_architecture.md`) runs entirely self-hosted on a single EC2 instance with Docker Compose — no data leaves the bank's infrastructure.

This converts the compliance paradox into a compliance advantage: the regulator's mandate for better analytics is satisfied with a system that never needed real PII in the first place.

## The Economic Argument

The numbers below are illustrative — they show what industry-typical improvements would be worth at this scale, not measured results from RiskFabric itself, which uses synthetic data and has not been deployed at a real institution.

For a mid-sized Indian financial institution processing 1 million transactions per day:

| Line Item | Current State (Rules) | With RiskFabric ML Pipeline |
|---|---|---|
| False positive rate | ~92% | 75–80% (targeted) |
| Investigation time per alert | ~30 min | ~5 min (SHAP + case management) |
| Analysts needed | ~94 | ~7 |
| Annual analyst salary consumed by FPs | ~₹5.4 crore | ~₹1.5–3 crore |
| Fraud catch rate improvement over rules | Baseline | +15–25% (behavioral features) |
| Avoided fraud losses (₹4.64 multiplier) | — | ₹4.64 saved per ₹1 of additional fraud caught |
| **Annual operational savings** | — | **₹3–5 crore** |

Every 1% reduction in false positive rate saves approximately ₹60 lakh per year in wasted analyst salary for this institution.

The infrastructure cost is a one-time EC2 deployment (documented in the deployment architecture decision). Ongoing costs are compute for periodic model retraining and inference — a fraction of the annual analyst savings.

At UPI ecosystem scale, even a half-percent improvement in false positive rates across participating institutions saves hundreds of crores annually.

## What This Analysis Is Not

This is a business case framework built from the best available independently verifiable data, with all caveats explicitly stated. It is not audited financial analysis. The cost model uses a mid-sized bank scenario; actual savings depend on transaction volume, existing fraud infrastructure, analyst headcount, and the quality of current rules-based detection systems.

Three sources carry caveats that must travel with any use of this document: the 90–95% false positive rate is industry directional consensus, not a verified original publication; the Unit21 investigation-time benchmarks are vendor-reported and self-caveated; the Parkar case study describes an unnamed bank of unspecified geography. All other claims trace to verifiable sources: LexisNexis (Forrester-commissioned survey with disclosed methodology), Ministry of Home Affairs (Lok Sabha disclosure), and WorldSalaries (corroborated across Indeed, PayScale, AmbitionBox).
