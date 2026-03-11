use crate::config::AppConfig;
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
                let amount_noise = card_rng.random_range(0.4..1.6);
                let amount = (avg_txn_amount * amount_noise).clamp(1.0, 500_000.0);

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

                // --- NEW: Burst-Aware Timestamp Selection ---
                let mut tx_date = timestamps[i];

                let mut final_amount = amount;
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

                let mut final_ua = device_map
                    .get(&final_channel)
                    .cloned()
                    .unwrap_or_else(|| "Mozilla/5.0".to_string());

                // Spatial Jitter: Reduced jitter (~100m) for transactions to keep them clustered but not identical
                let jitter_lat = card_rng.random_range(-0.001..0.001);
                let jitter_lon = card_rng.random_range(-0.001..0.001);
                let mut final_lat = lats[idx] + jitter_lat;
                let mut final_lon = lons[idx] + jitter_lon;

                let mut final_ip = format!(
                    "103.21.{}.{}",
                    card_rng.random_range(1..255),
                    card_rng.random_range(1..255)
                );
                let mut geo_anomaly = false;
                let mut device_anomaly = false;
                let mut ip_anomaly = false;
                let mut final_status = "Success".to_string();
                let mut auth_status = "approved".to_string();
                let mut failure_reason = None;
                let mut final_card_present = card_rng.random_bool(config.transactions.transactions.card_present_probability);

                let cat_name = &categories[idx];
                let mcc = mcc_map.get(cat_name).unwrap_or(&"5999".to_string()).clone();

                if fraud_target {
                    let profiles: Vec<&String> =
                        config.rules.fraud_injector.profiles.keys().collect();
                    let f_type = profiles[card_rng.random_range(0..profiles.len())];
                    let profile = &config.rules.fraud_injector.profiles[f_type];

                    // --- NEW: Profile-Aware Temporal Warping ---
                    if let Some(last_date) = last_processed_date {
                        if f_type == "account_takeover" || f_type == "velocity_abuse" {
                            // 10 to 60 seconds apart
                            tx_date = last_date + Duration::seconds(card_rng.random_range(10..60));
                        }
                    }

                    // --- NEW: Customer-Aware Amount Strategy ---
                    if let Some(strategy) = &profile.amount_strategy {
                        if strategy == "customer_normal_range" {
                            let multiplier_str = profile.amount_multiplier.clone().unwrap_or_else(|| "0.8_to_1.2".to_string());
                            let parts: Vec<&str> = multiplier_str.split("_to_").collect();
                            if parts.len() == 2 {
                                let min_m: f64 = parts[0].parse().unwrap_or(0.8);
                                let max_m: f64 = parts[1].parse().unwrap_or(1.2);
                                let m = card_rng.random_range(min_m..max_m);
                                final_amount = (avg_txn_amount * m).clamp(1.0, 500_000.0);
                            }
                        }
                    } else if let Some(amounts) = config.rules.fraud_patterns.get(&profile.amount_pattern) {
                        final_amount = amounts[card_rng.random_range(0..amounts.len())];
                    }

                    // CNP Fraud ALWAYS has card_present = false
                    if f_type == "card_not_present" {
                        final_card_present = false;
                    }

                    if card_rng.random_bool(profile.geo_anomaly_prob) {
                        geo_anomaly = true;
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
                            failure_reason =
                                Some(reasons[card_rng.random_range(0..reasons.len())].clone());
                        } else {
                            failure_reason =
                                Some(config.tuning.defaults.fallback_failure_reason.clone());
                        }
                    }

                    if let (Some(_c_id), Some(c_type)) = (&camp_id, &camp_type) {
                        if c_type == "coordinated_attack" {
                            final_ip = config.tuning.campaigns.coordinated_scam_ip.clone();
                            final_ua = config.rules.device_patterns.bot_user_agent_prefix.clone();
                            ip_anomaly = true;
                            if let (Some(alat), Some(alon)) = (attacker_lat, attacker_lon) {
                                final_lat = alat;
                                final_lon = alon;
                                geo_anomaly = true;
                            }
                        } else if c_type == "sequential_takeover" {
                            let escalation = config.tuning.campaigns.ato_escalation_rate;
                            final_amount *= 1.0 + (escalation * i as f64);
                            if let (Some(alat), Some(alon)) = (attacker_lat, attacker_lon) {
                                final_lat = alat;
                                final_lon = alon;
                                geo_anomaly = true;
                            }
                        }
                    }

                    local_meta.push(FraudMetadata {
                        transaction_id: tx_id.clone(),
                        fraud_target,
                        fraud_type: f_type.clone(),
                        label_noise: label_noise.clone(),
                        injector_version: "v2_chunked".to_string(),
                        geo_anomaly,
                        device_anomaly,
                        ip_anomaly,
                        burst_session: camp_type == Some("sequential_takeover".to_string()),
                        burst_seq: Some(i as i32 + i32::try_from(local_txs.len()).unwrap_or(0)),
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
                        injector_version: "v2_chunked".to_string(),
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

                let merchant = crate::models::transaction::MerchantInfo {
                    id: osm_ids[idx].to_string(),
                    name: names[idx].clone(),
                    category: cat_name.clone(),
                    mcc,
                    lat: final_lat,
                    long: final_lon,
                    h3_r7: h3_indices[idx].clone(),
                };

                local_txs.push(Transaction::new(
                    tx_id,
                    card.card_id.clone(),
                    card.account_id.clone(),
                    card.customer_id.clone(),
                    merchant,
                    final_amount,
                    tx_date,
                    final_channel,
                    final_ua,
                    final_ip,
                    (final_status, auth_status, failure_reason),
                    is_fraud_label,
                    final_card_present,
                    config,
                ));
                last_processed_date = Some(tx_date);
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
