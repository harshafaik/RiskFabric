use crate::models::card::Card;
use crate::models::customer::Customer;
use crate::models::transaction::Transaction;
use crate::models::fraud_metadata::FraudMetadata;
use crate::config::AppConfig;
use polars::prelude::*;
use rand::{Rng, SeedableRng, rngs::StdRng};
use rayon::prelude::*;
use std::fs::File;
use std::collections::HashMap;
use h3o::{CellIndex, Resolution};
use std::str::FromStr;
use chrono::{Utc, Duration};

pub fn generate_transactions(cards: &Vec<Card>, customers: &Vec<Customer>) -> (Vec<Transaction>, Vec<FraudMetadata>) {
    let config = AppConfig::load();
    
    // 1. Load Merchants
    let file = File::open("data/references/ref_merchants.parquet").expect("Merchant data missing");
    let df = ParquetReader::new(file).finish().expect("Failed to read Parquet");

    let h3_indices: Vec<String> = df.column("h3_index").unwrap().str().unwrap().into_no_null_iter().map(|s| s.to_string()).collect();
    let names: Vec<String> = df.column("merchant_name").unwrap().str().unwrap().into_no_null_iter().map(|s| s.to_string()).collect();
    let lats: Vec<f64> = df.column("lat").unwrap().f64().unwrap().into_no_null_iter().collect();
    let lons: Vec<f64> = df.column("lon").unwrap().f64().unwrap().into_no_null_iter().collect();
    let categories: Vec<String> = df.column("merchant_category").unwrap().str().unwrap().into_no_null_iter().map(|s| s.to_string()).collect();
    let osm_ids: Vec<i64> = df.column("osm_id").unwrap().i64().unwrap().into_no_null_iter().collect();
    let ref_count = h3_indices.len();

    // 2. Spatial Index
    let mut spatial_index_res5: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, h3_str) in h3_indices.iter().enumerate() {
        if let Ok(cell) = CellIndex::from_str(h3_str) {
            let p5 = cell.parent(Resolution::Five).unwrap().to_string();
            spatial_index_res5.entry(p5).or_default().push(idx);
        }
    }

    // 3. Customer Map
    let customer_map: HashMap<String, &Customer> = customers.iter().map(|c| (c.customer_id.clone(), c)).collect();

    println!("   ... generating transactions, fraud, and campaigns in a single pass (Tuned)");

    // Pre-calculate base timestamp
    let base_end_date = Utc::now();
    let base_start_date = base_end_date - Duration::days(365);
    let total_seconds = (base_end_date - base_start_date).num_seconds();

    let results: Vec<(Vec<Transaction>, Vec<FraudMetadata>)> = cards
        .par_iter()
        .map(|card| {
            let mut local_txs = Vec::new();
            let mut local_meta = Vec::new();
            
            // Seed RNG per card for stability using Configured Salt
            let mut card_rng = StdRng::seed_from_u64(config.rules.global.seed as u64 + config.tuning.salts.injector as u64 + card.card_id.as_bytes().iter().map(|&b| b as u64).sum::<u64>());
            
            let customer = customer_map.get(&card.customer_id).expect("Customer missing");
            let cust_cell = CellIndex::from_str(&customer.home_h3r7).expect("H3 invalid");
            let p5_key = cust_cell.parent(Resolution::Five).unwrap().to_string();

            // Determine if this CARD is targeted by a campaign
            let mut camp_id = None;
            let mut camp_type = None;
            let share = config.rules.fraud_campaigns.values().filter_map(|c| c.target_campaign_share).next().unwrap_or(0.15);
            if card_rng.random_bool(share) {
                let campaigns: Vec<&String> = config.rules.fraud_campaigns.keys().collect();
                let c_type = campaigns[card_rng.random_range(0..campaigns.len())].clone();
                camp_id = Some(format!("camp_{}_{}", c_type, &card.card_id[..8]));
                camp_type = Some(c_type);
            }

            let num_txns = card_rng.random_range(5..20);
            for i in 0..num_txns {
                // 1. SELECT MERCHANT
                let idx = if card_rng.random_bool(0.98) {
                    if let Some(indices) = spatial_index_res5.get(&p5_key) {
                        indices[card_rng.random_range(0..indices.len())]
                    } else {
                        card_rng.random_range(0..ref_count)
                    }
                } else {
                    card_rng.random_range(0..ref_count)
                };

                // 2. GENERATE BASE FIELDS
                let tx_id = format!("tx_{}_{}_{}", &card.card_id[..8], i, card_rng.random_range(1000..9999));
                let amount = card_rng.random_range(10.0..50000.0);
                let tx_date = base_start_date + Duration::seconds(card_rng.random_range(0..total_seconds));
                
                // 3. FRAUD LOGIC (Embedded)
                let r_target: f64 = card_rng.random();
                let fraud_target = r_target < config.rules.fraud_injector.target_share;

                let r_fn: f64 = card_rng.random();
                let r_fp: f64 = card_rng.random();
                let mut is_fraud_label = fraud_target;
                let mut label_noise = "none".to_string();

                if fraud_target && r_fn < config.rules.fraud_injector.default_fn_rate {
                    is_fraud_label = false;
                    label_noise = "fn".to_string();
                } else if !fraud_target && r_fp < config.rules.fraud_injector.default_fp_rate {
                    is_fraud_label = true;
                    label_noise = "fp".to_string();
                }

                // Mutations
                let mut final_amount = amount;
                let mut final_channel = "online".to_string();
                let mut final_ua = "Mozilla/5.0 (iPhone; CPU iPhone OS 15_4 like Mac OS X) [PhonePe/4.2]".to_string();
                let mut final_country = config.rules.global.default_country.clone();
                let mut final_lat = lats[idx];
                let mut final_lon = lons[idx];
                let mut final_ip = format!("103.21.{}.{}", card_rng.random_range(1..255), card_rng.random_range(1..255));
                let mut geo_anomaly = false;
                let mut device_anomaly = false;
                let mut ip_anomaly = false;
                let mut final_status = "Success".to_string();
                let mut auth_status = "approved".to_string();
                let mut failure_reason = None;

                if fraud_target {
                    let profiles: Vec<&String> = config.rules.fraud_injector.profiles.keys().collect();
                    let f_type = profiles[card_rng.random_range(0..profiles.len())];
                    let profile = &config.rules.fraud_injector.profiles[f_type];
                    
                    if let Some(amounts) = config.rules.fraud_patterns.get(&profile.amount_pattern) {
                        final_amount = amounts[card_rng.random_range(0..amounts.len())];
                    }
                    
                    if card_rng.random_bool(config.tuning.probabilities.geo_anomaly) {
                        geo_anomaly = true;
                        // Domestic Geo-Anomaly: Keep country as IN but jump to a random location in India
                        final_lat = card_rng.random_range(8.0..37.0);
                        final_lon = card_rng.random_range(68.0..97.0);
                    }
                    
                    if card_rng.random_bool(config.tuning.probabilities.device_anomaly) {
                        final_ua = config.rules.device_patterns.bot_user_agent_prefix.clone();
                        device_anomaly = true;
                    }

                    if card_rng.random_bool(config.tuning.probabilities.failure) {
                        final_status = "Failed".to_string();
                        auth_status = "declined".to_string();
                        
                        if let Some(reasons) = config.rules.failure_reasons_by_type.get(f_type) {
                            failure_reason = Some(reasons[card_rng.random_range(0..reasons.len())].clone());
                        } else {
                            failure_reason = Some(config.tuning.defaults.fallback_failure_reason.clone());
                        }
                    }

                    // Campaign Mutations
                    if let (Some(_c_id), Some(c_type)) = (&camp_id, &camp_type) {
                        if c_type == "coordinated_upi_scam" {
                            final_ip = config.tuning.campaigns.coordinated_scam_ip.clone();
                            final_ua = config.rules.device_patterns.bot_user_agent_prefix.clone();
                            final_channel = "upi".to_string();
                            ip_anomaly = true;
                        } else if c_type == "sequential_ato" {
                            let escalation = config.tuning.campaigns.ato_escalation_rate;
                            final_amount *= 1.0 + (escalation * i as f64);
                        }
                    }

                    local_meta.push(FraudMetadata {
                        transaction_id: tx_id.clone(),
                        fraud_target,
                        fraud_type: f_type.clone(),
                        label_noise: label_noise.clone(),
                        injector_version: "v2_one_pass_tuned".to_string(),
                        geo_anomaly,
                        device_anomaly,
                        ip_anomaly,
                        burst_session: camp_type == Some("sequential_ato".to_string()),
                        burst_seq: Some(i as i32 + 1),
                        campaign_id: camp_id.clone(),
                        campaign_type: camp_type.clone(),
                        campaign_phase: Some("active".to_string()),
                        campaign_day_number: Some((i / 5) as i32 + 1),
                    });
                } else if is_fraud_label {
                    // FP metadata
                    local_meta.push(FraudMetadata {
                        transaction_id: tx_id.clone(),
                        fraud_target: false,
                        fraud_type: "none".to_string(),
                        label_noise: "fp".to_string(),
                        injector_version: "v2_one_pass_tuned".to_string(),
                        geo_anomaly: false,
                        device_anomaly: false,
                        ip_anomaly: false,
                        burst_session: false,
                        burst_seq: None,
                        campaign_id: None,
                        campaign_type: None,
                        campaign_phase: None,
                        campaign_day_number: None,
                    });
                }

                local_txs.push(Transaction {
                    transaction_id: tx_id,
                    card_id: card.card_id.clone(),
                    account_id: card.account_id.clone(),
                    customer_id: card.customer_id.clone(),
                    merchant_id: osm_ids[idx].to_string(),
                    merchant_name: names[idx].clone(),
                    merchant_category: categories[idx].clone(),
                    merchant_country: final_country,
                    amount: final_amount,
                    currency: config.rules.global.base_currency.clone(),
                    timestamp: tx_date.to_rfc3339(),
                    transaction_channel: final_channel,
                    card_present: false,
                    user_agent: final_ua,
                    ip_address: final_ip,
                    status: final_status,
                    auth_status,
                    failure_reason,
                    is_fraud: is_fraud_label,
                    chargeback: false,
                    chargeback_days: None,
                    location_lat: final_lat,
                    location_long: final_lon,
                    h3_r7: h3_indices[idx].clone(),
                });
            }
            
            (local_txs, local_meta)
        })
        .collect();

    let mut all_transactions = Vec::new();
    let mut all_metadata = Vec::new();
    for (mut txs, mut meta) in results {
        all_transactions.append(&mut txs);
        all_metadata.append(&mut meta);
    }

    (all_transactions, all_metadata)
}
