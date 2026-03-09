{{ config(
    materialized='table'
) }}

with residential_counts as (
    select 
        state, 
        district_name, 
        count(*) as residential_nodes
    from {{ ref('mart_residential') }}
    group by 1, 2
),

merchant_counts as (
    select 
        state, 
        district_name, 
        count(*) as merchant_nodes,
        count(case when risk_level = 'VERY_HIGH' then 1 end) as high_risk_merchants,
        count(case when risk_level = 'HIGH' then 1 end) as medium_high_risk_merchants
    from {{ ref('mart_merchants') }}
    group by 1, 2
)

select
    coalesce(r.state, m.state) as state,
    coalesce(r.district_name, m.district_name) as district_name,
    coalesce(r.residential_nodes, 0) as residential_nodes,
    coalesce(m.merchant_nodes, 0) as merchant_nodes,
    coalesce(m.high_risk_merchants, 0) as high_risk_merchants,
    coalesce(m.medium_high_risk_merchants, 0) as medium_high_risk_merchants,
    -- Simple density metric
    case 
        when coalesce(r.residential_nodes, 0) > 0 
        then round(cast(coalesce(m.merchant_nodes, 0) as numeric) / r.residential_nodes, 2)
        else 0 
    end as merchant_to_residential_ratio
from residential_counts r
full outer join merchant_counts m 
    on r.state = m.state 
    and r.district_name = m.district_name
order by state, merchant_nodes desc
