# Configuration Reference

## Overview

RiskFabric simulation behaviour is controlled by five YAML files in `data/config/`. These files are loaded at runtime by `config.rs` and passed into the generation pipeline without recompiling any binary. They govern population size, transaction physics, financial product definitions, fraud injection rates, and anomaly mutation probabilities.

---

## `customer_config.yaml`

Controls the synthetic population's demographics, financial heuristics, device profiles, and ISP assignments.

### `control`

| Key | Default | Description |
| :--- | :--- | :--- |
| `customer_count` | `3334` | Total number of customers to generate per batch run. |
| `transactions_per_customer.min` | `400` | Minimum number of transactions generated per customer over the lookback period. |
| `transactions_per_customer.max` | `800` | Maximum number of transactions generated per customer over the lookback period. |
| `parallelism.customer_gen_threads` | `8` | Rayon thread count for the customer generation pass. |
| `parallelism.transaction_gen_threads` | `32` | Rayon thread count for the transaction generation pass. |

### `financials`

| Key | Default | Description |
| :--- | :--- | :--- |
| `base_spend.Metro` | `35000` | Baseline monthly spend (INR) for Metro-classified customers. |
| `base_spend.Urban` | `22000` | Baseline monthly spend (INR) for Urban-classified customers. |
| `base_spend.Semi-Urban` | `15000` | Baseline monthly spend (INR) for Semi-Urban-classified customers. |
| `base_spend.Rural` | `9000` | Baseline monthly spend (INR) for Rural-classified customers. |
| `credit_score.base` | `650` | Starting credit score before age-based adjustment. |
| `credit_score.age_weight` | `1.5` | Multiplier applied to age delta when computing the credit score offset. |
| `credit_score.min` | `300` | Hard lower bound on generated credit scores. |
| `credit_score.max` | `900` | Hard upper bound on generated credit scores. |

### `registration`

| Key | Default | Description |
| :--- | :--- | :--- |
| `lookback_years` | `5` | Maximum number of years in the past a customer's registration date can fall. |
| `default_location_type` | `"Urban"` | Fallback location type when no OSM residential node is matched. |

### `device_profiles.location_shares`

Per-`location_type` share weights for device OS assignment. All shares within a location type must sum to 1.0.

| Location Type | `android_share` | `ios_share` | `upi_app_share` | `desktop_share` |
| :--- | :--- | :--- | :--- | :--- |
| `Metro` | `0.65` | `0.20` | `0.10` | `0.05` |
| `Urban` | `0.72` | `0.12` | `0.12` | `0.04` |
| `Semi-Urban` | `0.82` | `0.04` | `0.12` | `0.02` |
| `Rural` | `0.82` | `0.04` | `0.12` | `0.02` |

### `isp_assignment.shares`

Per-`location_type` share weights for ISP assignment. The `subnets` block maps each ISP to a CIDR prefix used to generate the customer's `ip_subnet` field.

| Location Type | Jio | Airtel | ACT | BSNL | Others |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `Metro` | `0.45` | `0.35` | `0.15` | — | `0.05` |
| `Urban` | `0.50` | `0.30` | — | `0.15` | `0.05` |
| `Semi-Urban` | `0.50` | `0.15` | — | `0.35` | — |
| `Rural` | `0.50` | `0.15` | — | `0.35` | — |

The `names`, `email.domains`, and `*_ua_pool` lists are static lookup pools used for random selection during customer generation. They do not require tuning under normal circumstances.

---

## `transaction_config.yaml`

Controls the transaction stream's volume, timing, geography, and merchant category universe.

| Key | Default | Description |
| :--- | :--- | :--- |
| `transactions.amount_range` | `[10.0, 50000.0]` | Min and max transaction amount in INR before fraud profile overrides are applied. |
| `transactions.success_rate` | `0.96` | Probability that a generated transaction receives `status = "Success"`. The remaining 4% are declined. |
| `transactions.card_present_probability` | `0.35` | Probability that `card_present = true` for a given transaction. |
| `transactions.lookback_days` | `365` | Number of days of transaction history generated per customer. |
| `transactions.geo_bounds.lat_range` | `[8.0, 37.0]` | Latitude bounding box for fallback transaction location generation (India). |
| `transactions.geo_bounds.long_range` | `[68.0, 97.0]` | Longitude bounding box for fallback transaction location generation (India). |

### `transactions.temporal_patterns`

Relative weights used to sample transaction timestamps. All values are relative — they do not need to sum to any fixed total.

| Parameter | Values | Description |
| :--- | :--- | :--- |
| `hourly_weights` | 24-element array, index 0 = midnight | Activity weight per hour of day. Peak at hour 19 (weight `4.0`), trough at hour 2 (weight `0.05`). |
| `daily_weights` | 7-element array, index 0 = Monday | Activity weight per day of week. Weekend peak: Saturday (`1.5`), Friday (`1.3`). |

### `transactions.merchant_categories`

17 categories are defined, each with a `name` string and a 4-digit `mcc` code. The full list:

`GROCERY (5411)`, `FOOD_AND_DRINK (5812)`, `GENERAL_RETAIL (5311)`, `SERVICES (7299)`, `ENTERTAINMENT (7999)`, `ELECTRONICS (5732)`, `TRAVEL (4722)`, `MEDICAL (8099)`, `AUTOMOTIVE (5511)`, `LUXURY (5944)`, `B2B_WHOLESALE (5099)`, `HOME_GARDEN (5200)`, `TRANSPORT (4111)`, `CHARITY (8398)`, `RETAIL (5999)`, `ALCOHOL (5921)`, `GAMBLING (7995)`.

---

## `fraud_rules.yaml`

Controls the global simulation seed, payment channel market shares, fraud injection rates, fraud profile definitions, and per-profile temporal patterns.

### `global`

| Key | Default | Description |
| :--- | :--- | :--- |
| `seed` | `5555` | Global RNG seed for reproducible generation runs. |
| `base_currency` | `"INR"` | Currency code applied to all generated transactions. |
| `default_country` | `"IN"` | Default `merchant_country` value for domestic merchants. |

### `payment_channels`

| Channel | `market_share` | `risk_level` | Description |
| :--- | :--- | :--- | :--- |
| `upi` | `0.70` | `0.7` | Dominant channel; weighted toward UPI app user agents. |
| `mobile_wallets` | `0.10` | `0.6` | Wallet apps (AmazonPay, MobiKwik, Freecharge). |
| `cards` | `0.10` | `0.9` | Highest risk level; mapped to browser user agents. |
| `online` | `0.07` | `0.8` | Web-based card-not-present transactions. |
| `mobile_banking` | `0.03` | `0.5` | Bank app transactions; lowest risk level. |

All `market_share` values must sum to 1.0.

### `fraud_injector`

| Key | Default | Description |
| :--- | :--- | :--- |
| `target_share` | `0.01` | Fraction of customers designated as active fraud targets (1%). |
| `default_fp_rate` | `0.005` | Baseline false-positive noise rate applied to legitimate transactions (0.5%). |
| `default_fn_rate` | `0.08` | Baseline false-negative noise rate applied to fraudulent transactions (8%). |

### `fraud_injector.profiles`

Five profiles are defined. Each profile controls how a fraud event is constructed for a targeted customer:

| Profile | `frequency` | `geo_anomaly_prob` | `temporal_anomaly_prob` | Primary Channels |
| :--- | :--- | :--- | :--- | :--- |
| `upi_scam` | `0.35` | `0.95` | `0.70` | `upi (0.80)` |
| `account_takeover` | `0.25` | `0.60` | `0.90` | `mobile_banking (0.40)`, `upi (0.30)`, `online (0.30)` |
| `velocity_abuse` | `0.15` | `0.20` | `0.40` | `upi (0.70)`, `online (0.30)` |
| `card_not_present` | `0.15` | `0.40` | `0.50` | `online (0.70)`, `cards (0.30)` |
| `friendly_fraud` | `0.10` | `0.00` | `0.10` | `upi (0.40)`, `cards (0.60)` |

`frequency` is a relative weight — the five values sum to 1.0 and determine which profile is assigned when a fraud event is injected.

Each profile also specifies `amount_pattern` (a key into `fraud_patterns`), `amount_multiplier` (a range applied to the customer's normal spend), and `merchant_bias` (a weighted distribution over merchant categories).

The `temporal_patterns` block provides per-profile `hourly_weights` and `daily_weights` that override the baseline temporal distribution from `transaction_config.yaml` for fraud events. `account_takeover` concentrates 70%+ of activity in the 00:00–04:00 window.

---

## `fraud_tuning.yaml`

Controls global anomaly injection probabilities and campaign parameters. Applies independently of fraud profile assignment.

### `probabilities`

| Key | Default | Description |
| :--- | :--- | :--- |
| `geo_anomaly` | `0.15` | Probability that any transaction (regardless of fraud status) has `geo_anomaly = true` injected. |
| `device_anomaly` | `0.10` | Probability that any transaction has `device_anomaly = true` injected. |
| `ip_anomaly` | `0.15` | Probability that any transaction has `ip_anomaly = true` injected. |
| `failure` | `0.15` | Probability that a fraud-targeted transaction is forced to `status = "Failed"`. |
| `chargeback` | `0.08` | Probability that a fraudulent transaction results in a chargeback. |

### `defaults`

| Key | Default | Description |
| :--- | :--- | :--- |
| `geo_anomaly_country` | `"PK"` | `merchant_country` value assigned when a geo anomaly is injected. |
| `fallback_failure_reason` | `"Security check triggered"` | Failure reason used when no profile-specific reason applies. |
| `chargeback_days` | `30` | Default `chargeback_days` value when a chargeback is generated. |

### `salts`

| Key | Default | Description |
| :--- | :--- | :--- |
| `injector` | `7` | RNG salt for the fraud injection pass. |
| `mutator` | `99` | RNG salt for the anomaly mutation pass. |
| `campaign` | `555` | RNG salt for campaign assignment. |

---

## `product_catalog.yaml`

Controls financial product parameters for account and card generation.

### `accounts`

| Key | Default | Description |
| :--- | :--- | :--- |
| `types` | `["Savings", "Current", "Credit", "Salary"]` | Valid account type values. |
| `creation_window_years` | `3` | Maximum years in the past an account's `creation_date` can fall, relative to the generation date. |
| `bank_id_range` | `[1000, 9999]` | Inclusive range for randomly generating 4-digit `bank_id` values. |
| `balance_range` | `[1000.0, 500000.0]` | Inclusive range (INR) for randomly generating opening account balances. |

### `cards`

| Key | Default | Description |
| :--- | :--- | :--- |
| `networks` | `["VISA", "Mastercard", "RuPay", "Amex"]` | Valid card network values. |
| `types` | `["Debit", "Credit", "Prepaid"]` | Valid card type values. |
| `issue_window_years` | `4` | Maximum years in the past a card's `issue_date` can fall. |
| `expiry_duration_years` | `3` | Number of years after `issue_date` that the card expires. |
| `activation_delay_days` | `[2, 5]` | Inclusive range (days) for the delay between `issue_date` and `activation_date`. |
| `active_probability` | `0.92` | Probability a generated card receives `status = "Active"`. Remaining cards are `"Expired"` or `"Blocked"` based on expiry and fraud metrics. |
| `limits.contactless_default` | `"5000"` | Default contactless transaction limit (INR). Currently stored as a string; not enforced at transaction time. |
| `limits.daily_atm_default` | `"25000"` | Default daily ATM withdrawal limit (INR). Not enforced at transaction time. |
| `limits.online_default` | `"50000"` | Default online transaction limit (INR). Not enforced at transaction time. |
| `limits.international_enabled_prob` | `0.15` | Probability that international usage is enabled for a given card. |

## Known Issues

`fraud_campaigns.target_campaign_share` in `fraud_rules.yaml` is set to `0.0`, meaning campaign-based fraud injection is fully disabled at the configuration level. The campaign profiles (`coordinated_attack`, `sequential_takeover`, `burst_campaign`) are defined but never triggered. The corresponding Silver ETL stage (`SilverCampaign`) is also excluded from the parallel run due to signal reliability issues. These two disabled systems must be resolved together before campaign features can be activated end-to-end.

Card limit values in `product_catalog.yaml` (`contactless_default`, `daily_atm_default`, `online_default`) are stored as strings and are written to the `Card` struct but are not read by `transaction_gen.rs` during amount generation. Transactions are not validated against these limits, making the limit fields decorative. Wiring limit enforcement into the transaction generation pass would enable realistic "Limit Breach" fraud signals that are currently absent from the feature set.
