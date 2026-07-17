# ETL Pipeline Migration & Rot Audit

## Completed: Shell → Native Client Migration

All `podman exec clickhouse-client` shell invocations in `src/bin/etl.rs` and `src/bin/ingest.rs` replaced with the `clickhouse` Rust crate (clickhouse-rs v0.14). Connection is env-var driven:

| Variable | Default |
|---|---|
| `CLICKHOUSE_HOST` | `localhost` |
| `CLICKHOUSE_HTTP_PORT` | `8123` |
| `CLICKHOUSE_DB` | `riskfabric` |
| `CLICKHOUSE_USER` | `riskfabric_user` |
| `CLICKHOUSE_PASSWORD` | `123` |

Zero `podman exec` / `FORMAT Parquet` references remain in `src/`.

New files: `src/clickhouse_client.rs`, `src/clickhouse_types.rs`.


## Verified Clean: Row Structs vs DDLs vs Queries

All 12 write Row structs and 9 read Row structs are in exact alignment with their ClickHouse table DDLs and SELECT queries. The migration forced this — the `clickhouse` crate rejects field count/name/type mismatches at serialization time, unlike the old `FORMAT Parquet` positional pipe.


## Bug Fixed: `transaction_sequence_number` Type Mismatch

- **File:** `src/etl/features/sequence.rs` line ~128
- **Before:** `.cast(DataType::UInt64)` — DDL says `UInt32`, `df_to_silver_transactions` reads via `.u32()`
- **Effect:** Every `transaction_sequence_number` in Silver and Gold was silently written as `0`
- **Fix:** Changed to `.cast(DataType::UInt32)`


## Bug Found: `fact_transactions_silver` Transform Has 5 Orphan Columns

`transform_sequence_features` produces 35 columns via `.select()`, but the DDL only has 30. Five are `lit(0)` placeholders:

- `same_day_transaction_count`
- `is_holiday`
- `is_foreign`
- `is_cross_border`
- `is_ip_mismatch`

These are silently dropped in `df_to_silver_transactions` and consume zero storage. Decide: remove them from the transform, or add them to the DDL and Row struct if you plan to use them later.


## Not Fixed (Pipeline-Level): Campaign Features

- **Generator config:** `data/config/fraud_rules.yaml` → `fraud_campaigns.target_campaign_share: 0.0`
- Campaign assignment is disabled at the generator level. Every `campaign_id` in bronze is `None`.
- The ETL transform (`campaign.rs`) has a non-deterministic bug: no sort before `cum_sum`, making campaign sequence numbering unreliable.
- Isolated single fraud transactions become fake "campaigns of size 1."
- Hardcoded 48h gap threshold is untunable.

**Status:** Disabled in CLI (`--silver-campaign` shows a warning). Gold hardcodes zeros for all campaign columns. No data to join.

**To use campaigns one day:** Set `target_campaign_share > 0` in config, fix the sort/cum_sum ordering bug, and decide whether single-tx "campaigns" should be filtered.


## Decision: Device/IP and Network Features Removed

These stages are no longer deferred as "not fixed" — they are **removed by decision**. See `documentation/decisions/ip_device_reputation_removal.md` for full rationale. Summary:

- `network.rs` and `device_ip.rs` transforms, the `run_silver_network` / `run_silver_device_ip` stages, the `ip_features_silver` / `device_features_silver` tables and their `Row` structs, and the zero-fill Gold columns (`ip_fraud_rate`, `dev_fraud_rate`, `ip_degree`, `dev_degree`, `suspicious_cluster_member`) are being deleted.
- `ip_address` and `user_agent` remain on the transaction rows as join keys for a future external lookup; `geo_anomaly`, `device_anomaly`, `ip_anomaly` remain as injector labels.
- IP/device intelligence is deferred to a future Redis-backed external lookup consulted defensively by the scorer, keeping it out of model training entirely.

**Status:** Removed by design. Not merely disabled. Campaign features are deliberately untouched by this decision.


## Design Issue: Entity Reputation = Target Encoding = Data Leakage

IP reputation (`ip_fraud_rate = mean(is_fraud) per IP`) and device reputation (`dev_fraud_rate = mean(is_fraud) per user_agent`) are **global target encodings**. The model trains on the same data used to compute these aggregates. Result: features dominate SHAP because they're proxies for the generator's internal assignment logic, not behavioral signals.

**Why this happens in the generator:**

- ATO fraud set to `52.x`/`34.x` IP ranges (AWS/GCP datacenters) and a fixed 20-device UA pool
- CNP set to `185.x`/`104.16.x` IP ranges (VPN)
- Velocity abuse set to `45.60.x.x`
- No legitimate customer ever touches these IPs or UAs

These IP ranges and UA strings are 100% fraud-predictive because the generator never puts legitimate traffic on them. The model learns "datacenter IP = fraud" trivially and stops using behavioral features.

**What to do about it:**

### Option A (preferred, root cause fix at generator level)

Share infrastructure between fraud and legitimate users:

- Route ~5% of legitimate transactions through datacenter/VPN IP pools (users on cloud gaming, corporate VPNs, CI/CD)
- Route ~20% of fraud through residential IPs (compromised home routers, insider fraud on home connections)
- Overlap user agents: fraudsters should spoof common browser fingerprints, not use a dedicated pool

This makes reputation features properly probabilistic — an IP with 80% fraud rate is informative; one with 100% is leaked.

### Option B (quick fix at pipeline level, less realistic)

Use time-cutoff computation instead of global aggregates:

```sql
-- Before (current — trains on future data):
SELECT ip_address, avg(is_fraud) as ip_fraud_rate
FROM fact_transactions_bronze
GROUP BY ip_address

-- After (only prior transactions):
SELECT ip_address, avg(is_fraud) as ip_fraud_rate
FROM fact_transactions_bronze AS t
WHERE t.timestamp < (SELECT timestamp FROM fact_transactions_bronze WHERE ...)
```

This kills the leakage but still gives the model low-signal features because the underlying IP assignment is still deterministic.

### Option C (relational signals instead of aggregates)

Build graph-based features instead of rates:

- `ip_degree`: count of distinct customers sharing this IP
- `suspicious_cluster_member`: 1 if 2+ customers who use this IP have fraud history
- `shared_entity_fraud_rate`: fraud rate across all accounts touching this IP

These can't leak because they don't require looking at the current transaction's label — they're purely relational.


## Resolution: Entity Reputation Removed

The target-encoding leakage described above is not being fixed in-place. Per `documentation/decisions/ip_device_reputation_removal.md`, the IP/device reputation features are **removed** from generation and training; IP/device intelligence is deferred to an external lookup. Options A–C above are retained only as future re-introduction guidance, not pending work. Campaign features remain out of scope for this resolution.
