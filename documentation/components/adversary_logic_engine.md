# Fraud Injector (`fraud.rs`)

## Summary
The `fraud.rs` module contains the "attack logic" of RiskFabric. It defines the specific behavioral rules used to mutate legitimate transactions into adversarial patterns. This module ensures that synthetic fraud reflects realistic criminal tactics such as velocity abuse, account takeovers, and coordinated campaigns.

## Architectural Decisions
This module follows a **Profile-Driven Mutation Strategy**. The engine interprets profiles from `fraud_rules.yaml` to dynamically adjust transaction attributes, rather than using hardcoded fraud logic. This allows for experimentation with new fraud signatures without modifying the core simulation code.

For **Behavioral Mimicry**, a relative amount calculation strategy is implemented. By allowing an attacker to spend within a multiplier range of the customer's average transaction amount (e.g., 0.8x to 1.2x), the engine simulates subtle, low-value fraud that is difficult for simple rule-based systems to detect.

To simulate **Stateful Attacks**, the `apply_campaign_logic` function is used. This allows the generator to override standard spatial and device signals with persistent attacker metadata (e.g., a shared IP or fixed coordinates). This architectural decision is critical for generating the clustered signals that modern graph-based fraud models are designed to identify.

## System Integration
`fraud.rs` is a stateless logic provider consumed by the `transaction_gen.rs` module. It acts as a specialized "mutation filter" that takes a completed transaction and a fraud profile and returns a set of behavioral anomalies.

## Current Limitations
String-based matching (`f_type == "account_takeover"`) determines fraud mutation logic — a typo in the YAML silently falls through. Refactoring to an enum would give compile-time safety. `calculate_fraud_timestamp` is also limited to two fraud types; generalizing to more temporal attack patterns is needed.
