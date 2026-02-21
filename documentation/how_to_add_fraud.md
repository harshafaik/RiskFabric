# How-to: Add a New Fraud Signature

This guide provides a task-oriented path for developers to inject new fraud behaviors into the RiskFabric engine.

## 1. Define the Profile
New fraud patterns are defined in `src/generators/fraud.rs`. Every profile needs:
- A unique name.
- A weighted probability in the configuration.
- A Behavioral and Spatial signature.

## 2. Implement the Mutator
Add a new branch to the `FraudMutator` logic.
```rust
// Example skeleton
fn mutate_upi_scam(txn: &mut Transaction) {
    // Modify amount, location, or device
}
```

## 3. Register in Config
Update `data/config/fraud_rules.yaml` to include your new profile and its target weight.

---
*Detailed guide coming soon.*
