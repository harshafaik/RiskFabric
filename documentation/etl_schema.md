# ETL & Feature Schema

## Summary
The `etl_schema.md` document defines the behavioral features and data transformations performed by the RiskFabric ETL pipeline (`etl.rs`). It acts as the technical contract for the "Silver" and "Gold" layers, detailing how raw synthetic events are transformed into the high-dimensional vectors used for model training and real-time inference.

## Design Intent
The feature schema represents a **Hybrid Behavioral State**, intended to provide models with a multi-domain view of financial events across customer history, merchant risk, and temporal sequences. This approach facilitates sophisticated behavioral modeling, such as Z-scores and velocity-based indicators, similar to production fraud detection systems.

A critical design choice was the use of **Welford's Algorithm** for statistical aggregates. Calculating running means and variances locally in Rust (and Redis) ensures that features are numerically stable and computationally efficient for both batch processing and low-latency streaming. This architectural decision is intended to eliminate training-serving skew.

---

## 🥈 Silver Layer: Behavioral Features

### Transaction Sequence Features (`fact_transactions_silver`)
Calculated at the individual card level to identify temporal and spatial anomalies.

| Field | Description | Logic |
| :--- | :--- | :--- |
| `time_since_last` | Seconds since the previous event. | `T - T_prev` |
| `spatial_velocity`| Speed (km/h) between consecutive events. | `Dist(L, L_prev) / (T - T_prev)` |
| `amount_z_score` | Deviation from customer's mean spend. | `(Amt - Mean) / StdDev` |
| `hour_deviation` | Deviation from customer's peak spend hour. | Circular variance of `timestamp.hour()` |

### Network & Entity Features (`network_features_silver`)
Identifies high-risk clusters across the payment network.

| Field | Description | Logic |
| :--- | :--- | :--- |
| `shared_ip_fraud`| Fraud rate of cards sharing the same IP. | `SUM(is_fraud) / COUNT(card_id) OVER IP` |
| `scammer_hub` | Flag for known high-risk coordinates. | `1 if Lat/Lon in [hub_coordinates]` |

---

## 🥇 Gold Layer: The Master Table
The final flattened table used for model training, joining all Silver behavioral features with the original Bronze transactions.

---

## Known Issues
**Spatial Velocity** is currently calculated using a Euclidean distance approximation. While computationally efficient, this is inaccurate over long distances. Implementation of the **Haversine formula** is required to ensure geographic precision for cross-state and international fraud simulations.

Furthermore, **Feature Freshness** is limited to the last 10 transactions in Redis. This prevents the modeling of long-term behavioral baselines for infrequent spenders. Implementing "Stateful Cold Storage" in the ETL pipeline is necessary to retrieve historical data without exceeding real-time feature store capacity.
