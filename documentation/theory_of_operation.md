# Theory of Operation

This document explains the underlying philosophy, architecture, and logic of the RiskFabric simulation. It answers the question: "How does the engine actually think?"

## 1. Agent-Based Simulation (ABM) Philosophy
RiskFabric functions as an **Agent-Based Simulator** rather than a simple random data generator. 

- **The Agent**: The primary agent, the `Customer`, drives the logic.
- **The World**: **OpenStreetMap (OSM)** reference nodes (Residential and Merchant points) across India define the physical world.
- **The Rules**: Agents follow deterministic rules defined in `fraud_rules.yaml` and `transaction_config.yaml`.

Unlike statistical generators that sample from distributions to create flat tables, RiskFabric simulates the **lifecycle** of financial entities.

---

## 2. The Deterministic Lifecycle
To ensure consistency across 10M rows and all tables, RiskFabric follows a strict creation order:

1.  **Customer Birth**: The generator assigns each customer a name, age, and a **Home Coordinate** based on real residential OSM nodes.
2.  **Financial Anchoring**: The system assigns one or more `Accounts` to every customer.
3.  **Payment Instruments**: Accounts issue `Cards`. These cards act as "keys" for generating transaction streams.
4.  **The Spend Loop**: Each card generates transactions based on the customer's `monthly_spend` profile.

---

## 3. The "One-Pass" Parallel Architecture
Traditional simulators often use multiple passes (e.g., Pass 1: Generate legitimate data, Pass 2: Inject fraud). This approach increases latency and memory usage.

RiskFabric uses a **One-Pass Architecture** in Rust:
- **Parallelization**: The engine uses the `Rayon` library to process thousands of entities simultaneously across all CPU cores.
- **Unified Logic**: Merchant selection, amount calculation, fraud injection, and campaign coordination occur in a **single loop**.
- **Memory Efficiency**: By using "Batched Generation" (5,000 entities per cycle), the engine maintains a constant memory footprint whether generating 1M or 10M rows.

---

## 4. Spatial Realism & H3 Indexing
RiskFabric uses geographic high-fidelity. 

- **H3 Hierarchies**: The system uses Uber’s H3 hexagonal grid. When a user spends, the engine first looks for merchants within the same **H3 Resolution 5** cell (neighborhood level) as their home.
- **Local vs. Global Spend**: Legitimate transactions remain "local" (same H3 cell) approximately 98% of the time. Fraud profiles (like UPI Scams) explicitly force "Remote" coordinates to simulate offshore or cross-state attacks.

---

## 5. Statistical Reproducibility (Seeded PRNG)
Every card in the system has a **Deterministic Seed**. 

```rust
let mut card_rng = StdRng::seed_from_u64(global_seed + salt + card_id_hash);
```

Running the simulation with the same `global_seed` ensures every transaction for a given card remains identical. This enables **Machine Learning reproducibility**, allowing for feature adjustments without the underlying ground-truth shifting.

---

## 6. Simulated Imperfection (Label Noise)
To mirror real-world banking challenges, RiskFabric implements **Noisy Labeling**:
- **Ground Truth (`fraud_target`)**: The latent indicator of whether the generator injected a specific fraud pattern.
- **Noisy Label (`is_fraud`)**: The signal typically available to a bank's operational systems. It includes False Positives (legitimate transactions flagged as fraud) and False Negatives (undetected fraudulent transactions).

This design forces models to learn robustness and generalizable patterns rather than memorizing perfect synthetic signatures.

---

## 7. Hybrid Streaming & Verification Architecture
To support real-time fraud detection, RiskFabric includes a dedicated **Streaming Generator** that bridges the gap between static datasets and live production environments.

- **One-Pass Consistency**: The streaming engine reuses the exact same logic as the batch pipeline but operates on a continuous loop, producing transactions at a configurable rate (default 100 tx/s).
- **Type-Level Safety (Unlabeled Output)**: To prevent "label leakage" during live scoring, the system uses a specialized `UnlabeledTransaction` struct. This mirrors the standard transaction but programmatically omits all ground-truth and labeling fields (`is_fraud`, `chargeback`, etc.), ensuring the Kafka payload is consistent with a real production stream.
- **Verification Mode**: While in verification mode, the generator writes the "Ground Truth" of every streaming transaction to `ground_truth.csv`. This allows for a post-hoc join against real-time model scores to measure precision and recall in a simulated production environment.
- **Self-Correcting Rate Limiter**: The generator measures actual Kafka broker latency for every message sent. It dynamically adjusts its sleep interval to compensate for network jitter, ensuring steady, drift-free throughput over long durations.
