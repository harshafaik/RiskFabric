# Project Roadmap & Backlog

## Summary
The `to-do.md` document serves as the tactical roadmap for RiskFabric. It prioritizes engineering tasks and research objectives required to evolve the simulation from a prototype into a production-grade synthetic data platform.

## Design Intent
The roadmap is designed to be **Value-Driven**. Tasks are organized by their impact on the fidelity and utility of the simulation. Maintaining a public backlog provides a signal to contributors regarding critical system gaps, such as geographic realism, adversarial diversity, or operational performance.

---

## 🏗️ High-Priority Engineering
- [ ] **Native Database Drivers**: Replace `podman exec` calls with `clickhouse-rs` and `tokio-postgres`.
- [ ] **Incremental Generation**: Implement stateful resumption to allow adding to existing datasets.
- [ ] **Unified CLI**: Consolidate auxiliary binaries into a single `riskfabric` command-line tool.

## 🧠 Machine Learning Research
- [ ] **Concept Drift Simulation**: Implement time-varying fraud profiles to test model robustness.
- [ ] **OOT Validation Pipeline**: Move to Out-of-Time validation as the primary performance metric.
- [ ] **Graph Signal Generation**: Implement Account-to-Account (A2A) transfer chains.

---

## Known Issues
The roadmap is currently treated as a **flat list**, which does not communicate dependencies between tasks. For instance, implementing "Graph Signal Generation" requires significant refactoring of the "One-Pass" architecture to support inter-entity state. Transitioning to a milestone-based roadmap is necessary to define the sequence of architectural changes required for advanced features.

Furthermore, **Progress Tracking** is a manual process. Features are often completed without immediate updates to this document, resulting in stale tasks. Integrating the roadmap with source control milestones is required to ensure it reflects the current state of the codebase.
