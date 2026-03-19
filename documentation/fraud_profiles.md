# Synthetic Fraud Profiles

## Summary
The `fraud_profiles.md` document provides a detailed behavioral and statistical breakdown of the five core adversarial signatures simulated by RiskFabric. It explains the contextual logic used by the generator to mimic real-world financial crimes and provides examples of how these patterns manifest in the synthetic data stream.

## Design Intent
These profiles are designed to challenge machine learning models by mirroring the statistical "noise" and multi-dimensional anomalies of modern fraud. By shifting from simple "hardcoded amount" rules to **Behavioral Multipliers** and **Contextual Biases**, the generator forces downstream models to evaluate combinations of spatial velocity, merchant categories, and temporal deviations.

---

## 1. Velocity Abuse
**Objective:** Simulate a bot network or organized fraud ring rapidly "testing" compromised card details or exploiting a merchant gateway before limits are triggered.

### Behavioral Signature
*   **Amount Strategy:** `customer_normal_range` with a strict `0.90x to 1.10x` multiplier.
*   **Primary Signals:** Extreme Transaction Frequency (`rapid_fire_transaction_flag`), High Spatial Velocity (`impossible travel`), and Specific Merchant Bias (`GAMBLING`, `ENTERTAINMENT`).
*   **The "Trick":** By keeping the transaction amount perfectly aligned with the customer's normal spending habits, it evades simple threshold-based alerts, forcing the model to rely entirely on speed and location.

### Example Scenario
A customer whose average transaction is ₹500 has three transactions generated within a 4-minute window for exactly ₹490, ₹510, and ₹495 at three different entertainment merchants located 800km away from their last known physical transaction.

---

## 2. Account Takeover (ATO)
**Objective:** Simulate a malicious actor gaining unauthorized access to a legitimate user's banking app or online portal to drain funds or make high-value purchases.

### Behavioral Signature
*   **Amount Strategy:** `customer_normal_range` with a tight `0.95x to 1.05x` multiplier.
*   **Primary Signals:** Extreme Spatial Velocity (`impossible travel`), Temporal Anomaly (occurring during the customer's historical "sleep" hours), and Channel Bias (`mobile_banking`, `online`).
*   **The "Trick":** Similar to Velocity Abuse, the amount does not spike. The anomaly is purely contextual: the transaction occurs on a new device, from a new IP, at 3:00 AM, purchasing from a `LUXURY` or `ELECTRONICS` merchant.

### Example Scenario
A customer completes an in-store grocery purchase in Mumbai at 8:00 PM. At 3:15 AM the following morning, a mobile banking transfer for a standard amount is initiated from an IP address in Delhi.

---

## 3. Card Not Present (CNP) Fraud
**Objective:** Simulate the unauthorized use of stolen credit card details (PAN, CVV) for online purchases, typically for easily liquidatable goods.

### Behavioral Signature
*   **Amount Strategy:** `customer_normal_range` with an aggressive `1.0x to 5.0x` multiplier.
*   **Primary Signals:** Channel Bias (100% `online`), Merchant Category Bias (`ELECTRONICS`, `LUXURY`), and elevated `amount_deviation_z_score`.
*   **The "Trick":** This profile blends moderate amount spikes with specific merchant categories. It tests the model's ability to correlate the "Online" channel with high-risk retail sectors.

### Example Scenario
A customer who typically spends ₹2,000 per transaction across various local stores suddenly has an online transaction for ₹8,500 at an `ELECTRONICS` merchant, processed without physical card presence.

---

## 4. UPI Scam (Social Engineering)
**Objective:** Simulate phishing or coercive scams where a victim is tricked into authorizing a high-value transfer via the Unified Payments Interface (UPI).

### Behavioral Signature
*   **Amount Strategy:** `customer_normal_range` with a massive `1.5x to 4.0x` multiplier.
*   **Primary Signals:** Massive `amount_deviation_z_score`, Channel Bias (Heavily biased toward `upi`), and Merchant Category Bias (`GENERAL_RETAIL`, `SERVICES`).
*   **The "Trick":** This represents the classic "drain the account" scenario. The model must learn that extreme amount deviations on the UPI channel to unfamiliar service merchants are highly suspicious, even if the device fingerprint appears legitimate.

### Example Scenario
A user with an average transaction of ₹300 suddenly authorizes a UPI payment of ₹1,100 to a previously unseen "Services" merchant, heavily deviating from their historical spend pattern.

---

## 5. Friendly Fraud (First-Party Fraud)
**Objective:** Simulate a legitimate customer making a valid purchase (often digital goods or travel) and subsequently filing a false chargeback claim with their bank.

### Behavioral Signature
*   **Amount Strategy:** `customer_normal_range` with a standard `0.5x to 1.5x` multiplier.
*   **Primary Signals:** **None.** This profile intentionally lacks spatial, temporal, or behavioral anomalies.
*   **The "Trick":** This is the hardest profile to detect at the transaction level. The location, device, and amount are all perfectly normal. Detection relies entirely on historical entity-level features, such as the `cf_fraud_rate` (Customer Fraud Rate) or `merchant_category` risks (`TRAVEL`, `FOOD_AND_DRINK`).

### Example Scenario
A customer purchases a ₹1,200 airline ticket online from their home IP address, using their normal device, during their usual active hours. Three weeks later, the transaction is marked with a `chargeback` flag.
