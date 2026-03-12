use crate::config::AppConfig;
use crate::generators::fraud;
use crate::models::card::Card;
use crate::models::customer::Customer;
use crate::models::fraud_metadata::FraudMetadata;
use crate::models::transaction::Transaction;
use chrono::{Datelike, Duration, Utc};
use h3o::{CellIndex, Resolution};
use rand::{Rng, SeedableRng, rngs::StdRng};
use rayon::prelude::*;
use std::collections::HashMap;
use std::str::FromStr;

type MerchantTuple = (
    Vec<String>,
    Vec<String>,
    Vec<f64>,
    Vec<f64>,
    Vec<String>,
    Vec<i64>,
    Vec<String>,
);

pub fn generate_transactions_chunk(
    cards: &[Card],
    customer_map: &HashMap<String, &Customer>,
    spatial_indices: &(
        HashMap<String, Vec<usize>>,
        HashMap<String, Vec<usize>>,
        HashMap<String, Vec<usize>>,
    ),
    merchants: &MerchantTuple,
    config: &AppConfig,
) -> (Vec<Transaction>, Vec<FraudMetadata>) {
    let (h3_indices, names, lats, lons, categories, osm_ids, _states) = merchants;
    let (index_res6, index_res4, index_state) = spatial_indices;
    let ref_count = h3_indices.len();

    let mut mcc_map: HashMap<String, String> = HashMap::new();
    for entry in &config.transactions.transactions.merchant_categories {
        mcc_map.insert(entry.name.clone(), entry.mcc.clone());
    }

    let hourly_weights = &config
        .transactions
        .transactions
        .temporal_patterns
        .hourly_weights;
    let daily_weights = &config
        .transactions
        .transactions
        .temporal_patterns
        .daily_weights;
    let total_hourly_weight: f64 = hourly_weights.iter().sum();
    let total_daily_weight: f64 = daily_weights.iter().sum();

    let base_end_date = Utc::now();
    let base_start_date = base_end_date - Duration::days(365);

    let results: Vec<(Vec<Transaction>, Vec<FraudMetadata>)> = cards
        .par_iter()
        .map(|card| {
            let mut local_txs = Vec::new();
            let mut local_meta = Vec::new();

            let mut card_rng = StdRng::seed_from_u64(
                config.rules.global.seed as u64
                    + config.tuning.salts.injector as u64
                    + card
                        .card_id
                        .as_bytes()
                        .iter()
                        .map(|&b| b as u64)
                        .sum::<u64>(),
            );

            let customer = customer_map
                .get(&card.customer_id)
                .expect("Customer missing");
            let cust_cell = CellIndex::from_str(&customer.location.home_h3r7).expect("H3 invalid");

            // Pre-calculate spatial keys for this customer
            let p6_key = cust_cell.parent(Resolution::Six).unwrap().to_string();
            let p4_key = cust_cell.parent(Resolution::Four).unwrap().to_string();
            let state_key = &customer.location.state;

            let annual_budget = customer.financial.monthly_spend * 12.0;

            let mut device_map: HashMap<String, String> = HashMap::new();
            for (channel, chan_config) in &config.rules.payment_channels {
                if !chan_config.user_agents.is_empty() {
                    let idx = card_rng.random_range(0..chan_config.user_agents.len());
                    device_map.insert(channel.clone(), chan_config.user_agents[idx].clone());
                }
            }

            let mut camp_id = None;
            let mut camp_type = None;
            let mut attacker_lat = None;
            let mut attacker_lon = None;

            let share = config
                .rules
                .fraud_campaigns
                .values()
                .filter_map(|c| c.target_campaign_share)
                .next()
                .unwrap_or(0.15);
            if card_rng.random_bool(share) {
                let campaigns: Vec<&String> = config.rules.fraud_campaigns.keys().collect();
                let c_type = campaigns[card_rng.random_range(0..campaigns.len())].clone();
                camp_id = Some(format!("camp_{}_{}", c_type, &card.card_id[..8]));
                camp_type = Some(c_type);
                attacker_lat = Some(card_rng.random_range(8.0..37.0));
                attacker_lon = Some(card_rng.random_range(68.0..97.0));
            }

            let num_txns = card_rng.random_range(
                config.customer.control.transactions_per_customer.min
                    ..config.customer.control.transactions_per_customer.max,
            );
            let avg_txn_amount = annual_budget / num_txns as f64;

            // --- Step 1: Pre-generate and Sort Timestamps ---
            let mut timestamps = Vec::with_capacity(num_txns);
            for _ in 0..num_txns {
                let mut r_hour: f64 = card_rng.random_range(0.0..total_hourly_weight);
                let mut selected_hour = 0;
                for (h, weight) in hourly_weights.iter().enumerate() {
                    if r_hour < *weight {
                        selected_hour = h;
                        break;
                    }
                    r_hour -= weight;
                }

                let mut random_day_offset = card_rng.random_range(0..365);
                loop {
                    let date = base_start_date + Duration::days(random_day_offset);
                    let weekday = date.weekday().num_days_from_monday() as usize;
                    let day_weight = daily_weights[weekday];
                    if card_rng.random_range(0.0..total_daily_weight / 7.0 * 1.5) < day_weight {
                        break;
                    }
                    random_day_offset = card_rng.random_range(0..365);
                }

                let dt = base_start_date
                    + Duration::days(random_day_offset)
                    + Duration::hours(selected_hour as i64)
                    + Duration::minutes(card_rng.random_range(0..60))
                    + Duration::seconds(card_rng.random_range(0..60));
                timestamps.push(dt);
            }
            timestamps.sort();

            let mut last_processed_date = None;

            for i in 0..num_txns {
                // --- Refined Spatial Selection ---
                // 80% Super Local (Res 6), 15% City/District (Res 4), 3% State, 2% Global
                let r_spatial: f64 = card_rng.random();
                let idx = if r_spatial < 0.80 {
                    index_res6
                        .get(&p6_key)
                        .map(|v| v[card_rng.random_range(0..v.len())])
                        .unwrap_or_else(|| card_rng.random_range(0..ref_count))
                } else if r_spatial < 0.95 {
                    index_res4
                        .get(&p4_key)
                        .map(|v| v[card_rng.random_range(0..v.len())])
                        .unwrap_or_else(|| card_rng.random_range(0..ref_count))
                } else if r_spatial < 0.98 {
                    index_state
                        .get(state_key)
                        .map(|v| v[card_rng.random_range(0..v.len())])
                        .unwrap_or_else(|| card_rng.random_range(0..ref_count))
                } else {
                    card_rng.random_range(0..ref_count)
                };

                let tx_id = format!(
                    "tx_{}_{}_{}",
                    &card.card_id[..8],
                    i,
                    card_rng.random_range(1000..9999)
                );

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

                // --- Step 2: Base Attribute Selection ---
                let tx_date = timestamps[i];
                let total_share: f64 = config.rules.payment_channels.values().map(|c| c.market_share).sum();
                let mut r_chan: f64 = card_rng.random_range(0.0..total_share);
                let mut final_channel = "upi".to_string();
                for (name, c_config) in &config.rules.payment_channels {
                    if r_chan < c_config.market_share {
                        final_channel = name.clone();
                        break;
                    }
                    r_chan -= c_config.market_share;
                }

                let final_ua = device_map.get(&final_channel).cloned().unwrap_or_else(|| "Mozilla/5.0".to_string());
                let mut final_ip = format!("103.21.{}.{}", card_rng.random_range(1..255), card_rng.random_range(1..255));
                let mut final_amount = (avg_txn_amount * card_rng.random_range(0.4..1.6)).clamp(1.0, 500_000.0);
                
                let jitter_lat = card_rng.random_range(-0.001..0.001);
                let jitter_lon = card_rng.random_range(-0.001..0.001);
                let final_lat = lats[idx] + jitter_lat;
                let final_lon = lons[idx] + jitter_lon;
                
                let final_card_present = card_rng.random_bool(config.transactions.transactions.card_present_probability);

                let mut geo_anomaly = false;
                let mut device_anomaly = false;
                let mut ip_anomaly = false;
                let mut f_type = "none".to_string();

                // --- Step 3: Construct Temporary Transaction for Mutation ---
                let cat_name = &categories[idx];
                let mcc = mcc_map.get(cat_name).unwrap_or(&"5999".to_string()).clone();
                let merchant = crate::models::transaction::MerchantInfo {
                    id: osm_ids[idx].to_string(),
                    name: names[idx].clone(),
                    category: cat_name.clone(),
                    mcc,
                    lat: final_lat,
                    long: final_lon,
                    h3_r7: h3_indices[idx].clone(),
                };

                let mut tx = Transaction::new(
                    tx_id.clone(),
                    card.card_id.clone(),
                    card.account_id.clone(),
                    card.customer_id.clone(),
                    merchant,
                    final_amount,
                    tx_date,
                    final_channel,
                    final_ua,
                    final_ip,
                    ("Success".to_string(), "approved".to_string(), None),
                    is_fraud_label,
                    final_card_present,
                    config,
                );

                // --- Step 5: Apply Sneaky Fraud Logic ---
                if fraud_target {
                    let profiles: Vec<&String> = config.rules.fraud_injector.profiles.keys().collect();
                    f_type = profiles[card_rng.random_range(0..profiles.len())].clone();
                    let profile = &config.rules.fraud_injector.profiles[&f_type];

                    // 5a. Temporal Warping
                    tx.timestamp = fraud::calculate_fraud_timestamp(&f_type, last_processed_date, tx_date, &mut card_rng).to_rfc3339();
                    
                    // 5b. Amount Mimicry
                    tx.amount = fraud::calculate_fraud_amount(profile, avg_txn_amount, config, &mut card_rng);

                    // 5c. Behavioral Mutations (UA, IP, Geo, Channel)
                    let (geo, dev, ip) = fraud::apply_behavioral_mutations(&f_type, profile, &mut tx, config, &mut card_rng);
                    geo_anomaly = geo;
                    device_anomaly = dev;
                    ip_anomaly = ip;

                    // 5d. Campaign Coordination
                    if let (Some(c_id), Some(c_type)) = (&camp_id, &camp_type) {
                        let (c_geo, c_dev, c_ip) = fraud::apply_campaign_logic(c_type, c_id, &mut tx, (attacker_lat.unwrap_or(0.0), attacker_lon.unwrap_or(0.0)), config);
                        if c_geo { geo_anomaly = true; }
                        if c_dev { device_anomaly = true; }
                        if c_ip { ip_anomaly = true; }
                    }

                    local_meta.push(FraudMetadata {
                        transaction_id: tx_id.clone(),
                        fraud_target,
                        fraud_type: f_type.clone(),
                        label_noise: label_noise.clone(),
                        injector_version: "v3_modular".to_string(),
                        geo_anomaly,
                        device_anomaly,
                        ip_anomaly,
                        burst_session: camp_type == Some("sequential_takeover".to_string()),
                        burst_seq: Some(i as i32 + 1),
                        campaign_id: camp_id.clone(),
                        campaign_type: camp_type.clone(),
                        campaign_phase: Some("active".to_string()),
                        campaign_day_number: Some((i / 5) as i32 + 1),
                    });
                } else if is_fraud_label {
                    local_meta.push(FraudMetadata {
                        transaction_id: tx_id.clone(),
                        fraud_target: false,
                        fraud_type: "none".to_string(),
                        label_noise: "fp".to_string(),
                        injector_version: "v3_modular".to_string(),
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

                last_processed_date = Some(chrono::DateTime::parse_from_rfc3339(&tx.timestamp).unwrap().with_timezone(&Utc));
                local_txs.push(tx);
            }

            (local_txs, local_meta)
        })
        .collect();

    let mut all_txs = Vec::new();
    let mut all_meta = Vec::new();
    for (mut txs, mut meta) in results {
        all_txs.append(&mut txs);
        all_meta.append(&mut meta);
    }
    (all_txs, all_meta)
}
