# Performance Benchmarks

This page documents measured performance characteristics of the RiskFabric platform. Every measurement is reproducible — same seed, same data, same hardware. Each benchmark is a standalone, runnable script in `benchmarks/`. Run the combined suite with `bash benchmarks/run_all.sh`; it checks dependencies and skips unavailable services with clear error messages and the exact commands to run manually.

## Hardware

| Component | Detail |
| :--- | :--- |
| CPU | AMD Ryzen 7 4800H (8 physical / 16 logical cores) |
| RAM | 15.0 GB |
| Disk | btrfs, 475.4 GB NVMe |
| OS | Linux (Fedora 44) |
| Python | 3.14.5 |
| Rust | 1.95.0 (2026-04-14) |

## Methodology

All benchmarks use **seed=42**, **N=5 repetitions** (unless noted), and report **mean ± std**. Latency is always reported as **p50/p95/p99** percentiles — never averages alone. Hardware specs are printed at the top of every run so numbers are traceable to the machine that produced them.

## Summary

| Domain | Metric | Value |
| :--- | :--- | ---: |
| Scoring | End-to-end latency batch=50 (p50 / p95 / p99) | 173 / 505 / 550 ms |
| Scoring | End-to-end latency batch=1 (p50 / p95 / p99) | 11 / 27 / 30 ms |
| Scoring | Sustained throughput batch=50 / batch=1 | 82 / 3 tx/s |
| Generation | Throughput (3,400 customers, 1.54M txn) | 128,000 tx/s |
| Generation | Peak RSS at 1.5M transactions | 3.6 GB |
| Generation | Parquet output (transactions) | 46.8 bytes/txn |
| Data pipeline | DuckDB Gold extraction (1.54M rows) | 1,616 ms (p50) |
| Data pipeline | ClickHouse equivalent query | 2,039 ms (p50) |
| Data pipeline | ETL Silver + Gold (1.54M rows) | 6.3 s total |
| ML training | Honest AUC (chronological split) | 0.7622 |
| ML training | Target-encoding leak gap | +0.20 AUC |
| ML training | Random-split leak gap | +0.02 AUC |
| Redis | All operations p50 / p99 | < 261 / < 509 µs |
| Redis | Per-transaction overhead (13 ops) | 2,015 µs |
| Streaming | Rate-limiter accuracy (target 100 tx/s) | 88 tx/s (−12%) |
| Generation | Fraud-injection overhead | +64% |

## Scoring Pipeline

### End-to-End Latency & Throughput

Benchmark: `benchmarks/bench_scoring.py`. Dependencies: Redpanda + ClickHouse + Redis (`podman compose up -d`).

End-to-end pipeline latency (`scored_at − kafka_received_at`) measured from 49,250 scored rows over 10 minutes at batch_size=50. The "sub-second latency" claim in the documentation is confirmed — all three percentiles are well under 1,000 ms.

| Metric | batch=50 | batch=1 |
| :--- | ---: | ---: |
| p50 latency (ms) | 173 | 11 |
| p95 latency (ms) | 505 | 27 |
| p99 latency (ms) | 550 | 30 |
| Throughput (tx/s) | 82 | 3 |

Micro-batching at 50 delivers a 27× throughput improvement over batch=1 (82 vs 3 tx/s) at the cost of accumulation delay. The dominant latency component is waiting to fill the batch — per-transaction work (Redis + XGBoost inference + ClickHouse sink) is ~11 ms at p50, while batch=50 adds ~162 ms of accumulation wait. This is the classic latency/throughput tradeoff: batch=1 gives the lowest possible latency but XGBoost and ClickHouse insert overhead per transaction caps throughput at ~3 tx/s; batch=50 amortizes that overhead across 50 predictions and one ClickHouse insert.

**How to run:** `podman compose up -d && cargo run --release --bin stream & python src/ml/scorer.py &` — wait 5 minutes, then `python benchmarks/bench_scoring.py`.

### Redis Lookup Latency

Benchmark: `benchmarks/bench_redis.py`. Dependencies: Redis (`podman compose up -d redis`).

10,000 iterations per operation, single Redis connection, no pipelining. The "sub-millisecond" claim across the documentation is confirmed — every operation at both p50 and p99 is well under 1 ms. Per-transaction Redis overhead (13 sequential operations) is ~2.0 ms, roughly 1% of the ~173 ms p50 end-to-end latency. The dominant latency component is the micro-batch accumulation delay (waiting to fill a batch of 50) plus XGBoost inference — the batch=1 comparison isolates this tradeoff. When micro-batching delay dominates, batch=1 should show dramatically lower per-transaction latency at the cost of throughput.

| Operation | p50 (µs) | p99 (µs) | Mean (µs) |
| :--- | ---: | ---: | ---: |
| HGETALL (customer stats) | 145 | 509 | 169 |
| HGETALL (customer agg) | 138 | 305 | 154 |
| HGETALL (merchant agg) | 136 | 229 | 146 |
| HGETALL (card location) | 146 | 292 | 161 |
| HSET (save stats) | 152 | 379 | 166 |
| GET (last timestamp) | 125 | 196 | 132 |
| SET (update timestamp) | 138 | 235 | 147 |
| ZADD (burst add) | 131 | 210 | 138 |
| ZREMRANGEBYSCORE (burst evict) | 127 | 225 | 135 |
| ZCARD (burst count) | 121 | 203 | 126 |
| LINDEX (peek history) | 138 | 200 | 141 |
| LPUSH + LTRIM (push history) | 261 | 446 | 274 |
| INCR (sequence) | 120 | 193 | 124 |
| **Per-transaction total (13 ops)** | **2,015** | — | **2,015** |

**How to run:** `podman compose up -d redis && python benchmarks/bench_redis.py`.

### Streaming Rate-Limiter Accuracy

Benchmark: `benchmarks/bench_ratelimit.sh`. Dependencies: Redpanda (`podman compose up -d redpanda`).

The streaming generator targets 100 tx/s via a fixed-interval sleep loop. Over a 120-second run it produced 10,600 transactions (~88 tx/s), ~12% below the configured target. The shortfall is likely due to Kafka broker latency and per-iteration customer/account/card regeneration overhead. The output is steady — each batch fires at a consistent cadence — but the absolute rate trails the target.

| Target | Duration | Measured Rate | Error |
| ---: | ---: | ---: | ---: |
| 100 tx/s | 120 s | 88 tx/s | −12% |

**How to run:** `podman compose up -d redpanda && bash benchmarks/bench_ratelimit.sh`.

## Batch Generation

### Throughput & Scaling

Benchmark: `benchmarks/bench_generation.sh`. Dependencies: Rust toolchain, `data/references/ref_*.parquet`.

Default configuration (3,400 customers, 1.54M transactions) runs in ~12 seconds, producing 70 MB of Parquet at 46.8 bytes per transaction. Peak RSS is 3.6 GB — the 5,000-card chunk strategy keeps memory bounded, but scaling to 100K+ customers will push past the 15 GB available on this machine.

| Customers | Transactions | Wall Time (s) | tx/s | Peak RSS (MB) | Parquet Size (MB) | Bytes/txn |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 3,400 | 1.54M | 12.0 | 128,000 | 3,662 | 70.1 | 46.8 |
| 10,000 | 4.47M | 25.4 | 176,000 | 6,532 | 202.5 | 47.4 |
| 100,000 | — | — | — | — | — | — |
| 1,000,000 | — | — | — | — | — | — |

> 100,000 customers OOMed this 15 GB machine — 15 GB is insufficient. At 10,000 customers (4.47M txn, 6.5 GB RSS), the memory curve is visible: RSS grows ~2× for a ~3× txn increase. Extrapolating, 100K customers would need ~40−50 GB and 1M customers ~400−500 GB. The 5,000-card chunk strategy limits per-chunk allocation, but the final Parquet merge step materializes the full dataset. Run 100K on a machine with 64+ GB RAM; run 1M on a machine with 512+ GB RAM.

**Core scaling** Rayon thread counts at the 3,400-customer scale show no meaningful speedup — thread-pool overhead dominates when per-card transaction generation is already amortized within each chunk. Benefits are expected to appear at 10K+ customers where per-card parallelism is the bottleneck.

| RAYON_NUM_THREADS | Wall Time (s) | tx/s |
| ---: | ---: | ---: |
| 1 | 10.5 | 147,000 |
| 2 | 13.3 | 117,000 |
| 4 | 10.2 | 152,000 |
| default (16) | 12.0 | 128,000 |

**How to run:** `bash benchmarks/bench_generation.sh` (requires `cargo run --release --bin export_references` first-time setup). For core scaling: `RAYON_NUM_THREADS=8 bash benchmarks/bench_generation.sh`.

### Fraud-Injection Overhead

Benchmark: `benchmarks/bench_fraud_overhead.sh`. Dependencies: Rust toolchain, reference data.

Fraud injection adds ~64% generation overhead (7.3 s → 12.0 s at default settings). This is expected — the fraud path runs amount mimicry, behavioral mutations (UA/IP/geo), campaign coordination, and metadata writes. The injector evaluates `fraud_target` on every transaction, not just the ~1% that are actually fraudulent.

| Configuration | Wall Time (s) | Overhead |
| :--- | ---: | ---: |
| Fraud off (target_share=0.0) | 7.3 | — |
| Fraud on (target_share=0.01) | 12.0 | +64% |

**How to run:** `bash benchmarks/bench_fraud_overhead.sh`.

## Data Pipeline

### DuckDB vs ClickHouse

Benchmark: `benchmarks/bench_storage.py`. Dependencies: ClickHouse (`podman compose up -d clickhouse`), Gold Parquet snapshot.

> ClickHouse numbers obtained by reconstructing the defunct Gold path for benchmarking only. ClickHouse is streaming-only in production; this scaffold exists solely to validate the Storage Architecture Split decision.

Both engines extract 1.54M rows from the same 102 MB Gold Parquet snapshot. DuckDB is slightly faster despite being an in-process embedded library with zero infrastructure — no daemon, no port, no container, no connection pool. The old batch pipeline ingested Parquet into ClickHouse, read it back for feature computation, and wrote it again, a round-trip the new DuckDB path eliminates entirely.

| Engine | Query time p50 | p95 | p99 | Infrastructure |
| :--- | ---: | ---: | ---: | :--- |
| DuckDB | 1,616 ms | 1,716 ms | 1,727 ms | in-process, 0 servers, 0 ports, 0 containers |
| ClickHouse | 2,039 ms | 2,334 ms | 2,349 ms | 1 server, 1 port, 1 container |

**How to run:** `podman compose up -d clickhouse && python benchmarks/bench_storage.py`.

### ETL Throughput

Benchmark: `benchmarks/bench_etl.sh`. Dependencies: Rust toolchain, generated Parquet (`data/output/transactions.parquet`).

The complete batch pipeline (generate → Silver → Gold) takes ~18 seconds end-to-end for 1.54M transactions on this machine. Generation dominates at ~12 seconds; ETL is fast at ~6 seconds because it's pure Polars over Parquet with no database I/O.

| Stage | Wall Time (s) | Rows | Rows/sec |
| :--- | ---: | ---: | ---: |
| Silver-all | 4.3 | 1.54M txns + sequence + customer + merchant | ~360K |
| Gold-master | 2.0 | 1.54M | ~770K |
| **Total (Silver + Gold)** | **6.3** | — | — |

**How to run:** `bash benchmarks/bench_etl.sh`.

## ML Training

### Leakage AUC

Benchmark: `benchmarks/bench_leakage.py`. Dependencies: Python, Gold Parquet snapshot.

#### Documented History

Chronology extracted from `documentation/feature_leakage_issues.md`. These are development-log measurements, not reproducible benchmarks — included for provenance.

| Stage | AUC | What changed |
| :--- | ---: | :--- |
| Leak 1: amount as shortcut | 0.9079 | Raw `amount` had 96.3% feature importance |
| Leak 3: tuned amount, weak behavioral | 0.7960 | Amount retuned but behavioral signals missing |
| Leak 4: target encoding leak | 0.925 | `mf_fraud_rate`, `cf_fraud_rate` leaked from full batch |
| Leak 5: join destroys sort | — | 70% corrupted sequence features |
| Leak 6: random split | 0.786 | Random split inflates AUC vs chronological 0.7622 |
| **FINAL (honest)** | **0.7622** | All fixes, chronological split |

#### Ablation Study (seed=42, N=5)

Controlled experiments that individually reintroduce leak classes. These are **not** historical measurements — labels match methods. Target-encoded features (`mf_fraud_rate`, `cf_fraud_rate`) are the dominant leak class, inflating AUC by ~0.20. Random split inflation is ~0.02, consistent across seeds. The `amount` feature no longer leaks meaningfully (0.7622 → 0.7626) because fraud amount mimicry was retuned to overlap legitimate distributions — the 0.9079 from the development log was from a pre-retuning config.

| Variant | AUC (mean ± std) | What's reintroduced |
| :--- | ---: | :--- |
| Honest baseline (chronological split) | 0.7622 ± 0.0000 | 10 features, chronological split |
| + amount feature | 0.7626 ± 0.0000 | `amount` added to features (fraud injector directly sets it) |
| + target encoding | 0.9634 ± 0.0000 | `mf_fraud_rate`, `cf_fraud_rate` (direct label leak) |
| + random split | 0.7838 ± 0.0000 | `sklearn.train_test_split` instead of chronological |

**How to run:** `python benchmarks/bench_leakage.py`.
