{{ config(
    materialized='table',
    indexes=[
      {'columns': ['state'], 'type': 'btree'},
      {'columns': ['district_name'], 'type': 'btree'},
      {'columns': ['h3_index'], 'type': 'btree'},
      {'columns': ['pincode'], 'type': 'btree'}
    ]
) }}

with residential as (
    select * from {{ ref('stg_residential') }}
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
    r.osm_id,
    r.h3_index,
    r.latitude,
    r.longitude,
    {{ normalize_city('r.city') }} as city,
    -- Pincode Cleaning: Keep only digits
    regexp_replace(r.postcode, '[^0-9]', '', 'g') as pincode,
    -- Official Enrichment from DataMeet
    s.state_name as state,
    d.district_name
from residential r
left join states s 
    on ST_Intersects(s.geom, ST_SetSRID(ST_Point(r.longitude, r.latitude), 4326))
left join districts d 
    on ST_Intersects(d.geom, ST_SetSRID(ST_Point(r.longitude, r.latitude), 4326))
