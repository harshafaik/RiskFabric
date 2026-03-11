use crate::models::customer::{Customer, GeoLocation, FinancialProfile};
use crate::config::AppConfig;
use polars::prelude::*;
use rand::Rng;
use rayon::prelude::*;
use std::fs::File;

pub fn generate_customers(count: usize) -> Vec<Customer> {
    println!("   ... loading customer configuration and residential reference data");
    let config = AppConfig::load();
    
    let ref_file = File::open("data/references/ref_residential.parquet")
        .expect("Could not open ref_residential.parquet");
    
    let df = ParquetReader::new(ref_file)
        .finish()
        .expect("Failed to read Parquet");

    let h3_indices = df.column("h3_index").unwrap().str().unwrap().into_no_null_iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let lats = df.column("latitude").unwrap().f64().unwrap().into_no_null_iter().collect::<Vec<_>>();
    let lons = df.column("longitude").unwrap().f64().unwrap().into_no_null_iter().collect::<Vec<_>>();
    let states = df.column("state").unwrap().str().unwrap().into_no_null_iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let cities = df.column("city").unwrap().str().unwrap().into_iter().map(|opt_s| opt_s.map(|s| s.to_string())).collect::<Vec<_>>();
    let postcodes = df.column("postcode").unwrap().str().unwrap().into_iter().map(|opt_s| opt_s.map(|s| s.to_string())).collect::<Vec<_>>();

    let ref_count = h3_indices.len();
    println!("   ... dispatching threads for {} customers using {} reference points", count, ref_count);

    (0..count)
        .into_par_iter()
        .map(|_| {
            let mut rng = rand::rng();
            let idx = rng.random_range(0..ref_count);
            
            let first_name = &config.customer.names.first_names[rng.random_range(0..config.customer.names.first_names.len())];
            let last_name = &config.customer.names.last_names[rng.random_range(0..config.customer.names.last_names.len())];
            let name = format!("{} {}", first_name, last_name);
            
            let domain = &config.customer.email.domains[rng.random_range(0..config.customer.email.domains.len())];
            let email = format!("{}.{}{}@{}", 
                first_name.to_lowercase(), 
                last_name.to_lowercase(), 
                rng.random_range(10..999),
                domain
            );

            let age: u8 = rng.random_range(18..85);
            let customer_id = uuid::Uuid::new_v4().to_string();

            // 1. Spatial Jittering: Introduce a small drift (~500m) to avoid exact node overlays
            let jitter_lat = rng.random_range(-0.005..0.005);
            let jitter_lon = rng.random_range(-0.005..0.005);
            let final_lat = lats[idx] + jitter_lat;
            let final_lon = lons[idx] + jitter_lon;

            // 2. Infer location type
            let city_name = cities[idx].as_deref().unwrap_or("");
            let location_type = if config.customer.locations.metro_cities.iter().any(|m| m.to_lowercase() == city_name.to_lowercase()) {
                "Metro".to_string()
            } else if !city_name.is_empty() {
                "Urban".to_string()
            } else {
                let types = &config.customer.locations.types;
                types[rng.random_range(0..types.len())].clone()
            };

            // 3. Correlate Credit Score with Age
            let base_cs = config.customer.financials.credit_score.base as f64;
            let age_factor = (age as f64 - 18.0) * config.customer.financials.credit_score.age_weight;
            let noise = rng.random_range(-50.0..50.0);
            let credit_score = (base_cs + age_factor + noise).clamp(
                config.customer.financials.credit_score.min as f64, 
                config.customer.financials.credit_score.max as f64
            ) as u16;

            // 4. Correlate Monthly Spend with Location and Age
            let base_spend = config.customer.financials.base_spend.get(&location_type).unwrap_or(&15000.0);
            // Spend curve: peaks at age 45
            let age_spend_multiplier = 1.0 - ((age as f64 - 45.0).abs() / 60.0); 
            let spend_noise = rng.random_range(0.7..1.4);
            let monthly_spend = base_spend * age_spend_multiplier * spend_noise;

            // 5. Fraud Flags
            let is_fraud = rng.random_bool(0.02);
            let customer_risk_score = if is_fraud {
                rng.random_range(0.6..0.99)
            } else {
                rng.random_range(0.01..0.6)
            } as f32;

            
            
            Customer::new(
                customer_id,
                name,
                age,
                email,
                GeoLocation {
                    location: "".to_string(), // Built inside constructor
                    city: cities[idx].clone(),
                    state: states[idx].clone(),
                    location_type,
                    home_latitude: final_lat,
                    home_longitude: final_lon,
                    home_h3r5: "".to_string(), // Built inside constructor
                    home_h3r7: h3_indices[idx].clone(),
                },
                postcodes[idx].clone(),
                FinancialProfile {
                    credit_score,
                    monthly_spend,
                    customer_risk_score,
                    is_fraud,
                },
            )
        })
        .collect()
}
