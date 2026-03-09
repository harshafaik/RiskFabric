{{ config(
    materialized='table',
    indexes=[
      {'columns': ['merchant_category'], 'type': 'btree'},
      {'columns': ['risk_level'], 'type': 'btree'}
    ]
) }}

select 
    osm_id,
    h3_index,
    merchant_name,
    latitude,
    longitude,
    merchant_category,
    risk_level
from {{ ref('stg_merchants') }}
