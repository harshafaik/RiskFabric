# Synthetic Data Schema

RiskFabric generates a multi-table synthetic dataset mirroring professional financial environments. The entities link logically for scale and realism.

## Entity Relationship Overview

- **Customer**: The primary entity. Owns several Accounts.
- **Account**: A financial container (Savings, Current, Credit). Contains several Cards.
- **Card**: The instrument used for transactions.
- **Transaction**: A financial event linked to a Card, Account, and Customer.
- **FraudMetadata**: Ground-truth data linked 1:1 with Transactions to explain the generation context.

---

## 👥 Customer (`customers.parquet`)
Defines the synthetic population's demographics and geographic baseline.

| Field | Type | Description |
| :--- | :--- | :--- |
| `customer_id` | String | Unique UUID for the customer. |
| `name` | String | Full name (Indian-centric). |
| `age` | UInt8 | Age of the customer (18-90). |
| `email` | String | Synthetic email address. |
| `location` | String | Full residential address (OSM-based). |
| `state` | String | Standardized Indian state name. |
| `location_type` | String | Urban vs. Rural classification. |
| `home_latitude` | Float64 | WGS84 Latitude of home. |
| `home_longitude` | Float64 | WGS84 Longitude of home. |
| `home_h3r5` | String | H3 Resolution 5 index (Neighborhood level). |
| `home_h3r7` | String | H3 Resolution 7 index (Block level). |
| `credit_score` | UInt16 | Synthetic credit score (300-850). |
| `monthly_spend` | Float64 | Average expected monthly expenditure. |
| `customer_risk_score`| Float32 | Baseline risk probability (0.0 to 1.0). |
| `is_fraud` | Bool | Flag indicating if this customer represents a fraud target. |
| `registration_date` | String | ISO 8601 date of account registration. |

---

## 🏦 Account (`accounts.parquet`)
The logical banking container for funds.

| Field | Type | Description |
| :--- | :--- | :--- |
| `account_id` | String | Unique UUID for the account. |
| `customer_id` | String | FK to Customer. |
| `bank_id` | String | Identifier for the issuing bank. |
| `account_no` | String | 12-digit synthetic account number. |
| `account_type` | String | Savings, Current, or Credit. |
| `balance` | Float64 | Current funds in the account. |
| `status` | String | Active, Closed, or Suspended. |
| `creation_date` | String | The account opening date. |

---

## 💳 Card (`cards.parquet`)
The payment instrument associated with an account.

| Field | Type | Description |
| :--- | :--- | :--- |
| `card_id` | String | Unique UUID for the card. |
| `account_id` | String | FK to Account. |
| `customer_id` | String | FK to Customer. |
| `card_number` | String | 16-digit synthetic PAN. |
| `card_network` | String | VISA, Mastercard, or RuPay. |
| `card_type` | String | Debit or Credit. |
| `status` | String | Active, Blocked, or Expired. |
| `status_reason` | String | Reason for status changes (e.g., SIM Swap Suspect). |
| `issue_date` | String | Card issuance date. |
| `activation_date` | String | Initial card usage date. |
| `expiry_date` | String | Card expiry date. |
| `issuing_bank` | String | Full name of the bank. |
| `bank_code` | String | Standardized 4-digit bank identifier. |

---

## 💸 Transaction (`transactions.parquet`)
The high-volume stream of financial events.

| Field | Type | Description |
| :--- | :--- | :--- |
| `transaction_id` | String | Unique UUID for the transaction. |
| `card_id` | String | FK to Card. |
| `account_id` | String | FK to Account. |
| `customer_id` | String | FK to Customer. |
| `merchant_id` | String | Unique identifier for the merchant. |
| `merchant_name` | String | Name of the business. |
| `merchant_category`| String | Category (e.g., GROCERY, TRAVEL). |
| `merchant_country` | String | Country code of the merchant (defaults to IN). |
| `amount` | Float64 | Transaction value in base currency. |
| `timestamp` | String | ISO 8601 high-precision timestamp. |
| `transaction_channel`| String | online, in-store, UPI, etc. |
| `card_present` | Bool | Physical card usage flag. |
| `user_agent` | String | Browser or POS device identifier. |
| `ip_address` | String | IPv4 address of the requester. |
| `status` | String | High-level status (Success or Failed). |
| `auth_status` | String | Banking authorization code (approved/declined). |
| `failure_reason` | String | Detailed reason for declined transactions. |
| `is_fraud` | Bool | **Noisy Label** (includes FN/FP). |
| `chargeback` | Bool | Flag indicating a later customer dispute. |
| `location_lat` | Float64 | Latitude of the transaction event. |
| `location_long` | Float64 | Longitude of the transaction event. |
| `h3_r7` | String | H3 Resolution 7 index of the transaction location. |

---

## 🕵️ Fraud Metadata (`fraud_metadata.parquet`)
Internal ground-truth for debugging and advanced ML training. This table is **not** used in standard inference but is vital for "white-box" evaluation.

| Field | Type | Description |
| :--- | :--- | :--- |
| `transaction_id` | String | FK to Transaction. |
| `fraud_target` | Bool | **Ground Truth** (True Fraud flag). |
| `fraud_type` | String | Profile used (e.g., `upi_scam`, `ato`). |
| `label_noise` | String | Reason for label mismatch (if any). |
| `injector_version` | String | Engine version. |
| `geo_anomaly` | Bool | True if location represents an outlier. |
| `device_anomaly` | Bool | True if device/UA represents an outlier. |
| `ip_anomaly` | Bool | True if IP represents a known malicious prefix. |
| `burst_session` | Bool | Part of a rapid-fire sequence. |
| `burst_seq` | Int32 | Sequence number within a burst session. |
| `campaign_id` | String | Link to a coordinated attack campaign. |
| `campaign_type` | String | Coordination type (e.g., `coordinated_attack`). |
| `campaign_phase` | String | Phase within the campaign (early, active, late). |
| `campaign_day_number`| Int32 | Days since campaign start. |
