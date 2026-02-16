{{ config(materialized='table') }}

with raw_data as (
    -- Selecting from your raw table
    select 
        osm_id,
        h3_index,
        -- Clean up the name: Title Case, and fill NULLs with the category
        coalesce(initcap(name), initcap(sub_category), 'Unknown Merchant') as merchant_name,
        category as raw_category,
        sub_category as raw_sub_category,
        lat,
        lon
    from {{ source('public', 'raw_merchants') }}
),

category_map as (
    select * from {{ ref('merchant_category_map') }}
)

select 
    r.osm_id,
    r.h3_index,
    r.merchant_name,
    r.lat,
    r.lon,
    -- Map the risk category
    coalesce(m.standardized_category, 'GENERAL_RETAIL') as merchant_category,
    coalesce(m.risk_level, 'LOW') as risk_level
from raw_data r
left join category_map m 
    on trim(lower(r.raw_sub_category)) = trim(lower(m.sub_category))
