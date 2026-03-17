# Financial Entity Linking (`account_gen.rs` & `card_gen.rs`)

## Summary
The `account_gen.rs` and `card_gen.rs` modules are responsible for constructing the financial "graph" of the simulation. They define the hierarchical relationships between customers and their payment instruments, ensuring that every transaction is linked to a valid account and card entity. This layer establishes the structural foundation required for testing entity-linking models and cross-account fraud detection.

## Architectural Decisions
These generators prioritize **Relational Consistency**. Instead of generating accounts and cards in isolation, the system uses a top-down orchestration: Customers drive the creation of Accounts, which in turn drive the creation of Cards. This ensures that every card PAN is programmatically linked back to a specific customer ID, maintaining 100% referential integrity across the multi-million row dataset.

For **Entity Density**, a probabilistic account ownership model is implemented in `account_gen.rs`. While every customer is guaranteed a primary account, there is a 50% chance for a customer to own a secondary account (e.g., a "Credit" account in addition to a "Savings" account). This architectural decision allows the simulation to model complex multi-entity behaviors, such as "Balance Transfers" or "Cross-Account Velocity," which are common signals in sophisticated fraud patterns.

In `card_gen.rs`, an **Account-Driven Mapping** strategy is used. The card generator iterates over the accounts vector and issues a unique payment instrument for each. This one-to-one mapping simplifies the transaction generation logic while ensuring that the "issuing bank" metadata is correctly inherited from the parent account entity.

## System Integration
These modules are the primary components of the batch generation pipeline. They are invoked by `generate.rs` immediately after the population has been created. The resulting vectors of `Account` and `Card` structs are then materialized into Parquet files and passed downstream to the transaction engine.

## Known Issues
A hardcoded 50% probability for secondary account creation is currently used. This should be moved to `customer_config.yaml` to allow for more granular control over the "financial depth" of the population. 

Furthermore, **Card Metadata** (like `contactless_limit` and `online_limit`) is currently initialized as empty strings. This prevents the simulation from enforcing realistic "Limit Breaches" during transaction generation. A "Product Catalog" lookup in `card_gen.rs` is required to populate these fields with realistic values based on the account type, which will enable a new class of "Limit-Based" fraud detection features.
