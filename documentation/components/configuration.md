# Configuration Module (`config.rs`)

## What It Is

RiskFabric's behaviour is fully data-driven. Five YAML files in `data/config/` describe the population, the transaction stream, the financial products, and the adversary — `config.rs` loads them at runtime and parses them into a single strongly-typed `AppConfig` struct that every binary (`generate`, `stream`, `etl`) shares. No recompile is needed to retune the simulation; edit a YAML and rerun.

The value of this design is *consistency*. Because the struct hierarchy mirrors the file layout, a generator, the ETL layer, and the ML pipeline all read the same validated world-view. A change to one config key is checked by the Rust compiler everywhere it is consumed.

## How It Loads

`AppConfig::load()` performs five atomic reads, one per file:

| File | Governs |
| :--- | :--- |
| `customer_config.yaml` | Population size, demographics, financial heuristics, device & ISP profiles |
| `transaction_config.yaml` | Transaction volume, timing, geography, merchant universe |
| `fraud_rules.yaml` | Global seed, channel shares, fraud profiles & temporal patterns |
| `fraud_tuning.yaml` | Anomaly probabilities, campaign parameters, RNG salts |
| `product_catalog.yaml` | Account & card product definitions |

Each file is parsed independently (`serde_yaml::from_str`) and the results are merged into one `AppConfig`. Non-essential keys carry `#[serde(default)]` fallbacks (e.g. `streaming_rate` defaults to `100`), so a missing key degrades gracefully rather than failing the whole load.

## Reference

### `customer_config.yaml`

Population, financial heuristics, device profiles, and ISP assignment.

**`control`** — run scale.

| Key | Default | Notes |
| :--- | :--- | :--- |
| `customer_count` | `3400` | Customers generated per batch run. |
| `transactions_per_customer.min` / `.max` | `200` / `400` | Transaction count range per customer over the lookback window. |
| `parallelism.customer_gen_threads` | `8` | Rayon threads for the customer pass. |
| `parallelism.transaction_gen_threads` | `32` | Rayon threads for the transaction pass. |

**`financials`** — spend anchoring by location class.

| Key | Metro | Urban | Semi-Urban | Rural |
| :--- | :--- | :--- | :--- | :--- |
| `base_spend` (₹/mo) | `35000` | `22000` | `15000` | `9000` |

Credit score starts at `base: 650`, shifted by age via `age_weight: 1.5`, clamped to `[min: 300, max: 900]`.

**`registration`** — `lookback_years: 5` caps how far back a registration date falls; `default_location_type: "Urban"` is the fallback when no OSM node matches.

**`device_profiles.location_shares`** — OS mix per location type (each row sums to 1.0):

| Location | android | ios | upi_app | desktop |
| :--- | :--- | :--- | :--- | :--- |
| Metro | 0.65 | 0.20 | 0.10 | 0.05 |
| Urban | 0.72 | 0.12 | 0.12 | 0.04 |
| Semi-Urban | 0.82 | 0.04 | 0.12 | 0.02 |
| Rural | 0.82 | 0.04 | 0.12 | 0.02 |

**`isp_assignment`** — ISP share per location type plus a `subnets` map (CIDR prefix → `ip_subnet`):

| Location | Jio | Airtel | ACT | BSNL | Others |
| :--- | :--- | :--- | :--- | :--- | :--- |
| Metro | 0.45 | 0.35 | 0.15 | — | 0.05 |
| Urban | 0.50 | 0.30 | — | 0.15 | 0.05 |
| Semi-Urban | 0.50 | 0.15 | — | 0.35 | — |
| Rural | 0.50 | 0.15 | — | 0.35 | — |

The `names`, `email.domains`, and `*_ua_pool` lists are static selection pools — no tuning needed.

### `transaction_config.yaml`

The transaction stream's shape: volume, timing, geography, merchant universe.

| Key | Default | Notes |
| :--- | :--- | :--- |
| `transactions.amount_range` | `[10.0, 50000.0]` | Pre-fraud amount bounds (₹). |
| `transactions.success_rate` | `0.96` | Remaining 4% declined. |
| `transactions.card_present_probability` | `0.35` | P(card_present = true). |
| `transactions.lookback_days` | `365` | History depth per customer. |
| `transactions.geo_bounds` | lat `[8.0, 37.0]`, long `[68.0, 97.0]` | India fallback box. |

**`temporal_patterns`** — relative hourly (24-elem, peak hour 19 @ 4.0) and daily (7-elem, Sat 1.5 / Fri 1.3) weights for timestamp sampling.

**`merchant_categories`** — 17 categories, each `name` + 4-digit `mcc`: `GROCERY (5411)`, `FOOD_AND_DRINK (5812)`, `GENERAL_RETAIL (5311)`, `SERVICES (7299)`, `ENTERTAINMENT (7999)`, `ELECTRONICS (5732)`, `TRAVEL (4722)`, `MEDICAL (8099)`, `AUTOMOTIVE (5511)`, `LUXURY (5944)`, `B2B_WHOLESALE (5099)`, `HOME_GARDEN (5200)`, `TRANSPORT (4111)`, `CHARITY (8398)`, `RETAIL (5999)`, `ALCOHOL (5921)`, `GAMBLING (7995)`.

### `fraud_rules.yaml`

Seed, channel economics, and the fraud profile definitions.

**`global`** — `seed: 5555` (RNG reproducibility), `base_currency: "INR"`, `default_country: "IN"`.

**`payment_channels`** — market share (sums to 1.0) and risk level:

| Channel | share | risk |
| :--- | :--- | :--- |
| upi | 0.70 | 0.7 |
| mobile_wallets | 0.10 | 0.6 |
| cards | 0.10 | 0.9 |
| online | 0.07 | 0.8 |
| mobile_banking | 0.03 | 0.5 |

**`fraud_injector`** — `target_share: 0.01` (1% of customers are fraud targets), `default_fp_rate: 0.005` (false-positive noise), `default_fn_rate: 0.08` (hidden true fraud).

**`fraud_injector.profiles`** — five profiles; `frequency` is a relative weight (sums to 1.0) selecting which profile fires:

| Profile | freq | geo_anomaly_prob | temporal_anomaly_prob | Primary channels |
| :--- | :--- | :--- | :--- | :--- |
| upi_scam | 0.35 | 0.95 | 0.70 | upi (0.80) |
| account_takeover | 0.25 | 0.60 | 0.90 | mobile_banking (0.40), upi (0.30), online (0.30) |
| velocity_abuse | 0.15 | 0.20 | 0.40 | upi (0.70), online (0.30) |
| card_not_present | 0.15 | 0.40 | 0.50 | online (0.70), cards (0.30) |
| friendly_fraud | 0.10 | 0.00 | 0.10 | upi (0.40), cards (0.60) |

Each profile also carries `amount_pattern` (key into `fraud_patterns`), `amount_multiplier` (range × customer's normal spend), and `merchant_bias`. The `temporal_patterns` block overrides the baseline hourly/daily weights for fraud events — `account_takeover` concentrates 70%+ in 00:00–04:00.

### `fraud_tuning.yaml`

Anomaly probabilities (independent of profile) and campaign knobs.

| Key | Default | Notes |
| :--- | :--- | :--- |
| `probabilities.geo_anomaly` | 0.15 | Any txn gets geo anomaly. |
| `probabilities.device_anomaly` | 0.10 | Any txn gets device anomaly. |
| `probabilities.ip_anomaly` | 0.15 | Any txn gets IP anomaly. |
| `probabilities.failure` | 0.15 | Fraud-targeted txn forced `Failed`. |
| `probabilities.chargeback` | 0.08 | Fraud txn results in chargeback. |
| `defaults.geo_anomaly_country` | `"PK"` | `merchant_country` on geo anomaly. |
| `defaults.chargeback_days` | `30` | Chargeback delay. |
| `salts` | injector `7`, mutator `99`, campaign `555` | RNG salts isolating each pass. |

**`campaigns`** — `target_campaign_share: 0.0` (disabled; raise to activate). `coordinated_scam_ip: "103.21.244.12"`, `ato_escalation_rate: 0.25`.

> Campaign profiles (`coordinated_attack`, `sequential_takeover`, `burst_campaign`) are defined in `fraud_rules.yaml` but never triggered while `target_campaign_share` is 0. The Silver ETL stage that consumes them (`SilverCampaign`) is also excluded from the parallel run due to signal reliability — both must be resolved together.

### `product_catalog.yaml`

Account and card product definitions.

**`accounts`** — `types: [Savings, Current, Credit, Salary]`, `creation_window_years: 3`, `bank_id_range: [1000, 9999]`, `balance_range: [1000.0, 500000.0]`.

**`cards`** — `networks: [VISA, Mastercard, RuPay, Amex]`, `types: [Debit, Credit, Prepaid]`, `issue_window_years: 4`, `expiry_duration_years: 3`, `activation_delay_days: [2, 5]`, `active_probability: 0.92`.

| Card limit key | Default | Status |
| :--- | :--- | :--- |
| `limits.contactless_default` | `"5000"` | Stored as string; **not enforced** at generation. |
| `limits.daily_atm_default` | `"25000"` | Stored as string; **not enforced**. |
| `limits.online_default` | `"50000"` | Stored as string; **not enforced**. |
| `limits.international_enabled_prob` | `0.15` | P(international usage enabled). |

> The three limit values are written to the `Card` struct but never read by `transaction_gen.rs`, so transactions are not validated against them — making the fields decorative. Wiring enforcement in would enable realistic "Limit Breach" fraud signals currently absent from the feature set.

## Known Issues

Two rough edges remain in `load()`:

- **Panic-on-error.** File reads and parses use `.expect()`, so a missing file or a YAML syntax error aborts the process immediately. Acceptable for a CLI, but returning a `Result` would allow callers to surface a precise diagnostic instead.
- **Hardcoded paths.** All five paths are relative to the project root (`data/config/...`). Running a binary from another working directory breaks loading; a configurable base path (env var or CLI flag) is needed.
