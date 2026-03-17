# Model Metadata Utility (`dump_model.py`)

## Summary
The `dump_model.py` script is a specialized inspection utility used to extract the internal schema and feature definitions from a serialized XGBoost model. It ensures that the real-time scoring engine (`scorer.py`) has exact visibility into the feature names and data types (categorical, float, integer) expected by the binary booster.

## Architectural Decisions
This utility is designed to solve the **Feature Alignment Problem** in production ML. When an XGBoost model is saved as a JSON booster, it encodes its expected input schema. If the inference engine sends features in the wrong order or with the wrong data types, the model may crash or return incorrect results. By using `get_booster().feature_names`, this utility provides a programmatically verifiable source of truth for the inference interface, allowing the `scorer.py` to dynamically reorder and cast its input DataFrames to match the model's training state.

The implementation of **JSON-Path Extraction** for categorical features is a critical design choice. Since XGBoost's native categorical encoding is serialized within the `learner` block of the JSON file, this utility parses those internal dictionaries. This architectural safety measure ensures verification of the categorical "levels" (e.g., specific merchant categories) the model was exposed to during training, preventing "Unknown Category" errors during real-time scoring.

## System Integration
`dump_model.py` is an auxiliary utility in the **Machine Learning layer**. It is typically run after `train_xgboost.py` to verify the model artifact before it is deployed to the scoring service. It acts as a manual "Gatekeeper" for ensuring feature consistency across the pipeline.

## Known Issues
A fragile, regex-based approach (`re.findall`) is currently used to extract categorical strings from the XGBoost JSON. This is an unreliable method that depends on the specific serialization format of the XGBoost version being used. A more robust parser that follows the official XGBoost JSON schema is required. Additionally, the utility currently only prints the metadata to the console; refactoring is needed to export a structured `schema.yaml` file that the `scorer.py` can load automatically to configure its inference pipeline.
