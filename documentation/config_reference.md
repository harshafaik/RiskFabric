# Configuration Reference

RiskFabric uses a modular YAML-based configuration system to control every aspect of the simulation, from demographic distributions to specific fraud attack probabilities.

## 🛠️ Simulation Control (`generation_control.yaml`)
Controls the scale, volume, and performance of the generation pass.

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `customer_count` | Integer | Total number of synthetic customers to generate. |
| `transactions_per_customer` | Range | Min/Max transactions generated for each card lifecycle. |
| `parallelism` | Object | Thread counts for `Rayon` workers (e.g., `transaction_gen_threads`). |

---

## 👥 Customer Profile (`customer_config.yaml`)
Defines the "Identity" layer of the simulation.

- **Names & Email**: Lists of first/last names and email domains used to build synthetic identities.
- **Location Types**: Categorizes customers into `Metro`, `Urban`, `Semi-Urban`, and `Rural`.
- **Financial Heuristics**: 
    - `base_spend`: Sets the average monthly expenditure based on `location_type`.
    - `credit_score`: Defines the starting point and age-based progression of credit risk.

---

## 🏛️ Product Catalog (`product_catalog.yaml`)
Defines the "Banking" layer, including account types and card networks.

- **Accounts**: Sets the balance ranges and the variety of accounts (Savings, Salary, etc.).
- **Cards**: 
    - `networks`: Defines market participation (VISA, Mastercard, RuPay, Amex).
    - `limits`: Sets default transaction limits for online and contactless usage.
    - `active_probability`: Probability that a card remains in an "Active" state.

---

## 💸 Transaction Logic (`transaction_config.yaml`)
Defines the "Behavioral" layer for legitimate spend.

- **Merchant Categories**: Standard industry categories used for merchant assignment.
- **Geo Bounds**: The WGS84 bounding box used for "Global" coordinate fallback (defaults to India).
- **Success Rate**: The probability that the synthetic bank approves a legitimate transaction.

---

## 🕵️ Fraud Rules (`fraud_rules.yaml`)
Defines the "Malicious" layer, focusing on specific attack profiles.

- **Fraud Profiles**:
    - `frequency`: The relative weight of a profile (e.g., `upi_scam` vs `friendly_fraud`).
    - `amount_pattern`: Links to a specific distribution of amounts.
    - `geo_anomaly_prob`: Probability that this fraud type occurs far from the user's home.
- **Campaigns**: Configures coordinated attacks like `sequential_takeover` and `coordinated_attack`.

---

## 🎛️ Simulation Tuning (`fraud_tuning.yaml`)
Provides technical "knobs" to refine the difficulty of detection.

- **Probabilities**: Global overrides for `geo_anomaly`, `device_anomaly`, and `ip_anomaly`.
- **Salts**: Used to shift the PRNG state for injector and mutator cycles. This ensures that different runs produce unique data even when using the same base seed.
- **Campaign params**: Specific escalation rates for coordinated attacks (e.g., `ato_escalation_rate`).
