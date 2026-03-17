# Infrastructure & Local Service Stack

## Summary
The RiskFabric simulation is supported by a comprehensive local service stack orchestrated via **Docker Compose**. This infrastructure provides the multi-modal data environment—relational, columnar, stream, and cache—required to simulate a modern financial technology ecosystem. It enables the end-to-end lifecycle of synthetic data, from geographic world-building to real-time adversarial detection.

## Architectural Decisions
The infrastructure is designed using a **Multi-Model Database Strategy**. By incorporating **ClickHouse** for high-volume transactions and **Postgres/PostGIS** for geographic preparation, each stage of the simulation uses the optimal storage engine for its specific data type. The inclusion of **Redpanda** (a Kafka-compatible event store) and **Redis** facilitates the real-time scoring path, allowing the simulation to model the sub-millisecond latency requirements of production fraud systems.

For **Observability**, **Prometheus and Grafana** are integrated directly into the core stack. This architectural decision transforms RiskFabric from a simple data generator into a performance benchmarking environment. By instrumenting the database exporters and the real-time scorer, system metrics (e.g., Kafka ingestion lag, Redis lookup latency, and model inference time) can be visualized in real-time, providing visibility into the operational impact of different fraud detection strategies.

The use of **Healthchecks** across all critical services ensures that the generation binaries (`ingest.rs`, `etl.rs`) only attempt to connect when the infrastructure is ready. This improves the developer experience by reducing connection-refused errors during the initial cold-start of the simulation environment.

## System Integration
The infrastructure is the foundation upon which all RiskFabric binaries execute. The Rust-based generators and Python-based ML services connect to these containers via standardized ports and internal networks. The `scorer` service is configured to run as a long-lived container, automatically subscribing to the Kafka stream as soon as the stack is up.

## Known Issues
A **Single-Node Redpanda** instance without persistence is currently used. While this is sufficient for local development, it does not support testing "Consumer Group Rebalancing" or "Partition-Level Parallelism," which are common challenges in production streaming systems. A multi-node Redpanda cluster configuration is required to support high-availability testing scenarios.

Furthermore, **Postgres and ClickHouse credentials** are currently hardcoded as `harshafaik:123` across the `docker-compose.yml`. This security vulnerability prevents the stack from being used in shared or public environments. These credentials must be moved to an `.env` file and Docker Secrets used to manage sensitive information more securely.
