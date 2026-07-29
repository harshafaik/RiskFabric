# Deployment Architecture: Local Development and AWS

## Decision

Run RiskFabric on a single EC2 instance via Docker Compose, with RDS for Postgres and S3 for Parquet snapshots as the only managed services. Do not use managed Kafka (MSK), managed Redis (ElastiCache), or managed ClickHouse (ClickHouse Cloud) at the project's current throughput. Keep the architecture cloud-portable — Docker Compose and application code are cloud-agnostic; only Terraform and the two managed services are AWS-coupled.

## Local development

The local Docker Compose stack is documented in detail in [Infrastructure & Local Service Stack](../components/infrastructure.md). Key additions to note for deployment:

### Services (all in Docker Compose)

| Container | Image | Port | Purpose |
|-----------|-------|------|---------|
| `redpanda` | `redpandadata/redpanda:v23.2.1` | 9092 | Kafka-compatible streaming broker |
| `redis` | `redis:7-alpine` | 6379 | Real-time feature cache |
| `clickhouse` | `clickhouse/clickhouse-server:latest` | 8123, 9000 | Streaming scores + dashboard queries |
| `scorer` | `localhost/riskfabric_dlt` | — | ML scorer: Kafka → Redis → XGBoost → ClickHouse |
| `grafana` | `grafana/grafana-oss:latest` | 3000 | Grafana real-time monitoring |
| `oltp-postgres` | `postgres:15-alpine` | 5433→host | Case management (Django) + PostGIS |
| `case-admin` | built from `case_admin/Dockerfile` | 8001→host | Django admin (Gunicorn) |

### Host commands

```bash
# Start infrastructure
docker compose up -d

# Batch pipeline
cargo run --bin generate
cargo run --bin etl          # Polars → Parquet, no ClickHouse
python src/ml/train_xgboost.py  # DuckDB reads Gold Parquet

# Streaming
cargo run --bin stream        # Produces to localhost:9092
```

### Local costs

$0. All services run on the developer's machine. No cloud resources consumed during development.

---

## AWS deployment

A single EC2 instance runs Docker Compose for the streaming stack. RDS hosts Postgres with three schemas (`location`, `case_state`, `training`). S3 stores Bronze/Silver/Gold Parquet snapshots with lifecycle policies.

```
┌──────────────────────────────────────────────────────────────┐
│  AWS (us-east-1)                                             │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐     │
│  │  VPC (10.0.0.0/16)                                  │     │
│  │  Public Subnet (10.0.0.0/24)                        │     │
│  │                                                     │     │
│  │  ┌──────────────────────────────────────┐           │     │
│  │  │  EC2 t3.small (Ubuntu 24.04)         │           │     │
│  │  │  Elastic IP                          │           │     │
│  │  │                                      │           │     │
│  │  │  Docker Compose (via Podman):        │           │     │
│  │  │  ┌──────────┐ ┌───────┐ ┌─────────┐ │           │     │
│  │  │  │ Redpanda │ │ Redis │ │ClickHouse│ │           │     │
│  │  │  │ :9092 SIP│ │ :6379 │ │  :8123  │ │           │     │
│  │  │  └────┬─────┘ └──┬────┘ └────┬────┘ │           │     │
│  │  │       │          │           │      │           │     │
│  │  │  ┌────▼────┐ ┌───▼───┐ ┌────▼────┐ │           │     │
│  │  │  │ scorer  │ │ Redis │ │ grafana │ │           │     │
│  │  │  │(Python) │ │(cache)│ │  :3000🌐│ │           │     │
│  │  │  └─────────┘ └───────┘ └─────────┘ │           │     │
│  │  │                                      │           │     │
│  │  │  ┌───────────────┐                   │           │     │
│  │  │  │  case-admin   │                   │           │     │
│  │  │  │  :8001 SIP    │                   │           │     │
│  │  │  └───────┬───────┘                   │           │     │
│  │  └──────────┼───────────────────────────┘           │     │
│  │             │                                       │     │
│  │  ┌──────────▼──────────┐   ┌──────────────────┐     │     │
│  │  │  Security Group     │   │  RDS PostgreSQL   │     │     │
│  │  │  :22    → my_ip     │   │  db.t4g.micro     │     │     │
│  │  │  :9092  → my_ip     │   │  Schemas:         │     │     │
│  │  │    :3000  → 0.0.0.0/0 │   │   location        │     │     │
│  │  │  :8001  → my_ip     │   │   case_state      │     │     │
│  │  └─────────────────────┘   │   training        │     │     │
│  │                            └───────────────────┘     │     │
│  └──────────────────────────────────────────────────────┘     │
│                                                              │
│  ┌──────────────────────────────────────────────────┐        │
│  │  S3 Bucket (riskfabric-data)                     │        │
│  │                                                  │        │
│  │  bronze/{date}/*.parquet    (Standard tier)      │        │
│  │  silver/{date}/*.parquet    (Standard tier)      │        │
│  │  gold/{snapshot}/*.parquet  (Standard → IA)      │        │
│  │  models/*.json              (Standard)           │        │
│  │                                                  │        │
│  │  Lifecycle rules:                                │        │
│  │   bronze: → IA after 7d, → Glacier after 30d    │        │
│  │   silver: → IA after 14d, → Glacier after 60d   │        │
│  │   gold:   → IA after 30d, → Glacier after 90d   │        │
│  └──────────────────────────────────────────────────┘        │
└──────────────────────────────────────────────────────────────┘

Local dev machine:
  stream.rs ──produces──▶ <EIP>:9092
  generate.rs ──writes──▶ S3 (or runs on EC2)
  train_xgboost.py ──reads──▶ S3 Gold Parquet via DuckDB
```

### Terraform-provisioned resources

| Resource | Type | Purpose |
|----------|------|---------|
| VPC + subnet + IGW | Networking | Isolated network with public internet access |
| EC2 `t3.small` | Compute | Docker Compose host (Podman) |
| Elastic IP | Networking | Static public IP for stream producer + dashboard |
| Security group | Networking | SSH + Redpanda restricted to operator IP; dashboard public |
| RDS `db.t4g.micro` | Database | Managed Postgres (20GB gp3) |
| S3 bucket | Storage | Parquet snapshots with lifecycle policies |

### Why RDS but not ElastiCache or MSK

**RDS for Postgres ($16/mo)** provides automated backups, point-in-time recovery, minor version patching, and multi-AZ failover. Postgres is the system's transactional backbone — case state, investigator notes, and training metadata are records that must not be lost. At $16/month for a `db.t4g.micro`, managed Postgres is cheaper than the operational cost of self-managing backups and recovery on the EC2 instance.

**Redis on EC2** provides sub-millisecond lookups for running statistical aggregates (Welford mean/std, card velocity) — verified at p50: 120–261 µs across all 13 operations, max p99: 509 µs. [[See benchmarks](performance.md)] Redis data is cacheable and recomputable — it can be lost and rebuilt from Parquet snapshots. ElastiCache starts at ~$12/month for a `t3.micro` and adds no value over the Redis container already running in Docker Compose. The container uses <100MB RAM at project throughput.

**Redpanda on EC2** provides Kafka-compatible streaming at 100 tx/s. MSK serverless has a minimum base charge of $0.75/hour ($540/month) before a single message is sent. At the project's throughput, this is a 27× cost multiplier over self-hosting with no architectural benefit. Redpanda runs in a single container with negligible resource consumption.

### Monthly cost breakdown

| Resource | Specification | Monthly Cost |
|----------|--------------|-------------|
| EC2 `t3.small` | 2 vCPU, 2GB RAM, 20GB EBS | ~$16.50 |
| Elastic IP | Static public IP (attached to running instance) | $0 |
| RDS `db.t4g.micro` | 2 vCPU, 1GB RAM, 20GB gp3 | ~$16.00 |
| S3 Standard | ~5GB Bronze + Silver + Gold Parquet | ~$0.12 |
| S3 IA / Glacier | Older snapshots (lifecycle transitions) | ~$0.05 |
| Data transfer | Outbound to operator (dashboard, SSH) | ~$1–3 |
| **Total** | | **~$34–37/month** |

Data transfer is the only variable cost — the dashboard is publicly accessible on port 3000. A moderate amount of portfolio browsing traffic keeps this negligible.

### Cost tiering for interviews and demos

| Scenario | Monthly Cost | Notes |
|----------|-------------|-------|
| **Idle** (EC2 stopped, RDS stopped) | ~$0.12 | S3 storage only |
| **Development** (EC2 running, containers up, no stream) | ~$32.50 | EC2 + RDS running |
| **Demo** (full stack, stream producing, dashboard live) | ~$34–37 | EC2 + RDS + S3 + transfer |
| **Demo with managed Kafka** (MSK serverless added) | ~$574–577 | Adds $540/mo — not recommended |

For portfolio demonstrations, the recommended pattern is: run the infrastructure 1–2 hours before an interview, tear it down afterward. A 2-hour demo at full stack costs approximately $0.10.

### What is cloud-agnostic

- Docker Compose (`docker-compose.yml`) — runs identically on any VM with Docker or Podman
- All 7 container images — no cloud-specific dependencies
- All application code (Rust, Python, Django) — configured via environment variables
- Rust stream binary — connects via `KAFKA_BOOTSTRAP_SERVERS` env var
- Parquet snapshot format — portable across S3, Azure Blob, GCS, and local disk

### What is AWS-coupled (and would need rewrite for Azure/GCP)

- Terraform (`deploy/terraform/`) — 100% AWS provider (`hashicorp/aws ~> 5.0`). Rewriting for Azure requires ~180 lines of HCL using `azurerm` provider; for GCP, ~180 lines using `google` provider.
- RDS PostgreSQL — Azure equivalent is Azure Database for PostgreSQL Flexible Server; GCP equivalent is Cloud SQL. Both offer comparable `db.t4g.micro`-class instances at similar pricing.
- S3 bucket and lifecycle policies — Azure equivalent is Blob Storage with lifecycle management; GCP equivalent is Cloud Storage with object lifecycle. The Parquet format is identical across all three.

### Why not Kubernetes (EKS/AKS/GKE)

The project runs 7 containers on a single machine with no horizontal scaling requirements. Kubernetes would add operational complexity (cluster management, node groups, ingress controllers, persistent volume claims) without solving a problem the project actually has. Docker Compose on a single VM is the appropriate orchestration level for this workload — it demonstrates infrastructure-as-code (Terraform) and containerization without over-rotating into unnecessary orchestration overhead.

If the project later requires multi-node deployment (e.g., separating the streaming scorer from the dashboard for resource isolation), `docker-compose.yml` converts to Kubernetes manifests via Kompose with minimal effort. This decision defers that complexity until it is justified.
