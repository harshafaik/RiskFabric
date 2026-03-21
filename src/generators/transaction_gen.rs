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

            let campaign_ctx = fraud::initialize_campaign(config, &mut card_rng, &card.card_id);

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

                let (fraud_target, is_fraud_label, label_noise) =
                    fraud::determine_targeting(config, &mut card_rng);

                // --- Step 2: Base Attribute Selection ---
                let tx_date = timestamps[i];
                let total_share: f64 = config
                    .rules
                    .payment_channels
                    .values()
                    .map(|c| c.market_share)
                    .sum();
                let mut r_chan: f64 = card_rng.random_range(0.0..total_share);
                let mut final_channel = "upi".to_string();
                for (name, c_config) in &config.rules.payment_channels {
                    if r_chan < c_config.market_share {
                        final_channel = name.clone();
                        break;
                    }
                    r_chan -= c_config.market_share;
                }

                let final_ua = {
                    let ua_roll = card_rng.random_range(0.0..1.0);
                    if ua_roll < 0.92 {
                        customer.device.primary_ua.clone()
                    } else if ua_roll < 0.98 {
                        customer.device.secondary_ua.clone().unwrap_or_else(|| customer.device.primary_ua.clone())
                    } else {
                        // 2% different device
                        let pools = [&config.customer.device_profiles.android_ua_pool, &config.customer.device_profiles.ios_ua_pool, &config.customer.device_profiles.upi_app_ua_pool];
                        let pool = pools[card_rng.random_range(0..pools.len())];
                        pool[card_rng.random_range(0..pool.len())].clone()
                    }
                };

                let final_ip = {
                    let ip_roll = card_rng.random_range(0.0..1.0);
                    if ip_roll < 0.85 {
                        // Home subnet, consistent host (simplified: fixed host per customer for now)
                        customer.device.ip_subnet.replace("x.x/16", &format!("{}.{}", card_rng.random_range(1..255), card_rng.random_range(1..255)))
                    } else if ip_roll < 0.95 {
                        // Same ISP mobile range (different subnet)
                        let isp_subnets: Vec<&String> = config.customer.isp_assignment.subnets.values().collect();
                        let random_subnet = isp_subnets[card_rng.random_range(0..isp_subnets.len())];
                        random_subnet.replace("x.x/16", &format!("{}.{}", card_rng.random_range(1..255), card_rng.random_range(1..255)))
                    } else if ip_roll < 0.99 {
                        // VPN range
                        let vpn_prefixes = ["185.", "104."];
                        format!("{}16.{}.{}", vpn_prefixes[card_rng.random_range(0..vpn_prefixes.len())], card_rng.random_range(1..255), card_rng.random_range(1..255))
                    } else {
                        // Foreign/anomalous
                        format!("{}.{}.{}.{}", card_rng.random_range(1..255), card_rng.random_range(1..255), card_rng.random_range(1..255), card_rng.random_range(1..255))
                    }
                };
                let mut final_amount =
                    (avg_txn_amount * card_rng.random_range(0.4..1.6)).clamp(1.0, 500_000.0);

                let jitter_lat = card_rng.random_range(-0.001..0.001);
                let jitter_lon = card_rng.random_range(-0.001..0.001);
                let final_lat = lats[idx] + jitter_lat;
                let final_lon = lons[idx] + jitter_lon;

                let final_card_present =
                    card_rng.random_bool(config.transactions.transactions.card_present_probability);

                // --- Step 3: Construct Temporary Transaction for Mutation ---
                let cat_name = &categories[idx];

                // Fat tail legitimate spending logic
                if !fraud_target {
                    let fat_tail_categories =
                        ["ELECTRONICS", "TRAVEL", "MEDICAL", "JEWELLERY", "EDUCATION"];
                    if fat_tail_categories.contains(&cat_name.as_str()) {
                        if card_rng.random_bool(0.04) {
                            // 4% probability (between 3-5%)
                            final_amount = (avg_txn_amount * card_rng.random_range(2.0..8.0))
                                .clamp(1.0, 500_000.0);
                        }
                    }
                }

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
                if let Some(meta) = fraud::inject_fraud(
                    &mut tx,
                    fraud_target,
                    is_fraud_label,
                    label_noise,
                    &campaign_ctx,
                    last_processed_date,
                    avg_txn_amount,
                    i,
                    config,
                    &mut card_rng,
                ) {
                    local_meta.push(meta);
                }

                last_processed_date = Some(
                    chrono::DateTime::parse_from_rfc3339(&tx.timestamp)
                        .unwrap()
                        .with_timezone(&Utc),
                );
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
