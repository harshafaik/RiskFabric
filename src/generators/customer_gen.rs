use crate::models::customer::Customer;
use polars::prelude::*;
use rand::Rng;
use rayon::prelude::*;
use std::fs::File;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct CustomerConfig {
    names: NamesConfig,
    email: EmailConfig,
    locations: LocationsConfig,
    financials: FinancialsConfig,
}

#[derive(Debug, Deserialize)]
struct NamesConfig {
    first_names: Vec<String>,
    last_names: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct EmailConfig {
    domains: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct LocationsConfig {
    types: Vec<String>,
    metro_cities: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FinancialsConfig {
    base_spend: HashMap<String, f64>,
    credit_score: CreditScoreConfig,
}

#[derive(Debug, Deserialize)]
struct CreditScoreConfig {
    base: i32,
    age_weight: f64,
    min: u16,
    max: u16,
}

pub fn generate_customers(count: usize) -> Vec<Customer> {
    println!("   ... loading customer configuration and residential reference data");
    
    let config_file = File::open("data/config/customer_config.yaml")
        .expect("Could not open customer_config.yaml");
    let config: CustomerConfig = serde_yaml::from_reader(config_file)
        .expect("Failed to parse customer_config.yaml");

    let ref_file = File::open("data/processed/residential_enriched.parquet")
        .expect("Could not open residential_enriched.parquet");
    
    let df = ParquetReader::new(ref_file)
        .finish()
        .expect("Failed to read Parquet");

    let h3_indices = df.column("h3_index").unwrap().str().unwrap().into_no_null_iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let lats = df.column("latitude").unwrap().f64().unwrap().into_no_null_iter().collect::<Vec<_>>();
    let lons = df.column("longitude").unwrap().f64().unwrap().into_no_null_iter().collect::<Vec<_>>();
    let states = df.column("final_state").unwrap().str().unwrap().into_no_null_iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let cities = df.column("city").unwrap().str().unwrap().into_iter().map(|opt_s| opt_s.map(|s| s.to_string())).collect::<Vec<_>>();

    let ref_count = h3_indices.len();
    println!("   ... dispatching threads for {} customers using {} reference points", count, ref_count);

    (0..count)
        .into_par_iter()
        .map(|_| {
            let mut rng = rand::rng();
            let idx = rng.random_range(0..ref_count);
            
            let first_name = &config.names.first_names[rng.random_range(0..config.names.first_names.len())];
            let last_name = &config.names.last_names[rng.random_range(0..config.names.last_names.len())];
            let name = format!("{} {}", first_name, last_name);
            
            let domain = &config.email.domains[rng.random_range(0..config.email.domains.len())];
            let email = format!("{}.{}{}@{}", 
                first_name.to_lowercase(), 
                last_name.to_lowercase(), 
                rng.random_range(10..999),
                domain
            );

            let age: u8 = rng.random_range(18..85);
            let customer_id = uuid::Uuid::new_v4().to_string();

            // 1. Infer location type
            let city_name = cities[idx].as_deref().unwrap_or("");
            let location_type = if config.locations.metro_cities.iter().any(|m| m.to_lowercase() == city_name.to_lowercase()) {
                "Metro".to_string()
            } else if !city_name.is_empty() {
                "Urban".to_string()
            } else {
                let types = &config.locations.types;
                types[rng.random_range(0..types.len())].clone()
            };

            // 2. Correlate Credit Score with Age
            let base_cs = config.financials.credit_score.base as f64;
            let age_factor = (age as f64 - 18.0) * config.financials.credit_score.age_weight;
            let noise = rng.random_range(-50.0..50.0);
            let credit_score = (base_cs + age_factor + noise).clamp(
                config.financials.credit_score.min as f64, 
                config.financials.credit_score.max as f64
            ) as u16;

            // 3. Correlate Monthly Spend with Location and Age
            let base_spend = config.financials.base_spend.get(&location_type).unwrap_or(&15000.0);
            // Spend curve: peaks at age 45
            let age_spend_multiplier = 1.0 - ((age as f64 - 45.0).abs() / 60.0); 
            let spend_noise = rng.random_range(0.7..1.4);
            let monthly_spend = base_spend * age_spend_multiplier * spend_noise;

            // 4. Fraud Flags (Independent for now, but placeholders for logic)
            let is_fraud = rng.random_bool(0.02);
            let customer_risk_score = if is_fraud {
                rng.random_range(0.6..0.99)
            } else {
                rng.random_range(0.01..0.6)
            } as f32;

            let mut customer = Customer::new(
                customer_id,
                name,
                age,
                email,
                states[idx].clone(),
                cities[idx].clone(),
                lats[idx],
                lons[idx],
                h3_indices[idx].clone(),
                credit_score,
                monthly_spend,
                customer_risk_score,
                is_fraud,
            );
            
            customer.location_type = location_type;
            customer
        })
        .collect()
}
