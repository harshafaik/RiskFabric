# RiskFabric Project To-Do List

## Customer Generation
- [x] **Fix Location Heuristics:** Currently, `location_type` (Urban/Rural) is assigned based on city name or fallback from config.
- [ ] **Spatial Jittering (Clustering):** Instead of pointing all customers to exact OSM node coordinates, introduce a small random "drift" (jitter) to simulate neighbors and improve spatial clustering.
- [x] **Improve City Name Fallbacks:** Using "{State} Region" for missing city names to maintain geographic consistency.
- [x] **Validate Demographics:** Indian-centric names and email domains implemented via `customer_config.yaml`.

## Transaction & Merchant Logic
- [ ] **Accurate MCC Mapping:** Map `merchant_category` from OSM to standard Merchant Category Codes (MCC) for realistic financial analysis.
- [ ] **Budget-Aware Generation:** Link transaction amounts to the customer's `monthly_spend`. Ensure the sum of monthly transactions respects the customer's financial profile.
- [ ] **Temporal Refinement:** Implement hourly/daily weights for transactions (e.g., more restaurant spend in the evening, less banking at 3 AM).
- [ ] **User Agent Variance:** Reduce `user_agent` string length and increase variance/realism for different payment channels.
- [ ] **Spatial Refinement:** Review how transaction locations are selected. Ensure "near home" transactions aren't just at one point, but spread across a local radius.

## ETL & Infrastructure
- [x] **Fix Polars API Breakage:** Migrated ETL logic to Polars 0.51.0. Implemented UInt32 casting for flags and Int32 casting for weekday comparisons to resolve engine panics.
- [x] **Data Quality Audit:** Resolved schema mismatch between Polars output and ClickHouse Silver layer. Fixed timestamp precision (nanoseconds to seconds) and ensured all 22 feature columns are correctly populated.
- [x] **Silver & Gold ETL Suite:** Ported all 6 Silver logic modules and the Gold Master Join logic from Python/Spark to Rust/Polars. Verified with a 180k transaction dry run.
- [x] **Multi-Table Feature Joins:** Successfully implemented Master Joins between raw transactions and feature tables (Customer, Merchant, Device, Campaign, Network) to produce the final denormalized ML table.
- [ ] **Polars Streaming Implementation:** Refactor runners to use `.scan_parquet()` and `.sink_parquet()` (Streaming) to support the 100M+ row benchmark without OOM errors.
- [ ] **Scale Customer Features:** Optimize `etl_silver_customer` to handle joins across 100M transactions without exhausting memory.
- [ ] **Scale Customer Features:** Optimize `etl_silver_customer` to handle joins across 100M transactions without exhausting memory.
- [x] **Optimize Transaction Generation:** Achieved ~180k TPS (150k transactions in 823ms) using "One-Pass" parallel generation. 
- [ ] **Scalability Testing:** Validate that the "One-Pass" approach holds up for 1M+ customers (approx 15M transactions) and monitor memory usage.

## Machine Learning & Model Training
- [x] **Ingest Ground Truth Metadata**: Load `fraud_metadata.parquet` into ClickHouse (Bronze) to provide clean labels and anomaly flags for training.
- [x] **Enrich Silver Layer**: Update `etl_silver_sequence` to join the fraud metadata, replacing placeholders with actual ground-truth flags.
- [x] **Expand Gold ML Table**: Update Gold ETL to include all categorical features (merchant_category, channel) and behavioral signals for the model.
- [x] **XGBoost Training Pipeline**: Implement a Polars-based training script to train and evaluate the fraud detection model.
- [x] **Model Persistence**: Save trained models with versioning and feature metadata for consistent inference.
- [x] **Performance Tracking**: Log training and test metrics in `documentation/ml_metrics.md` for each model iteration.
- [ ] **Inference Service**: Create a fast inference script or service to score new transactions in real-time.
- [ ] **Feature Importance Analysis**: Analyze which synthetic patterns (UPI, ATO) are most identifiable by the model.

## Configuration & Tuning
- [x] **Externalize Hardcoded Parameters:** Moved magic numbers and tuning constants from Rust source code (`src/models/`, `src/generators/fraud.rs`) into YAML files.
- [x] **Product Catalog:** Centralized account/card types, networks, and default limits in `product_catalog.yaml`.
- [x] **Transaction & Geo Tuning:** Externalized merchant categories, success rates, and India-specific geo-bounding boxes in `transaction_config.yaml`.
- [x] **Fraud Operational Tuning:** Created `fraud_tuning.yaml` to separate attack probabilities and anomaly settings from the core fraud pattern definitions.
- [ ] **Decouple Rules from Tuning:** Implement two distinct loaders in `src/config.rs` to separate "Fraud Patterns" (Rules) from "Operational Knobs" (Tuning) for better architectural modularity.
