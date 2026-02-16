# Fraud Signatures & Attack Patterns

This document details the high-fidelity fraud behaviors and coordinated attack campaigns implemented in the `riskfabric` generator.

## 1. Fraud Profiles (Individual Patterns)

| Profile | Behavioral Signature | Spatial Signature |
| :--- | :--- | :--- |
| **UPI Scam** | High frequency, small to medium amounts (₹1,500 - ₹20,000). | **90% Geo-Anomaly**: Scammer is remote. |
| **Account Takeover** | High-value transfers, sudden change in device/channel. | **40% Geo-Anomaly**: Compromised from distant location. |
| **Velocity Abuse** | Rapid-fire "testing" transactions (₹1.01, ₹1.23, etc.). | **10% Geo-Anomaly**: Low spatial signal. |
| **Card Not Present** | Online-only channel bias, standard e-commerce amounts. | **30% Geo-Anomaly**: Card details used remotely. |
| **Friendly Fraud** | Legitimate channel/device, standard amounts. | **0% Geo-Anomaly**: Customer is physically at home. |

## 2. Campaign Attack Patterns (Coordinated)

### Coordinated Attack
*   **Signal**: Multiple distinct cards/customers targeted simultaneously by a single entity.
*   **Hard Correlation**: Every transaction in the campaign shares the **exact same IP Address** and **geographic coordinate** (simulating a scammer hub or proxy).
*   **Tuning**: Coordinated IP is configurable via `fraud_tuning.yaml` (Default: `103.21.244.12`).

### Sequential Takeover
*   **Signal**: A single card experiencing a progressive escalation of fraud.
*   **Monotonic Escalation**: Each subsequent transaction amount is multiplied by the `ato_escalation_rate` (Default: 30%).
*   **Persistent Location**: Once the takeover begins, the geographic coordinate "sticks" to the attacker's location for the remainder of the sequence.

### Burst Campaign
*   **Signal**: A high-frequency cluster of fraudulent transactions over a short window.
*   **Noise**: High volume relative to the individual customer's baseline.

## 3. Hybrid Campaigns (Combination)

**Are Hybrid Campaigns present? Yes.**

The current "One-Pass" architecture naturally implements hybrid campaigns. When a card is targeted by a campaign, it first selects a **Base Fraud Profile** (e.g., `upi_scam` or `account_takeover`) and then overlays the **Campaign Mutation** (e.g., `coordinated_attack`).

**Example: Coordinated UPI Scam**
1.  **Profile Layer**: Pick amounts from `upi_common_amounts`.
2.  **Campaign Layer**: Force all transactions to share the `103.21.244.12` IP and a single "Scammer Hub" coordinate.
3.  **Result**: An extremely sharp, multi-dimensional signal for ML models to learn both individual behavior and network-level coordination.
