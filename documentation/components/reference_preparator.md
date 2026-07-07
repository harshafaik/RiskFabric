# Reference Data Preparator

## Overview

The reference data preparator (`prepare_refs.rs`) is the world-building binary that converts raw OpenStreetMap (OSM) PBF data into the structured geographic staging tables used by the simulation generators. It exposes six subcommands — `extract-nodes`, `map-city-state`, `parse-districts`, `map-state-districts`, `normalize-states`, and `compare-city-district` — which collectively produce three Postgres staging tables (`raw_residential`, `raw_merchants`, `raw_financial`) populated from the India OSM PBF file. These tables are consumed by `export_references.rs`, which serializes them to Parquet for use by `generate.rs`, `stream.rs`, and `customer_gen.rs`.

## Schema

### `ResidentialPoint` (internal struct → `raw_residential`)

| Field Name | Type | Description |
| :--- | :--- | :--- |
| `osm_id` | `i64` | OSM node identifier for the residential point. |
| `h3_index` | `String` | H3 index at Resolution 8, computed from the node's latitude and longitude. |
| `lat` | `f64` | Latitude coordinate of the node. |
| `lon` | `f64` | Longitude coordinate of the node. |
| `city` | `Option<String>` | City name extracted from the `addr:city` OSM tag; null if the tag is absent. |
| `postcode` | `Option<String>` | Postal code extracted from the `addr:postcode` OSM tag; null if the tag is absent. |
| `state` | `Option<String>` | State name extracted from the `addr:state` OSM tag; null if the tag is absent. |

A node is classified as residential if it carries a `building=residential` or `landuse=residential` tag, or if it has either `addr:housenumber` or `addr:street` present and is not already classified as a merchant.

### `MerchantPoint` (internal struct → `raw_merchants`)

| Field Name | Type | Description |
| :--- | :--- | :--- |
| `osm_id` | `i64` | OSM node identifier for the merchant point. |
| `h3_index` | `String` | H3 index at Resolution 8, computed from the node's latitude and longitude. |
| `name` | `String` | Merchant name from the `name` OSM tag; defaults to `"Unknown Merchant"` if absent. |
| `category` | `String` | Top-level OSM tag type: `"shop"`, `"amenity"`, or `"tourism"`. |
| `sub_category` | `String` | Raw OSM tag value (e.g., `"jewelry"`, `"restaurant"`, `"hotel"`), used by dbt for risk level mapping. |
| `lat` | `f64` | Latitude coordinate of the node. |
| `lon` | `f64` | Longitude coordinate of the node. |
| `city` | `Option<String>` | City name from `addr:city`; null if absent. |
| `postcode` | `Option<String>` | Postal code from `addr:postcode`; null if absent. |
| `state` | `Option<String>` | State name from `addr:state`; null if absent. |

Merchant classification matches `shop=*` nodes (all sub-categories), `amenity` nodes restricted to `restaurant`, `cafe`, `fast_food`, `bar`, `pub`, `fuel`, `cinema`, and `pharmacy`, and `tourism` nodes restricted to `hotel`, `motel`, and `guest_house`.

### `FinancialPoint` (internal struct → `raw_financial`)

| Field Name | Type | Description |
| :--- | :--- | :--- |
| `osm_id` | `i64` | OSM node identifier for the financial point. |
| `h3_index` | `String` | H3 index at Resolution 8, computed from the node's latitude and longitude. |
| `kind` | `String` | Entity type: `"atm"` or `"bank"`, derived from the `amenity` OSM tag. |
| `operator` | `Option<String>` | Institution name from the `operator` or `brand` OSM tag; null if both are absent. |
| `lat` | `f64` | Latitude coordinate of the node. |
| `lon` | `f64` | Longitude coordinate of the node. |

**Parallel Map-Reduce Extraction** is the core performance mechanism of the `extract-nodes` subcommand. The `osmpbf` library's `par_map_reduce` function distributes OSM node processing across all available CPU cores using `rayon`. Each thread processes a local batch of `ResidentialPoint`, `MerchantPoint`, and `FinancialPoint` records, and the results are merged into a single collection after all threads complete. This allows the full India PBF file (several gigabytes) to be scanned in minutes rather than hours on a multi-core workstation.

**Binary Copy Insertion** is used to write extracted records into Postgres. Rather than issuing individual `INSERT` statements, the preparator uses `postgres::binary_copy::BinaryCopyInWriter` to stream all records for each table in a single `COPY FROM STDIN BINARY` operation. This bypasses SQL parsing overhead entirely and is the dominant factor in keeping Postgres insert time proportional to record count rather than to number of statements.

**H3 Indexing at Resolution 8** is assigned to every extracted node at extraction time, not deferred to the dbt layer. The `h3o` library converts each node's `(lat, lon)` pair into an H3 cell at Resolution 8 (~0.74 km² average area). This index is carried through all downstream tables and Parquet files, enabling the simulation generators to perform proximity-based merchant selection using H3 parent cell lookups without re-computing coordinates at runtime.

**State Normalization Utilities** are provided as separate subcommands (`map-city-state`, `parse-districts`, `normalize-states`) rather than being integrated into the main extraction pass. `map-city-state` produces a ranked city-to-state frequency report from `addr:city`/`addr:state` tag co-occurrences across the full PBF. `parse-districts` extracts India administrative level-5 relations using `ISO3166-2` and `is_in:state` tags. These utilities produce intermediate report files that inform manual or automated normalization rules, which are then applied in the dbt `staging` layer.

`prepare_refs.rs` is a standalone Level 0 utility and is the first step in the reference data pipeline. It must be run before `export_references.rs` and before any dbt models. Its outputs — `raw_residential`, `raw_merchants`, and `raw_financial` — are the primary inputs to the dbt `staging` models, which apply spatial joins and risk categorization before producing the final `mart_residential` and `mart_merchants` tables.

## Known Issues

The Postgres connection string is hardcoded as a CLI default value (`postgres://harshafaik:123@localhost:5432/riskfabric`) in the `extract-nodes` subcommand argument definition. This embeds a plaintext credential and a machine-specific hostname directly in the binary's help output and source code. The connection string must be moved to an environment variable or a `.env` file, and the hardcoded default must be removed, before this binary can be safely shared or deployed in any non-local environment.

The `extract-nodes` subcommand loads the entire extracted dataset into memory inside three `Arc<Mutex<Vec<_>>>` collections before writing to Postgres. For the India PBF file, this can accumulate millions of residential and merchant records simultaneously, with no streaming or chunked flush to Postgres during the scan. On systems with limited RAM, this creates a risk of OOM termination partway through extraction with no partial state recoverable. Switching to periodic flushes — writing to Postgres every N records and clearing the in-memory buffer — would bound the peak memory footprint regardless of dataset size.

The `prepare_refs.rs` binary stops at Postgres population and has no built-in export step. Producing the final Parquet files requires running `export_references.rs` as a separate binary after all dbt models have been executed. This two-binary, three-step workflow (prepare → dbt run → export) has no orchestration layer to enforce ordering or detect if any step was skipped. A single unified subcommand, or a lightweight pipeline runner, is needed to make the end-to-end world-building process reliable and repeatable.
