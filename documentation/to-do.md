# Project Roadmap & Backlog

## Summary
The `to-do.md` document serves as the tactical roadmap for RiskFabric. It details the completed milestones and upcoming engineering tasks required to evolve the simulation from a prototype into a production-grade synthetic data platform.

## Design Intent
The roadmap is designed to be **Value-Driven**, with tasks prioritized by their impact on the fidelity and utility of the simulation. Maintaining a public backlog provides a signal regarding critical system gaps, such as geographic realism, adversarial diversity, or operational performance.

---

## 👥 Customer Generation
- [x] **Location Heuristic Fix**: `location_type` (Urban/Rural) is assigned based on city name or configuration fallback.
- [x] **Spatial Jittering**: Implementation of multi-level jittering, including a ~500m drift for residential nodes and a deterministic ~100m drift for transaction events.
- [x] **City Name Fallbacks**: Use of "{State} Region" for missing city names to maintain geographic consistency.
- [x] **Demographic Validation**: Implementation of Indian-centric naming and email domain distributions via `customer_config.yaml`.
- [x] **Device & ISP Profiling**: Implementation of realistic device fingerprinting and ISP-level behavioral attributes for each customer profile.
- [x] **Feature Correlation**: Enforcing structural relationships between Age, Credit Score, and Monthly Spend to ensure dataset realism.
- [ ] **Simulation Scalability**: Transitioning to a streaming Parquet reader for residential reference data to support multi-million agent populations without memory exhaustion.

## 💸 Transaction & Merchant Logic
- [x] **One-Pass Chunked Generation**: Refactoring of the generator to process cards in batches of 5,000, enabling multi-million transaction generation on standard hardware.
- [x] **Chronological Simulation**: Implementation of time-ordered transaction generation with support for temporal burst warping.
- [x] **MCC Mapping**: Mapping of OSM categories to standard Merchant Category Codes (MCC) for realistic financial analysis.
- [x] **Budget-Aware Simulation**: Transaction amounts are linked to the customer's `monthly_spend` profile, with noise added to individual events.
- [x] **Temporal weighted Patterns**: Implementation of circadian rhythms via hourly and daily weights in `transaction_config.yaml`.
- [x] **Device & Agent Persistence**: Implementation of persistent devices and realistic app identifiers (e.g., GPay, PhonePe) per payment channel.
- [x] **Amount Distribution Tuning**: Remediation of the "Amount Shortcut" by ensuring fraudulent amounts significantly overlap with legitimate spending distributions.
- [ ] **Geographic Precision**: Implementing the Haversine formula for all spatial velocity and distance calculations to replace Euclidean approximations.

## 🥈 ETL & Infrastructure
- [x] **Unified CLI Tooling**: Consolidation of multiple utility binaries into unified `etl`, `prepare_refs`, and `ingest` tools for improved developer experience.
- [x] **Streaming Infrastructure**: Integration of Redpanda (Kafka-compatible) for high-throughput, low-latency transaction event streams.
- [x] **Stateful Feature Store**: Integration of Redis for sub-millisecond retrieval of behavioral context and running statistical aggregates.
- [x] **Full-Stack Observability**: Implementation of Prometheus and Grafana dashboards for real-time monitoring of generation throughput and scoring latency.
- [x] **Zero-Copy Stdin Piping**: Optimization of the ETL pipeline to pipe Parquet data directly from Polars to ClickHouse `stdin`, eliminating intermediate disk I/O.
- [ ] **Streaming ETL Implementation**: Refactoring of runners to use `.scan_parquet()` and `.sink_parquet()` to support 10M+ row benchmarks without memory exhaustion.
- [ ] **Infrastructure Hardening**: Transitioning from hardcoded credentials to an `.env` and Docker Secrets management system.

## 🤖 Machine Learning & Model Training
- [x] **"Operational Feature" Pivot**: Refactoring of the training pipeline to focus exclusively on behavioral signals, explicitly excluding synthetic metadata to prevent label leakage.
- [x] **SHAP Interpretability**: Integration of SHAP (SHapley Additive exPlanations) for global and profile-specific feature importance validation.
- [x] **Real-Time Scoring Service**: Development of a stateful inference service (`scorer.py`) capable of sub-millisecond fraud detection on Kafka streams.
- [x] **Point-in-Time State Seeder**: Implementation of `seed_redis.py` to synchronize historical warehouse state with the real-time feature store using Welford's algorithm.
- [ ] **GNN-based Campaign Detection**: Transitioning to Graph Neural Networks (GNNs) for coordinated multi-entity attacks, as traditional classifier-based models (e.g., XGBoost) are inherently unsuited for capturing non-local relational patterns.
- [ ] **OOT Validation & Drift**: Transitioning to Out-of-Time validation and implementing a retraining scheduler to simulate model performance under adversarial concept drift.

## ⚙️ Configuration & Tuning
- [x] **Consolidated Control**: Integration of all generation volume and parallelism settings into a centralized `customer_config.yaml`.
- [x] **Modular Fraud Logic**: Implementation of a profile-driven mutation engine that decouples adversarial patterns from core simulation code.
- [x] **Product Catalog Centralization**: Consolidation of card types, networks, and limits in `product_catalog.yaml`.
- [ ] **Configuration Robustness**: Refactoring the configuration loader to provide graceful error handling and support for relative path resolution across different execution environments.
- [ ] **Campaign Attack Implementation**: Finalization of coordinated adversarial logic (currently disabled in configuration pending GNN-ready data structures).

---

## Known Issues
The roadmap is currently maintained as a flat list, which does not effectively communicate dependencies between complex tasks. Transitioning to a milestone-based roadmap is necessary to clarify the sequence of required changes. Furthermore, the reliance on XGBoost for all fraud types represents a known limitation; campaign detection requires a fundamental shift to graph-based modeling. Progress tracking remains a manual process; integration with automated source control status is needed to ensure the backlog reflects the true state of the codebase.
