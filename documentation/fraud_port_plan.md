# Fraud Logic Porting Plan (Complete)

This document outlines the strategy for porting fraud injection and mutation logic from the Go-based `fraud-service` to the Rust-based `riskfabric` generator.

## 1. Dependency Updates (Complete)
- `sha2`, `hex`, `serde_yaml` added to `Cargo.toml`.

## 2. Data Model Enhancement (Two-Table Approach) (Complete)

### Transaction Table (Realistic Fields)
In `src/models/transaction.rs`, updated the `Transaction` struct:
- **Core Features**: `transaction_channel`, `card_present`, `user_agent`, `ip_address`, `merchant_country`.
- **Outcome/Post-Transaction**: `auth_status`, `failure_reason`, `chargeback`, `chargeback_days`.
- **Labels**: `is_fraud` (Remaining here for training visibility).

### Fraud Metadata Table (Internal Generation Context)
Created `src/models/fraud_metadata.rs`:
- `transaction_id` (FK), `fraud_target`, `fraud_type`, `label_noise`, `injector_version`.
- `geo_anomaly`, `device_anomaly`, `ip_anomaly`, `burst_session`, `burst_seq`, `campaign_id`.

## 3. Core Logic Implementation (`src/generators/fraud.rs`) (Complete)
- **Deterministic Hashing**: `hash01` using `Sha256` (16-char hex prefix).
- **FraudInjector**: Target selection, label noise, and weighted profile picking.
- **FraudMutator**: Mutation rules for UPI and SIM swap fraud, anomaly injection.

## 4. Configuration (Complete)
- Basic configuration implemented directly in the generator. (YAML parsing can be extended using `serde_yaml`).

## 5. Integration (Complete)
- `src/generators/transaction_gen.rs` updated to return `(Vec<Transaction>, Vec<FraudMetadata>)`.
- `src/bin/generate.rs` updated to save both `transactions.parquet` and `fraud_metadata.parquet`.
