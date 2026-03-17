# Project Goals & Objectives

## Summary
The `objectives.md` document defines the high-level mission and technical milestones for the RiskFabric project. It outlines the strategic intent behind building a high-fidelity synthetic data generator and the specific problems it aims to solve for the financial technology community.

## Design Intent
RiskFabric is designed to address the **"Data Paradox"** in fraud detection: researchers require large volumes of labeled data to develop effective models, but real-world financial data is sensitive and often inaccessible. By creating a high-fidelity, "white-box" alternative, the project provides a safe environment for testing machine learning algorithms and the operational infrastructure required for real-time fraud detection.

A key strategic objective is the promotion of **Infrastructure-as-Code for Simulation**. Transitioning from static CSV datasets to dynamic, configuration-driven environments allows organizations to "stress-test" systems against hypothetical scenarios—such as doubling transaction volumes—without requiring production data.

---

## 🎯 Key Milestones
1.  **High-Fidelity Generation**: Reaching 180k+ TPS while maintaining spatial and temporal realism.
2.  **Streaming Parity**: Ensuring models trained on batch data perform consistently in real-time Kafka environments.
3.  **Adversarial Diversity**: Expanding the fraud library to include multi-stage attacks like money laundering and mule-account networks.

---

## Known Issues
Focus is currently placed on **Individual and Coordinated Fraud**, but **Macroeconomic Factors** remain unimplemented. The simulation assumes spending patterns are unaffected by external events such as inflation or holidays. Implementing a "Global Event Engine" is necessary to simulate seasonal surges and economic shifts, providing a more challenging baseline for detection models.

Furthermore, the project lacks **Multi-Currency Support**. The simulation is anchored to a single base currency, preventing the modeling of international fraud or cross-border remittance scams. Refactoring the transaction engine to handle dynamic currency conversion and exchange-rate fluctuations is required to support global fintech use cases.
