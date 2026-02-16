{{ config(
    materialized='table',
    indexes=[
      {'columns': ['final_state'], 'type': 'btree'},
      {'columns': ['h3_index'], 'type': 'btree'}
    ]
) }}

with residential as (
    select * from {{ ref('stg_osm__residential') }}
),

state_boundaries as (
    select 
        st_nm as state_name, 
        wkb_geometry as geom 
    from {{ source('public', 'ref_india_states') }}
),

spatial_match as (
    select 
        r.osm_id,
        r.h3_index,
        r.latitude,
        r.longitude,
        r.city,
        r.postcode,
        -- Step 1: Resolve the Unknowns using the Map
        case 
            when r.state_standardized = 'Unknown' then (
                select b.state_name 
                from state_boundaries b 
                where ST_Contains(b.geom, ST_SetSRID(ST_Point(r.longitude, r.latitude), 4326))
                limit 1
            )
            else r.state_standardized
        end as raw_state_name
    from residential r
)

-- Step 2: Clean up the duplicates in the final output
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
from spatial_match
