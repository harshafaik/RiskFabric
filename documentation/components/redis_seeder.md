# Redis Feature Seeder (`seed_redis.py`)

## Summary
The `seed_redis.py` script is an operational utility that initializes the real-time feature store (Redis) with historical data from the warehouse (ClickHouse). It bridges the gap between the batch-trained model and the streaming inference engine by ensuring that every card and customer has immediate behavioral context before real-time transactions start arriving.

## Architectural Decisions
This seeder is designed to facilitate **Warm-Start Inference**. Without this script, the first few transactions for every card in the streaming pipeline would be difficult to score accurately (as there would be no "previous" location for velocity or "previous" amount for Z-score). The seeder extracts the most recent state for every card and customer, including the last 10 transactions, the final coordinate pair, and the cumulative count of events.

A key architectural choice is the **Redis Hash/List strategy**. Redis Lists (`RPUSH`) are used to store chronological card history and Hashes (`HSET`) to store aggregate statistics. This allows `scorer.py` to perform O(1) lookups for behavioral context, maintaining the strict latency requirements of real-time fraud detection. Furthermore, the seeder explicitly calculates the initial Welford state (Mean and M2) from the warehouse, enabling the online scorer to continue updating statistical variance incrementally without a full history scan.

## System Integration
`seed_redis.py` acts as a synchronization service between the **Warehouse layer** (ClickHouse) and the **Scoring layer** (Redis/Kafka). It must be executed after `etl.rs` completes (to ensure the "Gold" table is populated) and before `stream.rs` and `scorer.py` are started.

## Known Issues
The entire feature initialization set is currently pulled into local Python memory before pushing to Redis. For datasets with millions of cards, this may lead to a **memory-exhaustion failure**. The ClickHouse queries should be refactored to use chunked fetching (cursors) or a parallelized worker pool implemented to stream data from the warehouse to Redis in batches. Additionally, a hardcoded password for ClickHouse is currently used; this should be moved to an environment variable to align with project security standards.
