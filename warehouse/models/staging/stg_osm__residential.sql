{{ config(
    materialized='table',
    indexes=[
      {'columns': ['state_standardized']},
      {'columns': ['h3_index'], 'type': 'btree'}
    ]
) }}

with raw_data as (
    select * from {{ source('raw_osm', 'raw_residential') }}
),

mapping as (
    select * from {{ ref('state_map') }}
)

select 
    r.osm_id,
    r.h3_index,
    r.lat as latitude,
    r.lon as longitude,
    r.city,
    r.postcode,
    -- The State Fix: Match messy state names to the clean seed
    coalesce(m.clean_name, r.state, 'Unknown') as state_standardized
from raw_data r
left join mapping m 
    on trim(upper(r.state)) = trim(upper(m.raw_name))
