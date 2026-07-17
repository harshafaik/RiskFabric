# IP and Device Reputation Removal

## Decision

Remove the Device/IP and Network reputation machinery from the RiskFabric generation and ETL pipeline. Keep `ip_address` and `user_agent` as plain identifier columns on the transaction, but delete the `*_fraud_rate`, `*_degree`, and `suspicious_cluster_member` derivations and their orphaned Silver tables. Defer all IP/device intelligence to a future external lookup table that the scorer consults at serve time, not a feature the model trains on.

Campaign features are explicitly out of scope for this decision and remain disabled-but-retained.

## Why this direction was chosen

**Leakage is better removed than managed.** `ip_fraud_rate` and `dev_fraud_rate` are global target encodings computed as `mean(is_fraud)` per entity over the same batch the model trains on. The generator routes fraud into near-categorical ranges (`185.`/`104.` VPN prefixes, a fixed 20-device UA pool) with no legitimate traffic sharing them, so the rate is a deterministic ground-truth proxy rather than a probabilistic signal. Masking, gating, or time-cutoff computation all keep the signal in the training loop. Deleting it from generation and training removes the leak at the source instead of patching it downstream.

**The current code is dead weight.** `network.rs` and `device_ip.rs` are already excluded from `silver-all`. `run_gold_master` hardcodes `ip_fraud_rate`, `dev_fraud_rate`, `ip_degree`, `dev_degree`, and `suspicious_cluster_member` to zero in `fact_transactions_gold`. `ip_features_silver` and `device_features_silver` are written by the Network stage but never joined into Gold. The generator still hand-crafts ISP subnets and VPN prefixes (`transaction_gen.rs`, `customer_gen.yaml` `isp_assignment`) purely to feed these unused features. This maintenance tax produces no usable ML feature today.

**An external lookup is the correct home for this signal.** A model frozen on static IP ranges is stale the moment abuse rotates. IP/device reputation belongs in a frequently-refreshed store the scorer enriches with at serve time, mirroring the existing `redis_seeder.md` / `seed_redis.py` pattern. Keeping it out of training data means the model trains on behavior and consults IP defensively. This also makes the "IP is a confirmer, not a decider" principle structural rather than a feature-masking rule.

## What is removed

- `src/etl/features/network.rs` and `src/etl/features/device_ip.rs` transforms.
- `run_silver_network` and `run_silver_device_ip` stages in `src/bin/etl.rs`.
- `ip_features_silver` and `device_features_silver` tables, and their `Row` structs in `src/clickhouse_types.rs`.
- Zero-fill columns `ip_fraud_rate`, `dev_fraud_rate`, `ip_degree`, `dev_degree`, `suspicious_cluster_member` in `run_gold_master`.
- `isp_assignment` and `device_profiles` reputation config from the generator where it exists only to feed the removed features.

## What is retained

- `ip_address` and `user_agent` columns on `fact_transactions_bronze` and downstream tables — required as join keys for any future external lookup.
- `geo_anomaly`, `device_anomaly`, and `ip_anomaly` columns in `fact_fraud_metadata_bronze` — these are ground-truth injector labels, not reputation features, and remain valid model targets and features.

## Future implementation

**External IP/device reputation store.** Stand up a Redis-backed lookup table (consistent with `seed_redis.py`) keyed by `ip_address` and `user_agent`. The table is computed from live or recent transactional data on a separate cadence from model training, so the model never observes label-derived reputation during training. The scorer reads the lookup at inference and applies it defensively — only after behavioral flags (`rapid_fire_transaction_flag`, `escalating_amounts_flag`, `merchant_category_switch_flag`, the `*_anomaly` flags) already indicate suspicion.

**Generator overlap, if reputation is ever re-derived internally.** Should the project later re-introduce internal IP reputation, the generator must route a share of legitimate traffic (`~5%`) through datacenter/VPN ranges and route a share of fraud (`~20%`) through residential IPs, so the reputation rate becomes probabilistic rather than a `100%` tell. This overlaps fraudster and legitimate infrastructure, converting the feature from a deterministic proxy into an informative one.

**Relational signals over aggregates.** If network structure is needed, compute `ip_degree` as distinct-customer counts and `suspicious_cluster_member` via shared-entity detection. These are label-free and do not leak, unlike the rate aggregates removed here.
