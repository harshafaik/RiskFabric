# Data Generation Modules

To generate the synthetic financial data required for training a fraud detection model, there are four data generation modules, each responsible for generating a set of financial records, such as customer profiles, account and card details for each customer profile, as well as transactions done by each customer. The relationship between them is as follows:

Customer: single entity
Account: N:1 with Customer
Card: N:1 with Account and N:1 with Customer
Transaction: N:1 with Card

For enhancing the realism of the transactions done by customers, geographical parameters are added using OpenStreetMap(OSM) through which an exported parquet file, one responsible for residential locations and another for merchant locations is used.

## Modules

- [Customer Data Generator](components/customer_generator.md)
- [Account and Card Data Generator](components/account_and_card_generator.md)
- [Transactional Data Generator](components/transaction_generator.md)
- [Batch Orchestrator](components/batch_generator.md)
- [Streaming Orchestrator](components/streaming_generator.md)
- [Fraud Injector](components/adversary_logic_engine.md)
- [Configuration Module](components/configuration.md)

