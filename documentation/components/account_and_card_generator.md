# Account and Card Data Generator

## Overview
The account and card generator modules `account_gen.rs` and `card_gen.rs` are responsible for constructing the financial "graph" of the simulation, establishing the hierarchical relationships between customers and their payment instruments. This layer provides the relational structure necessary to validate entity-linking algorithms, check transaction flows, and simulate realistic cross-account fraud behaviors.

## Schema

Each account and card record maintains consistent relations back to the customer profile:

<div style="max-width: 400px; margin: 0 auto;">

```text
Customer ──customer_id──► Account
Account ──account_id───► Card
Customer ──customer_id──► Card
```
</div>

<details>
<summary><code>Account</code></summary>

| Field Name | Type | Description |
| :--- | :--- | :--- |
| `account_id` | `String` | Unique UUID v4 identifying the account. |
| `customer_id` | `String` | UUID v4 link back to the parent customer profile. |
| `bank_id` | `String` | String identifier representing the bank (e.g., `"Bank-XXXX"`). |
| `account_no` | `String` | Synthetic 12-digit bank account number. |
| `account_type` | `String` | Type of account (`"Savings"`, `"Current"`, or `"Credit"`), stochastically chosen. |
| `balance` | `f64` | Starting balance randomly selected between 1,000.00 and 500,000.00. |
| `account_status` | `String` | Current status of the account (defaulting to `"Active"`). |
| `creation_date` | `String` | ISO 8601 date string (`"YYYY-MM-DD"`) representing when the account was opened. |

</details>

<details>
<summary><code>Card</code></summary>

| Field Name | Type | Description |
| :--- | :--- | :--- |
| `card_id` | `String` | Unique UUID v4 identifying the card. |
| `account_id` | `String` | UUID v4 link back to the parent account. |
| `customer_id` | `String` | UUID v4 link back to the customer. |
| `card_number` | `String` | Synthetic 16-digit primary account number (PAN). |
| `card_network` | `String` | Brand network (`"VISA"`, `"Mastercard"`, or `"RuPay"`), stochastically chosen. |
| `card_type` | `String` | Classification of the card (`"Debit"` or `"Credit"`), stochastically chosen. |
| `status` | `String` | State of the card (`"Active"`, `"Expired"`, or `"Blocked"`), derived from days to expiry or fraud metrics. |
| `status_reason` | `String` | Description of current card status (`"Normal usage"`, `"Suspected Fraud"`, `"Card Validity Ended"`). |
| `issue_date` | `String` | ISO 8601 date string (`"YYYY-MM-DD"`) representing when the card was issued. |
| `activation_date` | `String` | ISO 8601 date string (`"YYYY-MM-DD"`) representing activation (typically 2-4 days post-issue). |
| `expiry_date` | `String` | ISO 8601 date string (`"YYYY-MM-DD"`) representing card expiration (exactly 3 years post-issue). |
| `contactless_limit` | `String` | Maximum amount allowed for contactless transactions (currently empty string placeholder). |
| `daily_atm_limit` | `String` | Maximum amount allowed for ATM withdrawals (currently empty string placeholder). |
| `online_limit` | `String` | Maximum amount allowed for online transactions (currently empty string placeholder). |
| `international_usage` | `String` | Flag designating if international usage is enabled (currently empty string placeholder). |
| `issuing_bank` | `String` | Formatted name of the bank (composed as `"Bank of {bank_id}"`). |
| `bank_code` | `String` | The bank routing/clearing code (equivalent to `bank_id`). |

</details>

These generators prioritize **Relational Consistency**. Instead of generating accounts and cards in isolation, the system uses a top-down orchestration: Customers drive the creation of Accounts, which in turn drive the creation of Cards. This ensures that every card PAN is programmatically linked back to a specific customer ID, maintaining 100% referential integrity across the multi-million row dataset.

For **Entity Density**, a probabilistic account ownership model is implemented in `account_gen.rs`. While every customer is guaranteed a primary account, there is a 50% chance for a customer to own a secondary account (e.g., a "Credit" account in addition to a "Savings" account). This architectural decision allows the simulation to model complex multi-entity behaviors, such as "Balance Transfers" or "Cross-Account Velocity," which are common signals in sophisticated fraud patterns.

In `card_gen.rs`, an **Account-Driven Mapping** strategy is used. The card generator iterates over the accounts vector and issues a unique payment instrument for each. This one-to-one mapping simplifies the transaction generation logic while ensuring that the "issuing bank" metadata is correctly inherited from the parent account entity.

`account_gen.rs` and `card_gen.rs` act as the second stage of the generation pipeline. They consume the generated `Customer` vector produced by `customer_gen.rs` and construct matching financial accounts and credit/debit cards. The resulting vectors of `Account` and `Card` structs are materialized into Parquet files by `generate.rs` and passed downstream to the transaction simulation engine.

## Known Issues
The probability of secondary account creation is currently hardcoded to 50% directly inside the generation logic. This makes it impossible to configure custom account densities dynamically. Moving this threshold check to `customer_config.yaml` is required.

Furthermore, card limit metadata parameters (`contactless_limit`, `daily_atm_limit`, and `online_limit`) are currently initialized as empty strings. This prevents downstream transaction engines from enforcing realistic "Limit Breaches" during transaction scoring. A "Product Catalog" lookup is required to assign realistic limit boundaries based on the account profile type.
