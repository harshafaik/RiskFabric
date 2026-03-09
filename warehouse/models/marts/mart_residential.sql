{{ config(
    materialized='table',
    indexes=[
      {'columns': ['final_state'], 'type': 'btree'},
      {'columns': ['h3_index'], 'type': 'btree'}
    ]
) }}

with residential as (
    select * from {{ ref('stg_residential') }}
),

state_boundaries as (
    select 
        st_nm as state_name, 
        wkb_geometry as geom 
    from {{ source('reference', 'ref_boundaries') }}
),

known_states as (
    select *, state_standardized as raw_state_name
    from residential
    where state_standardized != 'Unknown'
),

unknown_states as (
    select r.*, b.state_name as raw_state_name
    from residential r
    left join state_boundaries b 
        on ST_Intersects(b.geom, ST_SetSRID(ST_Point(r.longitude, r.latitude), 4326))
    where r.state_standardized = 'Unknown'
),

combined as (
    select * from known_states
    union all
    select * from unknown_states
)

select
    osm_id,
    h3_index,
    latitude,
    longitude,
    city,
    postcode,
    case 
        when raw_state_name in ('Jammu and Kashmir', 'Ladakh') then 'Jammu & Kashmir'
        when raw_state_name ilike '%Punjab%' then 'Punjab'
        when raw_state_name ilike '%Maharashtra%' then 'Maharashtra'
        when raw_state_name is null then 'Unknown'
        else raw_state_name
    end as final_state
from combined
