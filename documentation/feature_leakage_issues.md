# Feature Leakage Case Study

While designing a fraud detection model for flagging potentially fraudulent transactions using XGBoost, some problems were discovered that made it unsuitable for being used in real-time scoring. The issue revolves around an individual feature primarily deciding the model's decision making.

Transactions were synthetically generated for 10,000 customers with a total figure being around 6M transactions. After feature engineering and interpolating them into a gold_master_table which is used by XGBoost for training. An AUC score of 0.9079 was achieved which felt more realistic than the previous test which ran on a smaller dataset (4.3M transactions and achieved an AUC score of 0.97). However, the crux of the issue became apparent when the top features by importance were checked:

| Features (AUC)                | 0.9079 |
| ----------------------------- | ------ |
| amount                        | 0.9632 |
| escalating_amounts_flag       | 0.0093 |
| cf_night_tx_ratio             | 0.0049 |
| transaction_sequence_number   | 0.0048 |
| rapid_fire_escalation_flag    | 0.0048 |
| time_since_last_transaction   | 0.0042 |
| transaction_channel           | 0.0039 |
| merchant_category_switch_flag | 0.0027 |
| t.merchant_category           | 0.0021 |
| card_present                  | 0      |

What this means is that if this model identifies a suspicious transaction amount, there is a high probability it will flag the transaction as fraud without considering other characteristics. This is sub-optimal, as the model should evaluate multiple constraints such as temporal factors and geographic behavior. To address this, two strategies can be evaluated: the removal of amount as a training feature to test performance on purely behavioral flags, or the use of feature binning to reduce reliance on exact values. After testing the second option and seeing negligible difference in feature importance, the first option was evaluated, which revealed the underlying issue within the system. After removing amount as a feature and executing the training script again, a ROC score of 0.5868 was achieved—a significant decrease from the previous result—but the resulting feature importance distribution was more revealing:

| Features (AUC)                | 0.5868 |
| ----------------------------- | ------ |
| escalating_amounts_flag       | 0.8865 |
| transaction_channel           | 0.0227 |
| time_since_last_transaction   | 0.0208 |
| transaction_sequence_number   | 0.0203 |
| cf_night_tx_ratio             | 0.0194 |
| rapid_fire_escalation_flag    | 0.0172 |
| t.merchant_category           | 0.0076 |
| merchant_category_switch_flag | 0.0057 |

This indicates that the behavioral features designed have very little predictive power to be captured by the model. Essentially, the model can't distinguish them from normal variation. Re-tuning the fraud generator is required to create distinctive behavioral signals and ensure those features are engineered effectively and processed through to the gold table. This ensures each fraud signature has its distinctive characteristics so that it can be captured by the model.

Analysis of the pipeline identified a significant gap: The behavioral features engineered in Rust were being dropped before reaching the XGBoost: 

1. The "Silently Dropped" Features In src/etl/features/sequence.rs, high-value signals are calculated that would likely address the 0.58 AUC problem, but they are missing from the ClickHouse tables: 
   * amount_deviation_z_score: This measures if ₹5,000 is "normal" for that specific customer. Without this, the model only sees the absolute ₹5,000 and assumes it's fraud because the average transaction is ₹500. 
   * fraud_type & campaign_id: These are currently calculated but not stored in the Silver layer. 
   * Granular Anomalies: geo_anomaly, device_anomaly, and ip_anomaly are being calculated in the transformation but aren't being selected in the final Gold table join. 

2. The Gold Table Join is too narrow The run_gold_master function in src/bin/etl.rs only pulls a small subset of columns from the Silver tables. It's ignoring the very features needed to replace the "Amount Shortcut." 
   
The Plan to Fix It: 

Step 1: Repair the ETL Pipeline (The "Plumbing" fix) * Update the CREATE TABLE statement for fact_transactions_silver to include the missing behavioral columns. 
* Update the run_gold_master query to pull these features into the final training set. 
* Outcome: The model will finally "see" the Z-Score and the behavioral context. 

Step 2: Re-tune the Generator (The "Signal" fix) * The `fraud_rules.yaml` configuration is modified to ensure fraudulent transaction amounts overlap with legitimate amounts. * Outcome: The model will be forced to stop using "High Amount" as a shortcut and start using the "Z-Score" and "Velocity" features fixed in Step 1.

| Features (AUC)                | 0.8246 |
| ----------------------------- | ------ |
| amount_deviation_z_score      | 0.9069 |
| escalating_amounts_flag       | 0.0468 |
| transaction_sequence_number   | 0.0122 |
| cf_night_tx_ratio             | 0.0121 |
| time_since_last_transaction   | 0.0075 |
| transaction_channel           | 0.0047 |
| rapid_fire_transaction_flag   | 0.0033 |
| merchant_category_switch_flag | 0.0033 |
| t.merchant_category           | 0.0033 |
| card_present                  | 0.0000 |

The score improved to 0.8246 just by fixing the plumbing. No new features, no generator retuning, no architectural changes. The signals were there the whole time. However, `amount_deviation_z_score` at 90% is the new dominant feature. It is better than raw `amount` — customer-relative making it more meaningful but still a single feature carrying almost everything.

The generator needs to be retuned to overlap fraud and legitimate amount distributions. Force fraudsters to transact at amounts that are normal for that customer — the Z-score becomes less dominant, behavioral features have to carry more weight.

When fraud amounts overlap with legitimate amounts, the model must rely on:

- `cf_night_tx_ratio` — when does this customer normally transact?
- `rapid_fire_transaction_flag` — velocity anomaly
- `merchant_category_switch_flag` — behavioral deviation
- `time_since_last_transaction` — timing patterns

| Features (AUC)                | 0.7960 |
| ----------------------------- | ------ |
| amount_deviation_z_score      | 0.9337 |
| escalating_amounts_flag       | 0.0229 |
| cf_night_tx_ratio             | 0.0108 |
| transaction_sequence_number   | 0.0103 |
| time_since_last_transaction   | 0.0071 |
| transaction_channel           | 0.0060 |
| rapid_fire_transaction_flag   | 0.0044 |
| t.merchant_category           | 0.0026 |
| merchant_category_switch_flag | 0.0023 |
| card_present                  | 0.0000 |

After retuning the generator to create more overlap and amplify behavioral signals, the score dropped slightly to 0.7960, fraud amounts now blend into legitimate ranges, so `amount_deviation_z_score` has less to work with. The model is being forced away from the amount shortcut. AUC dropped because the problem genuinely got harder.

However, `amount_deviation_z_score` is still at 93% despite the overlap. This means the Z-score is still capturing enough separation between fraud and legitimate amounts to dominate. The overlap wasn't aggressive enough. The problem appears to be fraud amounts as they are mostly specific values — ₹5000, ₹8500, ₹12000. Legitimate transactions cluster around ₹660. Due to this, the Z-score still sees as "this customer normally spends ₹660, this transaction is ₹8500 — suspicious." The relative deviation is still huge.

The Z-score only becomes less dominant when fraudsters transact at amounts that are normal _for that specific customer_. This requires the generator to look up the customer's `monthly_spend` and generate fraud amounts within their normal range:

```
account_takeover:
amount_strategy: "customer_normal_range"  # instead of fixed high_value_amounts
amount_multiplier: 0.8_to_1.2  # within customer's normal band
```

This makes fraud amounts become customer-relative rather than absolute. In this iteration, the Z-score was removed and the model was trained without `amount_deviation_z_score` and `escalating_amounts_flag` to evaluate the strength of the behavioral signals.

| Features (AUC)                | 0.6704 |
| ----------------------------- | ------ |
| amount_deviation_z_score      | 0.9101 |
| transaction_sequence_number   | 0.0188 |
| cf_night_tx_ratio             | 0.0165 |
| escalating_amounts_flag       | 0.0152 |
| time_since_last_transaction   | 0.0121 |
| transaction_channel           | 0.0117 |
| rapid_fire_transaction_flag   | 0.0067 |
| t.merchant_category           | 0.0045 |
| merchant_category_switch_flag | 0.0044 |
| card_present                  | 0.0000 |

Removing `escalating_amounts_flag` dropped AUC to 0.6704 but `amount_deviation_z_score` remains at 91%.

Every amount-derived feature removed makes the Z-score more dominant. The model is completely anchored to amount-relative signals. The behavioral features — `cf_night_tx_ratio`, `rapid_fire_transaction_flag`, `merchant_category_switch_flag` — collectively contribute approximately 5-6% of decisions.

Removing the Z-score and executing the training once more provides the definitive test. The resulting AUC represents the pure behavioral signal floor — no amount, no Z-score, no escalating amounts. Just time, velocity, channel, merchant, sequence.

| Features (AUC)                | 0.5572 |
| ----------------------------- | ------ |
| transaction_channel           | 0.1691 |
| escalating_amounts_flag       | 0.1677 |
| time_since_last_transaction   | 0.1644 |
| transaction_sequence_number   | 0.1602 |
| cf_night_tx_ratio             | 0.1532 |
| rapid_fire_transaction_flag   | 0.0851 |
| t.merchant_category           | 0.0679 |
| merchant_category_switch_flag | 0.0325 |

The score dropped to 0.5572, and for the first time, feature importance is evenly distributed. No single feature exceeds 17%, with every behavioral signal contributing. This structure represents a balanced feature set.

The challenge is that none of these features possess sufficient signal strength to detect fraud reliably. The model is unable to distinguish fraud because fraudsters in the simulation behave too similarly to legitimate customers. For instance:

- `transaction_channel` at 17% — channel bias exists but is weak.
- `cf_night_tx_ratio` at 15% — night patterns exist but fraud is not concentrated enough at night to be distinctive.
- `rapid_fire_transaction_flag` at 8.5% — velocity fraud occurs but not with sufficient frequency.
- `merchant_category_switch_flag` at 3.25% — almost no signal. Fraudsters shop at similar merchants as legitimate customers.

To address this at the root level, the logic responsible for injecting fraudulent signatures and behaviors requires refinement to increase signal strength for training a behaviorally-driven fraud model.

In the RiskFabric project, `fraud.rs` is primarily responsible for injecting fraud labels and altering transaction behavior according to the fraud signature. Two configurations drive transaction behavior: `geo_anomaly_prob` and `device_anomaly_prob`. Inspection of `geo_anomaly_prob` identified significant limitations:

If a transaction has the `geo_anomaly` flag set to true, its coordinates are randomized from the global range. While this creates an anomaly, it does not provide a behavioral signal that the model can learn without access to the customer's "Home" coordinates or a feature like "Distance from Home." Consequently, the model only evaluates `final_lat` and `final_lon`. Since legitimate transactions are also distributed across India (clustered around specific homes), a random coordinate appears normal to a model lacking home location context.

To resolve this, a new feature, **Spatial Velocity**, was introduced in the ETL layer. This measures: `distance(txn_N, txn_N-1) / time(txn_N, txn_N-1)`, enabling the model to identify high-velocity spatial anomalies, such as transactions occurring in distant cities within short time intervals.

| Features (AUC)                | 0.6868 |
| ----------------------------- | ------ |
| spatial_velocity              | 0.6126 |
| escalating_amounts_flag       | 0.1847 |
| time_since_last_transaction   | 0.0652 |
| merchant_category_switch_flag | 0.0643 |
| t.merchant_category           | 0.0167 |
| transaction_sequence_number   | 0.0148 |
| rapid_fire_transaction_flag   | 0.0145 |
| transaction_channel           | 0.0144 |
| cf_night_tx_ratio             | 0.0128 |
| card_present                  | 0.0000 |

The AUC increased to 0.6868 from a single feature addition. `spatial_velocity` at 61% became the dominant behavioral feature—a genuine behavioral signal. This also had a cascading effect on other features:

- `merchant_category_switch_flag` increased from 3.25% → 6.43% 
- `time_since_last_transaction` changed from 16% → 6.52% 

Several issues still required attention:

- `spatial_velocity` at 61% was too dominant, capturing almost the entire `geo_anomaly` fraud signal. The implementation at the time teleported fraudsters to random coordinates, almost guaranteed to trigger the impossible travel flag.
- `cf_night_tx_ratio` decreased to 1.28%, as night behavior was not sufficiently distinctive in the generator.
- `card_present` remained at 0%, indicating CNP fraud was not being captured.

Analysis of the low `cf_night_tx_ratio` (1.28%) led to an audit of the hourly distribution, particularly under Account Takeover (ATO) fraud. While `hourly_weights` peaked in the early morning and late evening to simulate attacker activity, the "Night Ratio" was not a strong signal due to legitimate late-night spending and a lack of sharpness in the ATO peak.

This was addressed by updating `account_takeover` hourly weights to concentrate over 70% of transactions between 00:00 and 04:00. Additionally, the `hour_deviation_from_norm` feature was introduced in the ETL layer to capture temporal anomalies at the transaction level by determining the absolute deviation from a customer's average transaction hour.

| Features (AUC)                | 0.7005 |
| ----------------------------- | ------ |
| spatial_velocity              | 0.6439 |
| escalating_amounts_flag       | 0.1450 |
| merchant_category_switch_flag | 0.0664 |
| time_since_last_transaction   | 0.0620 |
| t.merchant_category           | 0.0165 |
| transaction_channel           | 0.0149 |
| rapid_fire_transaction_flag   | 0.0136 |
| hour_deviation_from_norm      | 0.0130 |
| cf_night_tx_ratio             | 0.0124 |
| transaction_sequence_number   | 0.0122 |

AUC increased to 0.7005—a small but consistent improvement. `hour_deviation_from_norm` appeared at 1.3%, registering as a signal. `cf_night_tx_ratio` remained at 1.24%, and `escalating_amounts_flag` decreased from 18% → 14.5%, indicating behavioral features were gradually gaining influence.

Despite this, night-based features contributed only ~2.5% combined. The sharpened hourly weights provided marginal benefit, but `cf_night_tx_ratio` dilution persisted—a small number of ATO transactions does not significantly shift a customer-level ratio.

A higher impact correction involved `card_present`, which was at 0%. Correcting the 'wiring' for this feature was identified as a high-impact fix, as CNP transactions are by definition not `card_present`.

| Features (AUC)                | 0.7500 |
| ----------------------------- | ------ |
| amount_deviation_z_score      | 0.4973 |
| spatial_velocity              | 0.3402 |
| merchant_category_switch_flag | 0.0497 |
| escalating_amounts_flag       | 0.0299 |
| time_since_last_transaction   | 0.0289 |
| transaction_sequence_number   | 0.0126 |
| cf_night_tx_ratio             | 0.0117 |
| t.merchant_category           | 0.0091 |
| transaction_channel           | 0.0082 |
| hour_deviation_from_norm      | 0.0073 |

After restoring the Z-score as a feature, its dominance remained strong but was lower than in previous instances, supplemented by `spatial_velocity`.

| Features (AUC)                | 0.7491 |
| ----------------------------- | ------ |
| amount_deviation_z_score      | 0.5038 |
| spatial_velocity              | 0.3026 |
| card_present                  | 0.0499 |
| merchant_category_switch_flag | 0.0445 |
| time_since_last_transaction   | 0.0271 |
| escalating_amounts_flag       | 0.0186 |
| cf_night_tx_ratio             | 0.0117 |
| transaction_sequence_number   | 0.0114 |
| transaction_channel           | 0.0085 |
| t.merchant_category           | 0.0083 |

After correcting the 'CNP' wiring, it increased to 5%. However, `rapid_fire_transaction_flag` disappeared from the top 10 features. Analysis of the code revealed that this flag utilized a 300-second (5-minute) threshold, and `max_interval_seconds` for velocity abuse was set to a random minute within an hour, which was too coarse for signatures depending on second-level timing.

A more realistic temporal pattern for fraud bursts was implemented, ensuring transactions occur in tighter sequences (e.g., seconds apart) via `max_burst_interval_seconds`. This creates a sharper behavioral signal for the `rapid_fire_transaction_flag` to capture.

| Features (AUC)                | 0.8085 |
| ----------------------------- | ------ |
| time_since_last_transaction   | 0.3280 |
| rapid_fire_transaction_flag   | 0.2707 |
| amount_deviation_z_score      | 0.2153 |
| spatial_velocity              | 0.0727 |
| card_present                  | 0.0614 |
| merchant_category_switch_flag | 0.0213 |
| escalating_amounts_flag       | 0.0105 |
| transaction_sequence_number   | 0.0049 |
| hour_deviation_from_norm      | 0.0045 |
| transaction_channel           | 0.0042 |

Amount-based features are now in third place at 21%. Temporal behavioral signals—`time_since_last_transaction` and `rapid_fire_transaction_flag`—together drive 60% of model decisions. This allows for fraud detection based on behavioral patterns rather than absolute cost, providing logic suitable for real-time scoring of velocity abuse, ATO, and CNP fraud without flagging high-value legitimate transactions.

`merchant_category_switch_flag` at 2.1% and `hour_deviation_from_norm` at 0.45% both have potential for growth through future cross-card coordination work.
