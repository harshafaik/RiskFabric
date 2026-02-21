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
To ensure consistency across 100M rows and all tables, RiskFabric follows a strict creation order:

1.  **Customer Birth**: The generator assigns each customer a name, age, and a **Home Coordinate** based on real residential OSM nodes.
2.  **Financial Anchoring**: The system assigns one or more `Accounts` to every customer.
3.  **Payment Instruments**: Accounts issue `Cards`. These cards act as "keys" for generating transaction streams.
4.  **The Spend Loop**: Each card generates transactions based on the customer's `monthly_spend` profile.

---

## 3. The "One-Pass" Parallel Architecture
Traditional simulators often use several passes (e.g., Pass 1: Generate legit data, Pass 2: Inject fraud). This approach increases latency and memory usage.

RiskFabric uses a **One-Pass Architecture** in Rust:
- **Parallelization**: The engine uses the `Rayon` library to process thousands of entities simultaneously across all CPU cores.
- **Unified Logic**: Merchant selection, amount calculation, fraud injection, and campaign coordination all happen in a **single loop**.
- **Memory Efficiency**: By using "Batched Generation" (5,000 entities per cycle), the engine maintains a constant memory footprint whether generating 1M or 100M rows.

---

## 4. Spatial Realism & H3 Indexing
RiskFabric differentiates itself through geographic high-fidelity. 

- **H3 Hierarchies**: We use Uber’s H3 hexagonal grid. When a user spends, the engine first looks for merchants within the same **H3 Resolution 5** cell (neighborhood level) as their home.
- **Local vs. Global Spend**: Legitimate transactions remain "local" (same H3 cell) 98% of the time. Fraud profiles (like UPI Scams) explicitly force "Remote" coordinates to simulate offshore or cross-state attacks.

---

## 5. Statistical Reproducibility (Seeded PRNG)
Every card in the system has a **Deterministic Seed**. 

```rust
let mut card_rng = StdRng::seed_from_u64(global_seed + salt + card_id_hash);
```

If you run the simulation with the same `global_seed`, every transaction for `Card_ABC` remains identical. This enables **Machine Learning reproducibility**, allowing data scientists to tweak features without the underlying ground-truth shifting.

---

## 6. Simulated Imperfection (Label Noise)
To mirror real-world banking challenges, RiskFabric implements **Noisy Labeling**:
- **Ground Truth (`fraud_target`)**: The perfect indicator of whether the generator injected a fraud pattern.
- **Noisy Label (`is_fraud`)**: The signal the "Bank" actually sees. It includes False Positives (legit txns flagged as fraud) and False Negatives (fraud txns that went undetected).

This forces models to learn robustness rather than memorizing perfect patterns.
