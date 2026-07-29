# ETL Pipeline — Outstanding Items

## Orphan Columns in `fact_transactions_silver` Transform

`transform_sequence_features` produces 35 columns via `.select()`, but the Silver schema only has 30. Five are `lit(0)` placeholders:

- `same_day_transaction_count`
- `is_holiday`
- `is_foreign`
- `is_cross_border`
- `is_ip_mismatch`

These are silently dropped in `df_to_silver_transactions` and consume zero storage. Decide: remove them from the transform, or add them to the schema if they're planned for future use.

## Campaign Features (Disabled)

- Generator config: `data/config/fraud_rules.yaml` → `fraud_campaigns.target_campaign_share: 0.0`
- Campaign assignment is disabled at the generator level. Every `campaign_id` in bronze is `None`.
- The ETL transform (`campaign.rs`) has a non-deterministic bug: no sort before `cum_sum`, making campaign sequence numbering unreliable.
- Isolated single fraud transactions become fake "campaigns of size 1."
- Hardcoded 48h gap threshold is untunable.

**Status:** Excluded from `silver-all`. CLI flag `--silver-campaign` shows a warning. Gold hardcodes zeros for all campaign columns.

**To use campaigns:** Set `target_campaign_share > 0`, fix the sort/cum_sum ordering, and filter single-tx campaigns.

## Device/IP and Network Features (Removed by Decision)

Per `documentation/decisions/ip_device_reputation_removal.md`: `network.rs` and `device_ip.rs` transforms, the `run_silver_network` / `run_silver_device_ip` stages, and the `ip_features_silver` / `device_features_silver` tables were deleted. `ip_address` and `user_agent` remain on transaction rows as future join keys. IP/device intelligence is deferred to a Redis-backed external lookup consulted defensively by the scorer, keeping it out of model training.
