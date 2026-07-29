# Fraud Signatures & Attack Patterns

## Overview

RiskFabric does not invent fraud — it *simulates* it. The synthetic adversary is defined entirely in configuration (`fraud_rules.yaml` for profile behavior, `fraud_tuning.yaml` for mutation probabilities and campaign parameters) and executed by the injector in `fraud.rs`. Every fraud transaction is produced alongside a `FraudMetadata` record carrying its ground-truth type, injected anomalies, and any label noise — so models can be trained and audited against known answers.

Fraud is injected at generation time, not post-hoc. The engine picks a target share of transactions (default `target_share: 0.01`, i.e. 1% intentional fraud), then layers a behavioral profile, a spatial/temporal anomaly, and optional campaign coordination onto each one.

## Design Principles

**Structured, not random.** Each profile anchors to a real payment-system threat vector (UPI scams, credential theft, card testing, etc.) rather than sampling noise from a distribution. This gives the synthetic data recognizable structure that resembles what fraud teams actually see.

**Probabilistic mutation.** Anomalies are config-driven probabilities, not guarantees. A UPI scam only fires a geo-anomaly 95% of the time; the other 5% it looks ordinary. Combined with **label noise** (`default_fp_rate` injects false positives, `default_fn_rate` hides true fraud), this forces models to learn high-dimensional behavioral boundaries instead of memorizing a single telltale feature.

**Amount mimicry.** Every fraud profile uses `customer_normal_range` strategy — the fraudulent amount is derived from the *victim's own* spending baseline via a profile-specific multiplier, then clamped to a sane bound. Fraud rarely looks like a round, obviously-wrong number; it looks like a slightly-to-severely inflated version of normal.

## 1. Fraud Profiles

All five profiles share the same injection path but differ in *which* dimensions they corrupt.

| Profile | Amount Multiplier | Geo-Anomaly | Key Signals | Primary Channel |
| :--- | :--- | :--- | :--- | :--- |
| **UPI Scam** | `1.5x – 4.0x` | **95%** | Massive amount deviation, UPI channel bias, retail/service merchant bias | UPI (80%) |
| **Account Takeover** | `0.95x – 1.05x` | **60%** | Impossible travel, nocturnal timing, foreign datacenter IP, attacker UA pool | Mobile Banking (40%), UPI (30%), Online (30%) |
| **Velocity Abuse** | `0.90x – 1.10x` | **20%** | Rapid-fire frequency, impossible travel, entertainment/gambling bias | UPI (70%), Online (30%) |
| **Card Not Present** | `1.0x – 5.0x` | **40%** | 100% online channel, card-not-present, VPN IP, headless browser UA | Online (70%), Cards (30%) |
| **Friendly Fraud** | `0.5x – 1.5x` | **0%** | *None at transaction level* — relies on later `chargeback` flag | Cards (60%), UPI (40%) |

### 1.1 UPI Scam
The classic "drain the account" social-engineering attack. A victim is tricked into authorizing a high-value UPI transfer to an unfamiliar merchant. The multiplier is the most aggressive of any profile (`1.5x – 4.0x`), sometimes sampled directly from `upi_common_amounts` (₹150–₹10,000) which deliberately overlaps with normal grocery and retail spend. Detection depends on the model correlating extreme amount deviation on the UPI channel toward `GENERAL_RETAIL`/`SERVICES` merchants — even when the device fingerprint looks legitimate.

### 1.2 Account Takeover (ATO)
A malicious actor gains access to a legitimate user's banking session and drains funds. The amount stays near-normal (`0.95x – 1.05x`) — the anomaly is purely *contextual*: the transaction lands in the victim's sleep hours (hourly weights concentrate 70%+ in 00:00–04:00), originates from a foreign datacenter IP (`52.`, `34.` ranges), and uses a device from a fixed attacker UA pool (`ato_ua_pool`). Geo-anomaly fires 60% of the time, planting the transaction at a distant coordinate.

### 1.3 Velocity Abuse
A botnet or fraud ring "tests" stolen card details with tiny, repeated probes before a real charge. Amounts stay tightly within `0.90x – 1.10x` of normal and are drawn from `velocity_test_amounts` (₹1.01, ₹1.23, ₹2.05, ₹5.00, ₹10.00, ₹25.00, ₹50.00). The signal is **frequency and velocity**, not value: multiple transactions within seconds, impossible travel between them, entertainment/gambling merchant bias. Geo-anomaly is low (20%) — the bot stays local to avoid location rules.

### 1.4 Card Not Present (CNP)
Stolen card credentials used for online purchases of liquidatable goods. Forcefully sets `card_present = false`, biases 100% to the `online` channel, and applies a moderate `1.0x – 5.0x` amount multiplier sampled toward `cnp_common_amounts`. Device/IP anomalies use VPN ranges (`185.`, `104.16.`) and a headless Chrome UA. The model must correlate the online channel with high-risk retail sectors (`ELECTRONICS`, `LUXURY`).

### 1.5 Friendly Fraud
The hardest profile to catch. A legitimate customer makes a perfectly normal purchase, then later files a false chargeback. There is **no spatial, temporal, or behavioral anomaly** — geo-anomaly is 0%, the device and location are the customer's own. Detection is impossible at the transaction level and relies entirely on entity-level history: the `cf_fraud_rate` (customer fraud rate) and merchant-category risk (`TRAVEL`, `FOOD_AND_DRINK`). The `chargeback` flag lands weeks after the transaction.


## 2. Campaigns (Coordinated Attacks)

Beyond standalone profiles, RiskFabric can coordinate multiple transactions into attack structures. Campaigns are configured in the `fraud_campaigns` block but **disabled by default** (`target_campaign_share: 0.0`) — raise it above zero to activate. When active, `initialize_campaign()` assigns a card to a campaign context carrying an attacker coordinate, and `apply_campaign_logic()` overrides the per-transaction anomalies.

**Coordinated Attack** — multiple distinct cards hit the same shared infrastructure simultaneously. Every transaction in the campaign inherits the *exact same* IP (`coordinated_scam_ip`, default `103.21.244.12`) and the attacker's geographic coordinate, mimicking a scammer call center or centralized botnet hub.

**Sequential Takeover** — a single card experiences progressive escalation. Each step's amount escalates by `amount_escalation` (default `0.30`) from the previous, and the location "sticks" to the attacker's coordinate across the whole sequence, producing a persistent anomalous footprint rather than isolated spikes.

## 3. Current Limitations

- **No traveling-customer model.** Legitimate trips produce velocity spikes indistinguishable from fraud, inflating false positives in spatial anomaly detectors.
- **Campaigns are network/IP + coordinate only.** No A2A graph signals or mule chains. Supporting graph-based detection requires destination-account entities and graph topology in `fraud.rs`.
