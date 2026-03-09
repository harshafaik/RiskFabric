{{ config(
    indexes=[
      {'columns': ['merchant_category'], 'type': 'btree'},
      {'columns': ['risk_level'], 'type': 'btree'}
    ]
) }}

with raw_merchants as (
    select * from {{ source('raw_osm', 'raw_merchants') }}
),

category_map as (
    select * from {{ ref('ref_category_map') }}
)

select 
    m.osm_id,
    m.h3_index,
    m.name as merchant_name,
    m.lat as latitude,
    m.lon as longitude,
    m.city,
    m.postcode,
    m.state,
    coalesce(cm.standardized_category, 'GENERAL_RETAIL') as merchant_category,
    coalesce(cm.risk_level, 'LOW') as risk_level
from raw_merchants m
left join category_map cm 
    on trim(lower(m.sub_category)) = trim(lower(cm.sub_category))
