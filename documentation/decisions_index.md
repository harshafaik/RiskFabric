# Decisions

Architectural and design decisions that have changed or removed functionality in RiskFabric. These documents record the rationale behind structural choices that are not visible from the code alone.

- [IP and Device Reputation Removal](decisions/ip_device_reputation_removal.md) — why Device/IP and Network reputation features were deleted from generation and training and deferred to an external lookup.
- [Storage Architecture Split](decisions/storage_architecture_split.md) — why batch ETL and training data move off ClickHouse to a Parquet + DuckDB pipeline, leaving ClickHouse as a streaming-only analytics engine.
- [Deployment Architecture](decisions/deployment_architecture.md) — how RiskFabric runs locally via Docker Compose and on AWS with Terraform (EC2 + RDS + S3), with full cost breakdown and rationale for keeping Redis and Redpanda self-hosted.
- [Fraud Operations Cost Model](decisions/fraud_operations_cost_model.md) — researched cost of fraud alert investigation in Indian financial institutions and how RiskFabric's architecture addresses false positive waste, investigation time, and fraud catch rate.
