# Customer Data Generator

## Overview
The customer generator module `customer_gen.rs` is responsible for generating a synthetic group of customer profiles, using the geographic data provided from OSM reference points as well as financial behavioral profiles dialed in using YAML configurations. 

## Schema

Each customer profile consists of several sub-profiles holding geographical, financial, and device metadata:

<div style="max-width: 500px; margin: 0 auto;">

```text
Customer ◄── GeoLocation
Customer ◄── FinancialProfile
Customer ◄── DeviceProfile
```
</div>
<details>
<summary><code>Customer</code></summary>

| Field Name | Type | Description |
| :--- | :--- | :--- |
| `customer_id` | `String` | Unique UUID v4 identifying the customer. |
| `name` | `String` | Generated first and last name (from configuration names pool). |
| `age` | `u8` | Age of the customer (stochastically chosen between 18 and 85). |
| `email` | `String` | Synthetic email address generated based on name and domains pool. |
| `location` | `GeoLocation` | Embedded structure representing the customer's geographic data. |
| `financial` | `FinancialProfile` | Embedded structure representing credit score, spend limits, and risk metrics. |
| `device` | `DeviceProfile` | Embedded structure representing client agent and ISP/IP subnet details. |
| `registration_date` | `String` | ISO 8601 date string (`"YYYY-MM-DD"`) representing when the customer registered. |
| `registration_year` | `i32` | Year portion of the registration date. |
| `registration_month` | `u32` | Month portion of the registration date. |
| `registration_day` | `u32` | Day portion of the registration date. |

</details>

<details>
<summary><code>GeoLocation</code></summary>

| Field Name | Type | Description |
| :--- | :--- | :--- |
| `location` | `String` | Full text residential address (composed of building number, street name, city, and pincode). |
| `city` | `Option<String>` | Name of the city/town from OSM reference data (if available). |
| `state` | `String` | Name of the state (derived and normalized from reference data). |
| `location_type` | `String` | Proximity classification (`"Metro"`, `"Urban"`, `"Rural"`). |
| `postcode` | `Option<String>` | Normalized postal code (pincode). |
| `home_latitude` | `f64` | Latitude coordinate. |
| `home_longitude` | `f64` | Longitude coordinate |
| `home_h3r5` | `String` | H3 Index at Resolution 5. |
| `home_h3r7` | `String` | H3 Index at Resolution 7. |

</details>

<details>
<summary><code>FinancialProfile</code></summary>

| Field Name | Type | Description |
| :--- | :--- | :--- |
| `credit_score` | `u16` | Normalized credit score correlated with customer age and bounded by config limits. |
| `monthly_spend` | `f64` | Baseline monthly spend limit correlated with age spend curve and location type. |
| `customer_risk_score` | `f32` | Probability score representing the default risk level of the customer. |
| `is_fraud` | `bool` | Flag designating if this customer profile is simulated as compromised/fraudulent. |

</details>

<details>
<summary><code>DeviceProfile</code></summary>

| Field Name | Type | Description |
| :--- | :--- | :--- |
| `primary_ua` | `String` | Primary User Agent string (Android, iOS, desktop, or UPI app, chosen via share weights). |
| `secondary_ua` | `Option<String>` | Optional secondary User Agent string (simulating multi-device ownership, present in 15% of agents). |
| `isp` | `String` | Assigned Internet Service Provider (derived from location type share weights). |
| `ip_subnet` | `String` | CIDR IP subnet mask assigned to the customer based on their ISP. |

</details>


To ensure realistic correlation between customer behavior, the module uses relations configured in the form of YAML configurations such as programmatically linking credit score to age, monthly spend vis-a-vis to location_type (Metro vs Rural). This ensures that customer profiles resemble structural patterns consistently similar to equivalent real-world financial data.

The module's decision to pick a residential location from the exported residential addresses parquet file is based on location_type which also includes a jitter of ~500m to the original residential nodes. This prevents "clumping" where multiple customers would otherwise share identical coordinates. As of now, the addition of jitter is limited to a strict numeric figure but later can be designated on the basis of location_type since Metro and Urban cities have traditionally denser clustering compared to Rural location types.

`customer_gen.rs` acts as the first stage of the data generation pipeline. It uses the `ref_residential.parquet` file for referencing residential nodes and the `customer_config.yaml` configuration for referencing customer behavior. The generated dataset is passed downstream to the account and card generators to complete the entity hierarchy.

## Known Issues
The entire residential reference dataset is currently loaded into memory using Polars' `ParquetReader` for every generation run. While efficient for populations up to 100,000 customers, this creates a significant memory bottleneck when scaling to millions of agents. Moving to a chunked or streaming approach for reading reference data is required. Additionally, the jitter range (0.005) is currently hardcoded in the source code; moving this to the configuration would allow for different levels of spatial precision.
