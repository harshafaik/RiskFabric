# Fraud Signatures & Attack Patterns

## Overview

The `fraud_signatures.md` document serves as the high-level behavioral specification for RiskFabric's synthetic adversary. It defines both individual fraud profiles and coordinated multi-entity attack campaigns executed during data generation in `transaction_gen.rs`, providing the theoretical and structural basis for synthetic financial anomalies.

## Design Intent

These attack signatures move beyond unstructured random noise toward **Structured Adversarial Intelligence**. Each individual profile (e.g., `UPI Scam`, `Account Takeover`) anchors in a specific real-world financial threat vector observed in payment systems. By layering **Campaign Logic** on top of individual profiles, the simulation models the synchronized, multi-entity signals characteristic of organized criminal rings.

A critical design choice is **Probabilistic Mutation**. Rather than generating deterministic, flagrant outliers, configuration-driven probabilities in `fraud_tuning.yaml` ensure that a portion of fraudulent activity shares behavioral overlap with legitimate transactions (e.g., Friendly Fraud). This forces downstream machine learning models to learn high-dimensional behavioral decision boundaries rather than relying on trivial, static rule thresholds.

---

## 1. Fraud Profiles (Individual Patterns)

| Profile | Behavioral Signature | Spatial Signature | Primary Channels |
| :--- | :--- | :--- | :--- |
| **UPI Scam** | Social engineering; high-frequency transfers of ₹1,500–₹20,000 using customer-normal or elevated multipliers. | **90% Geo-Anomaly**: Transactions originated from remote scammer locations. | UPI (`80%`) |
| **Account Takeover** | High-value credential hijacking concentrated in nocturnal hours (00:00–04:00). | **60% Geo-Anomaly**: Compromised access from distinct geographic locations. | Mobile Banking (`40%`), UPI (`30%`), Online (`30%`) |
| **Velocity Abuse** | Automated rapid-fire testing using small fixed probing amounts (₹1.01, ₹1.23, ₹2.05). | **20% Geo-Anomaly**: Low spatial deviation to evade location-based rules. | UPI (`70%`), Online (`30%`) |
| **Card Not Present** | Unauthorized e-commerce authorization using stolen card credentials for liquid goods. | **40% Geo-Anomaly**: Remote IP and location signatures. | Online (`70%`), Cards (`30%`) |
| **Friendly Fraud** | First-party fraud where a legitimate customer falsely disputes valid purchases. | **0% Geo-Anomaly**: Originated from customer's home location and standard device. | Cards (`60%`), UPI (`40%`) |

---

## 2. Campaign Attack Patterns (Coordinated)

In addition to standalone individual fraud patterns, RiskFabric supports multi-transaction coordinated attack structures injected across groups of cards or over time.

### Coordinated Attack
*   **Signal**: Multiple distinct cards and customer entities targeted simultaneously by a single adversarial operator.
*   **Hard Correlation**: Every transaction belonging to the campaign shares the **exact same IP Address** and **geographic coordinate**, simulating a scammer call center, proxy pool, or centralized botnet hub.
*   **Configuration**: The target IP address is configurable via `fraud_tuning.yaml` (`coordinated_scam_ip`, defaulting to `103.21.244.12`).

### Sequential Takeover
*   **Signal**: A single card experiencing a progressive escalation of fraud over multiple successive transactions.
*   **Monotonic Escalation**: Each subsequent transaction amount escalates systematically according to the `ato_escalation_rate` (defaulting to a 25%–30% increase per transaction).
*   **Persistent Location**: Once the takeover sequence initiates, the geographic coordinate "sticks" to the attacker's location across all subsequent steps in the sequence.

---

## Known Issues

Spatial signatures for fraud are currently implemented as instantaneous latitude/longitude coordinate jumps. While this creates a distinct spatial anomaly, the generator lacks a dedicated **Traveling Customer Model** for legitimate users. As a result, legitimate travel can produce spatial velocity spikes that mimic fraud, leading to elevated false-positive rates in baseline spatial anomaly models.

Campaign logic is presently restricted to shared network IP addresses and shared geographic coordinates. The simulation engine does not yet implement **Account-to-Account (A2A) Graph Signals** or multi-hop money laundering networks (mule account chains). Introducing explicit destination account entities and graph topology structures in `transaction_gen.rs` is required to support graph neural network (GNN) and flow-based fraud detection research.
