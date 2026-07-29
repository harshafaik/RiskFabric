# Infrastructure & Local Service Stack

## Overview

The local service stack (`docker-compose.yml`) provisions 7 services on `riskfabric_default`: ClickHouse, Redpanda, Redis, OLTP Postgres, scorer (`scorer.py`), Grafana, and Django case-admin. Mandatory runtime dependency for all Rust binaries and Python ML services. A separate Postgres instance (not in this compose file) handles world-building for `prepare_refs.rs` and `export_references.rs`.

## Schema

<div style="max-width: 400px; margin: 0 auto;">

```mermaid
flowchart TB
    classDef script fill:#22252a,stroke:#4d535b,stroke-width:1px,rx:5px,ry:5px,color:#cfd2d9;
    classDef store fill:#1b2a3a,stroke:#304e70,stroke-width:1px,rx:5px,ry:5px,color:#cfd2d9;
    classDef ui fill:#251e36,stroke:#483a68,stroke-width:1px,rx:5px,ry:5px,color:#cfd2d9;
    subgraph Host["Host Machine"]
        STREAM["stream.rs\n(host binary)"]:::script
    end
    subgraph Stack["Docker Compose (riskfabric_default)"]
        RP[(Redpanda\n:9092 :29092)]:::store
        RD[(Redis\n:6379)]:::store
        CH[(ClickHouse\n:8123 :9000)]:::store
        PG[(oltp-postgres\n:5432)]:::store
        scorer[scorer.py]:::script
        grafana[Grafana\n:3000]:::ui
        caseadmin[case_admin\nDjango :8000]:::ui
    end
    STREAM -->|"localhost:9092"| RP
    RP -->|"raw_transactions"| scorer
    scorer -->|"fraud_scores"| CH
    scorer -->|"cases"| caseadmin
    caseadmin --> PG
    RD <-.->|"feature state"| scorer
    CH --> grafana

    style Host fill:#1e232e,stroke:#333e54,stroke-width:1px,stroke-dasharray: 3 3,color:#cfd2d9
    style Stack fill:#1c241e,stroke:#304033,stroke-width:1px,stroke-dasharray: 3 3,color:#cfd2d9
```
</div>

**<a id="fig-8"></a>Figure 8:** Infrastructure Service Stack

| Service | Image | Ports | Persistence |
| :--- | :--- | :--- | :--- |
| `clickhouse` | `clickhouse/clickhouse-server:latest` | `8123`, `9000` | `clickhouse_data`, `clickhouse_logs` |
| `redpanda` | `redpandadata/redpanda:v23.2.1` | `9092` (ext), `29092` (int) | None (ephemeral) |
| `redis` | `redis:7-alpine` | `6379` | None (ephemeral) |
| `scorer` | `localhost/riskfabric_dlt` | — | Workspace mount `/usr/app` |
| `grafana` | `grafana/grafana-oss:latest` | `3000` | `grafana_data` |
| `oltp-postgres` | `postgres:15-alpine` | `5433`→host | `oltp_postgres_data` |
| `case-admin` | built from `case_admin/Dockerfile` | `8001`→host | `case_admin/` mount |

## Architecture

### Multi-Model Database Strategy
ClickHouse for append-heavy analytical queries (fraud scores), Redpanda for Kafka-compatible streaming, Redis for sub-millisecond per-card/customer state (verified: p50 120–261 µs, p99 ≤509 µs across all operations) [[See benchmarks](performance.md)], Postgres for OLTP case management.

### Health-Check Gating
All data services health-checked (ClickHouse via `wget`, Redpanda via `rpk`, Redis via `redis-cli`, Postgres via `pg_isready`). `scorer`, `grafana`, and `case-admin` wait for `service_healthy` before startup. 5s interval, 5 retries.

### Dual Kafka Listener
Redpanda exposes port `29092` for container-to-container traffic and `9092` for host access (`stream.rs`). `scorer` connects via `KAFKA_BOOTSTRAP_SERVERS=redpanda:29092`.

### Workspace Volume Mounts
`scorer` mounts entire project at `/usr/app`, `case-admin` mounts `case_admin/` at `/usr/app`. Source changes take effect on container restart without image rebuild. The scorer image must be built from `Dockerfile.dlt` before first run.

## Current Limitations

Redpanda is ephemeral — all topic data lost on restart, preventing replay testing. Passwords hardcoded in `docker-compose.yml` and repeated across Python scripts with no `.env` support. No `build:` directive for the scorer image.
