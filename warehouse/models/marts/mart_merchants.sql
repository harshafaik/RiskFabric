{{ config(
    materialized='table',
    indexes=[
      {'columns': ['state'], 'type': 'btree'},
      {'columns': ['district_name'], 'type': 'btree'},
      {'columns': ['h3_index'], 'type': 'btree'},
      {'columns': ['merchant_category'], 'type': 'btree'},
      {'columns': ['pincode'], 'type': 'btree'}
    ]
) }}

with merchants as (
    select * from {{ ref('stg_merchants') }}
),

states as (
    select 
        st_nm as state_name, 
        geom 
    from {{ source('reference', 'ref_boundaries_states') }}
),

districts as (
    select 
        district as district_name,
        st_nm as state_name,
        geom
    from {{ source('reference', 'ref_boundaries_districts') }}
)

select
    m.osm_id,
    m.h3_index,
    m.latitude,
    m.longitude,
    m.merchant_name,
    m.merchant_category,
    m.risk_level,
    {{ normalize_city('m.city') }} as city,
    -- Pincode Cleaning: Keep only digits
    regexp_replace(m.postcode, '[^0-9]', '', 'g') as pincode,
    -- Official Enrichment from DataMeet
    s.state_name as state,
    d.district_name
from merchants m
left join states s 
    on ST_Intersects(s.geom, ST_SetSRID(ST_Point(m.longitude, m.latitude), 4326))
left join districts d 
    on ST_Intersects(d.geom, ST_SetSRID(ST_Point(m.longitude, m.latitude), 4326))
