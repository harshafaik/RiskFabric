# Geospatial Reference Pipeline

## Overview

The `warehouse/` dbt project transforms raw OSM staging tables into geographic reference data for the simulation generators. It runs on Postgres/PostGIS, consumes `raw_residential`, `raw_merchants`, and `raw_financial` (populated by `prepare_refs.rs`), and produces `mart_residential`, `mart_merchants`, and `mart_district_summary` for export by `export_references.rs`. Two staging models, three mart models, one macro.

The full pipeline from OSM → staging → dbt → export is shown in the [OSM Reference Extractor](reference_preparator.md) diagram.

## Schema

| Model | Input | Key Transformations |
| :--- | :--- | :--- |
| `stg_residential` | `raw_residential` | State normalization via `ref_state_map` lookup |
| `stg_merchants` | `raw_merchants` | Category + risk level via `ref_category_map` seed |
| `mart_residential` | `stg_residential` | `ST_Intersects` against DataMeet boundaries for verified `state`/`district_name`; `normalize_city` macro; `regexp_replace` pincode |
| `mart_merchants` | `stg_merchants` | Same spatial joins + normalization as `mart_residential` |
| `mart_district_summary` | `mart_residential` + `mart_merchants` | District-level aggregation: node counts, risk distribution, merchant-to-residential ratio |

<details>
<summary>Full field listings</summary>

### `stg_residential`
`osm_id` (`BIGINT`), `h3_index` (`TEXT`), `latitude`, `longitude` (`DOUBLE PRECISION`), `city`, `postcode`, `state_standardized` (`TEXT`)

### `stg_merchants`
`osm_id` (`BIGINT`), `h3_index` (`TEXT`), `merchant_name`, `latitude`, `longitude`, `city`, `postcode`, `state`, `merchant_category`, `risk_level` (all `TEXT`)

### `mart_residential`
`osm_id`, `h3_index`, `latitude`, `longitude`, `city`, `pincode`, `state`, `district_name`

### `mart_merchants`
`mart_residential` fields + `merchant_name`, `merchant_category`, `risk_level`

### `mart_district_summary`
`state`, `district_name` (`TEXT`), `residential_nodes`, `merchant_nodes`, `high_risk_merchants`, `medium_high_risk_merchants` (`INTEGER`), `merchant_to_residential_ratio` (`NUMERIC`)

</details>

## Architecture

### Spatial Join Strategy
Both mart models use `ST_Intersects` against DataMeet administrative boundaries (`ref_boundaries_states`, `ref_boundaries_districts`) to assign verified state/district names, bypassing inconsistent OSM `addr:state` tags.

### Category & Risk Mapping
`stg_merchants` joins raw OSM `sub_category` against a `ref_category_map` seed table to produce standardized `merchant_category` and `risk_level`. Unmatched values default to `GENERAL_RETAIL` / `LOW`. The fraud engine filters on `risk_level` in Parquet rather than embedding risk logic in Rust.

### City Normalization
The `normalize_city` macro strips appended state/city names (regex), drops postal suffixes (truncate at dash), and removes compound fragments (truncate at comma). Uppercased and trimmed. Best-effort heuristic — city is a label, not authoritative.

### Indexing
Btree indexes on `state`, `district_name`, `h3_index`, `pincode` (all marts) + `merchant_category` (`mart_merchants`). Applied at materialization time to keep `export_references.rs` queries proportional to result size.

## Current Limitations

`ST_Intersects` scans every boundary polygon on every run with no spatial index — GIST indexes on the boundary tables would fix this. The `normalize_city` macro uses a hardcoded name list that can't handle Devanagari variants. `mart_district_summary` has no downstream consumers and no automated validation.
