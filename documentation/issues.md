# Technical Issues & Resolutions

## Summary
The `issues.md` document acts as the primary engineering log for RiskFabric. It captures architectural hurdles, environment-specific bugs, and performance bottlenecks encountered during the development of the simulation engine, along with their implemented or proposed resolutions.

## Design Intent
This document serves as **Institutional Knowledge** for the project. In complex simulations, the most difficult bugs often arise from the interaction between system layers (e.g., Rust → Kafka → Python). Documenting these issues provides a roadmap for future optimizations and prevents the repetition of architectural errors.

A critical part of this design is the focus on **Empirical Resolution**. Every issue listed is paired with a technical fix or a design change validated through performance benchmarking or regression testing. This ensures the document remains a practical engineering resource.

---

## 🛠️ High-Impact Issues

### 1. Parquet Serialization Bottleneck
- **Problem**: Transaction generation required 44 seconds, with 90% of the time spent in disk I/O and Parquet encoding.
- **Resolution**: A **One-Pass Parallel Architecture** was implemented and the Polars chunk size was optimized. This reduced the runtime to 4.4 seconds, a 10x improvement.

### 2. Label Leakage in Training
- **Problem**: Early models achieved 99.9% AUC by learning internal generator flags (e.g., `geo_anomaly`) instead of behavioral signals.
- **Resolution**: A strict **"Honest Feature" Sanitization** step was implemented in `train_xgboost.py` to drop internal metadata before training.

---

## Known Issues
There is ongoing difficulty with **Container Runtime Variability**. The `podman exec` calls used in ingestion and ETL pipelines behave inconsistently across Linux and macOS environments, causing failures in the data warehouse loading process. Transitioning to native database drivers is required to eliminate dependency on the host's container CLI.

Furthermore, **Memory Management during Reference Extraction** is currently insufficient. When processing large OSM PBF files, the `prepare_refs.rs` binary can consume up to 16GB of RAM. Implementing a "Spill-to-Disk" strategy for the parallel map-reduce operation is necessary to maintain a memory footprint below 4GB.
