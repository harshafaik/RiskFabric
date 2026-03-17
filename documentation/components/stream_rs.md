Language: Rust

The streaming generator produces unlabeled transactions at a configurable rate and publishes them to the `raw_transactions` Kafka topic for real-time scoring.

It reuses `generate_transactions_chunk` from the batch pipeline — the core generation logic is untouched. The one-pass architecture is preserved: transactions and fraud metadata are produced in a single traversal, then separated at the output layer via `UnlabeledTransaction`, which is a struct that mirrors `Transaction` but omits `is_fraud`, `chargeback`, and all label fields entirely. The Kafka payload is guaranteed label-free at the type level.

The generator operates in two modes, controlled by `streaming_mode` in `generator_config.yaml`:

- **Pure streaming** (`streaming_mode: true`) — behavioral mutations active, no labels assigned, no metadata collected. Used for live fraud detection.
- **Verification mode** (`streaming_mode: false`) — identical Kafka output, but ground truth labels are captured internally to `ground_truth.csv` via `FraudMetadata`. Used to measure scorer precision/recall by joining against `fraud_scores` after a test run.

The rate limiter targets configurable throughput (default 100 tx/s) using a self-correcting mechanism — each send measures actual Kafka latency and sleeps only the remaining interval, preventing cumulative drift under variable broker response times.

The merchant population is loaded from `data/references/ref_merchants.parquet` and indexed at H3 resolutions 4 and 6 for spatial locality lookups during generation.

**Known issue:** Population size is hardcoded to 1,000 customers, decoupled from the batch pipeline's 10,000 customer population. This should be moved to config to ensure Redis seeding and streaming population are consistent.

