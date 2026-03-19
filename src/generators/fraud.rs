use crate::config::{AppConfig, FraudProfileConfig};
use crate::models::fraud_metadata::FraudMetadata;
use crate::models::transaction::Transaction;
use chrono::{DateTime, Duration, Utc, Timelike};
use rand::Rng;
use rand::rngs::StdRng;

/// Context for a fraud campaign associated with a customer/card
#[derive(Clone, Debug)]
pub struct FraudCampaignContext {
    pub id: String,
    pub campaign_type: String,
    pub attacker_lat: f64,
    pub attacker_lon: f64,
}

/// Orchestration: Initializes a fraud campaign for a specific card/customer
pub fn initialize_campaign(
    config: &AppConfig,
    rng: &mut StdRng,
    card_id: &str,
) -> Option<FraudCampaignContext> {
    let share = config.rules.fraud_campaigns.target_campaign_share;

    if rng.random_bool(share) {
        let campaigns: Vec<&String> = config.rules.fraud_campaigns.profiles.keys().collect();
        let c_type = campaigns[rng.random_range(0..campaigns.len())].clone();
        Some(FraudCampaignContext {
            id: format!("camp_{}_{}", c_type, &card_id[..8]),
            campaign_type: c_type,
            attacker_lat: rng.random_range(8.0..37.0),
            attacker_lon: rng.random_range(68.0..97.0),
        })
    } else {
        None
    }
}

/// Orchestration: Determines if a transaction is a fraud target and handles label noise
pub fn determine_targeting(config: &AppConfig, rng: &mut StdRng) -> (bool, bool, String) {
    let r_target: f64 = rng.random();
    let fraud_target = r_target < config.rules.fraud_injector.target_share;

    let r_fn: f64 = rng.random();
    let r_fp: f64 = rng.random();
    let mut is_fraud_label = fraud_target;
    let mut label_noise = "none".to_string();

    if fraud_target && r_fn < config.rules.fraud_injector.default_fn_rate {
        is_fraud_label = false;
        label_noise = "fn".to_string();
    } else if !fraud_target && r_fp < config.rules.fraud_injector.default_fp_rate {
        is_fraud_label = true;
        label_noise = "fp".to_string();
    }

    (fraud_target, is_fraud_label, label_noise)
}

/// Orchestration: Injects fraud mutations and returns metadata
pub fn inject_fraud(
    tx: &mut Transaction,
    fraud_target: bool,
    is_fraud_label: bool,
    label_noise: String,
    campaign_ctx: &Option<FraudCampaignContext>,
    last_processed_date: Option<DateTime<Utc>>,
    avg_txn_amount: f64,
    txn_index: usize,
    config: &AppConfig,
    rng: &mut StdRng,
) -> Option<FraudMetadata> {
    if fraud_target {
        let f_type: String;
        let mut geo_anomaly: bool;
        let mut device_anomaly: bool;
        let mut ip_anomaly: bool;

        let profiles: Vec<&String> = config.rules.fraud_injector.profiles.keys().collect();
        f_type = profiles[rng.random_range(0..profiles.len())].clone();
        let profile = &config.rules.fraud_injector.profiles[&f_type];

        // 1. Temporal Warping (Velocity/Burst)
        let tx_date = DateTime::parse_from_rfc3339(&tx.timestamp)
            .unwrap()
            .with_timezone(&Utc);
        let mut final_tx_date = calculate_fraud_timestamp(&f_type, last_processed_date, tx_date, rng);

        // 2. Temporal Anomaly (Hour Shift)
        let current_hour = final_tx_date.hour();
        let fraud_hour = calculate_fraud_hour(profile, &f_type, current_hour, config, rng);
        if fraud_hour != current_hour {
            final_tx_date = final_tx_date.with_hour(fraud_hour).unwrap_or(final_tx_date);
        }
        tx.timestamp = final_tx_date.to_rfc3339();

        // 3. Amount Mimicry
        tx.amount = calculate_fraud_amount(profile, avg_txn_amount, config, rng);

        // 4. Behavioral Mutations (UA, IP, Geo, Channel)
        let (geo, dev, ip) = apply_behavioral_mutations(&f_type, profile, tx, config, rng);
        geo_anomaly = geo;
        device_anomaly = dev;
        ip_anomaly = ip;

        // 5. Campaign Coordination
        if let Some(ctx) = campaign_ctx {
            let (c_geo, c_dev, c_ip) = apply_campaign_logic(
                &ctx.campaign_type,
                &ctx.id,
                tx,
                (ctx.attacker_lat, ctx.attacker_lon),
                config,
            );
            if c_geo {
                geo_anomaly = true;
            }
            if c_dev {
                device_anomaly = true;
            }
            if c_ip {
                ip_anomaly = true;
            }
        }

        // 6. Select Random Behavioral Flags
        let mut behavioral_flags = Vec::new();
        if let Some(flags) = config.rules.fraud_flags.get(&f_type) {
            if !flags.is_empty() {
                // Sample 1-2 flags
                let num_flags = if flags.len() > 1 { rng.random_range(1..=2) } else { 1 };
                for _ in 0..num_flags {
                    let f = &flags[rng.random_range(0..flags.len())];
                    if !behavioral_flags.contains(f) {
                        behavioral_flags.push(f.clone());
                    }
                }
            }
        }

        if !config.transactions.transactions.streaming_mode {
            return Some(FraudMetadata {
                transaction_id: tx.transaction_id.clone(),
                fraud_target,
                fraud_type: f_type.clone(),
                label_noise: label_noise.clone(),
                injector_version: "v3_modular".to_string(),
                geo_anomaly,
                device_anomaly,
                ip_anomaly,
                flags: if behavioral_flags.is_empty() { None } else { Some(behavioral_flags) },
                burst_session: campaign_ctx
                    .as_ref()
                    .map_or(false, |c| c.campaign_type == "sequential_takeover"),
                burst_seq: Some(txn_index as i32 + 1),
                campaign_id: campaign_ctx.as_ref().map(|c| c.id.clone()),
                campaign_type: campaign_ctx.as_ref().map(|c| c.campaign_type.clone()),
                campaign_phase: Some("active".to_string()),
                campaign_day_number: Some((txn_index / 5) as i32 + 1),
            });
        }
    } else if is_fraud_label && !config.transactions.transactions.streaming_mode {
        return Some(FraudMetadata {
            transaction_id: tx.transaction_id.clone(),
            fraud_target: false,
            fraud_type: "none".to_string(),
            label_noise: "fp".to_string(),
            injector_version: "v3_modular".to_string(),
            geo_anomaly: false,
            device_anomaly: false,
            ip_anomaly: false,
            flags: None,
            burst_session: false,
            burst_seq: None,
            campaign_id: None,
            campaign_type: None,
            campaign_phase: None,
            campaign_day_number: None,
        });
    }

    None
}

/// Pure logic for calculating fraud amounts based on profile strategy
pub fn calculate_fraud_amount(
    profile: &FraudProfileConfig,
    avg_txn_amount: f64,
    config: &AppConfig,
    rng: &mut StdRng,
) -> f64 {
    if let Some(strategy) = &profile.amount_strategy {
        if strategy == "customer_normal_range" {
            let multiplier_str = profile
                .amount_multiplier
                .clone()
                .unwrap_or_else(|| "0.8_to_1.2".to_string());
            let parts: Vec<&str> = multiplier_str.split("_to_").collect();
            if parts.len() == 2 {
                let min_m: f64 = parts[0].parse().unwrap_or(0.8);
                let max_m: f64 = parts[1].parse().unwrap_or(1.2);
                let m = rng.random_range(min_m..max_m);
                return (avg_txn_amount * m).clamp(1.0, 500_000.0);
            }
        }
    }

    // Fallback to pattern-based amounts
    if let Some(amounts) = config.rules.fraud_patterns.get(&profile.amount_pattern) {
        return amounts[rng.random_range(0..amounts.len())];
    }

    avg_txn_amount // Ultimate fallback
}

/// Determines if a transaction should be "warped" into a high-velocity burst
pub fn calculate_fraud_timestamp(
    f_type: &str,
    last_date: Option<DateTime<Utc>>,
    original_date: DateTime<Utc>,
    rng: &mut StdRng,
) -> DateTime<Utc> {
    if let Some(last) = last_date {
        if f_type == "account_takeover" || f_type == "velocity_abuse" {
            // High-velocity burst: 10 to 60 seconds apart
            return last + Duration::seconds(rng.random_range(10..60));
        }
    }
    original_date
}

/// Determines the hour for a fraudulent transaction, potentially deviating from customer norm
pub fn calculate_fraud_hour(
    profile: &FraudProfileConfig,
    f_type: &str,
    original_hour: u32,
    config: &AppConfig,
    rng: &mut StdRng,
) -> u32 {
    let dev_prob = profile.temporal_anomaly_prob.unwrap_or(0.0);
    
    if rng.random_bool(dev_prob) {
        // Find the profile's temporal pattern or fallback to global
        if let Some(pattern) = config.rules.temporal_patterns.get(f_type) {
            let total_weight: f64 = pattern.hourly_weights.iter().sum();
            let mut r: f64 = rng.random_range(0.0..total_weight);
            for (hour, weight) in pattern.hourly_weights.iter().enumerate() {
                if r < *weight {
                    return hour as u32;
                }
                r -= weight;
            }
        }
    }
    
    original_hour
}

/// Applies behavioral mutations (UA, IP, Status, Merchant Category Bias)
pub fn apply_behavioral_mutations(
    f_type: &str,
    profile: &FraudProfileConfig,
    tx: &mut Transaction,
    config: &AppConfig,
    rng: &mut StdRng,
) -> (bool, bool, bool) {
    let mut geo_anomaly = false;
    let mut device_anomaly = false;
    let mut ip_anomaly = false;

    // 1. Channel Bias
    if !profile.channel_bias.is_empty() {
        let mut r_chan: f64 = rng.random();
        for (chan, weight) in &profile.channel_bias {
            if r_chan < *weight {
                tx.transaction_channel = chan.clone();
                break;
            }
            r_chan -= weight;
        }
    }

    // 2. Merchant Category Bias (Contextual Anomaly)
    if let Some(bias) = &profile.merchant_bias {
        let mut r_cat: f64 = rng.random();
        for (cat, weight) in bias {
            if r_cat < *weight {
                tx.merchant_category = cat.clone();
                break;
            }
            r_cat -= weight;
        }
    }

    // 3. Card Present Logic
    if f_type == "card_not_present" {
        tx.card_present = false;
    }

    // 4. Geo Anomaly
    if rng.random_bool(profile.geo_anomaly_prob) {
        tx.merchant_country = config.tuning.defaults.geo_anomaly_country.clone();
        tx.location_lat = rng.random_range(8.0..37.0);
        tx.location_long = rng.random_range(68.0..97.0);
        geo_anomaly = true;
    }

    // 5. Device and IP Anomaly (Fraud-type specific)
    match f_type {
        "account_takeover" => {
            // Foreign datacenter range (AWS/GCP)
            let dc_prefixes = ["52.", "34."];
            tx.ip_address = format!("{}{}.{}.{}", dc_prefixes[rng.random_range(0..dc_prefixes.len())], rng.random_range(1..255), rng.random_range(1..255), rng.random_range(1..255));
            
            // Attacker device pool: Select from a fixed pool to simulate reuse (degree > 1)
            if !config.rules.device_patterns.ato_ua_pool.is_empty() {
                let pool = &config.rules.device_patterns.ato_ua_pool;
                tx.user_agent = pool[rng.random_range(0..pool.len())].clone();
            } else {
                // Fallback to random if pool is missing
                let pools = [&config.customer.device_profiles.android_ua_pool, &config.customer.device_profiles.ios_ua_pool];
                let pool = pools[rng.random_range(0..pools.len())];
                tx.user_agent = pool[rng.random_range(0..pool.len())].clone();
            }
            
            device_anomaly = true;
            ip_anomaly = true;
        },
        "card_not_present" => {
            // VPN range
            let vpn_prefixes = ["185.", "104.16."];
            tx.ip_address = format!("{}{}.{}", vpn_prefixes[rng.random_range(0..vpn_prefixes.len())], rng.random_range(1..255), rng.random_range(1..255));
            
            // Headless browser
            let headless_uas = ["HeadlessChrome/114.0.5735.0", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 HeadlessChrome"];
            tx.user_agent = headless_uas[rng.random_range(0..headless_uas.len())].to_string();
            
            device_anomaly = true;
            ip_anomaly = true;
        },
        "velocity_abuse" => {
            // Rotate through datacenter IPs (same subnet)
            tx.ip_address = format!("45.60.{}.{}", rng.random_range(1..255), rng.random_range(1..255));
            
            // Android SDK emulator
            tx.user_agent = "Mozilla/5.0 (Linux; Android 9; Android SDK built for x86) AppleWebKit/537.36 Chrome/69.0.0.0".to_string();
            
            device_anomaly = true;
            ip_anomaly = true;
        },
        "upi_scam" | "friendly_fraud" => {
            // No IP/UA mutation - victim's own device/initiator
            // (Keep original tx.ip_address and tx.user_agent assigned in transaction_gen.rs)
        },
        _ => {
            // Default legacy bot behavior if needed
            if rng.random_bool(config.tuning.probabilities.device_anomaly) {
                tx.user_agent = format!("{} v{}.{}", 
                    config.rules.device_patterns.bot_user_agent_prefix,
                    rng.random_range(1..5),
                    rng.random_range(1..20)
                );
                device_anomaly = true;
            }
            if rng.random_bool(config.tuning.probabilities.ip_anomaly) {
                let prefixes: Vec<&String> = config.rules.device_patterns.known_bad_prefixes.keys().collect();
                if !prefixes.is_empty() {
                    let selected = prefixes[rng.random_range(0..prefixes.len())];
                    tx.ip_address = format!("{}.{}", selected, rng.random_range(1..255));
                    ip_anomaly = true;
                }
            }
        }
    }

    // 7. Failure Logic
    if rng.random_bool(config.tuning.probabilities.failure) {
        tx.status = "Failed".to_string();
        tx.auth_status = "declined".to_string();
        if let Some(reasons) = config.rules.failure_reasons_by_type.get(f_type) {
            tx.failure_reason = Some(reasons[rng.random_range(0..reasons.len())].clone());
        }
    }

    (geo_anomaly, device_anomaly, ip_anomaly)
}

/// Handles coordinated campaign overrides (Shared IP, persistent locations)
pub fn apply_campaign_logic(
    c_type: &str,
    _c_id: &str,
    tx: &mut Transaction,
    attacker_geo: (f64, f64),
    config: &AppConfig,
) -> (bool, bool, bool) {
    let mut geo = false;
    let mut dev = false;
    let mut ip = false;

    match c_type {
        "coordinated_attack" => {
            tx.ip_address = config.tuning.campaigns.coordinated_scam_ip.clone();
            // Match the randomized versioning used in apply_behavioral_mutations
            tx.user_agent = format!("{} v1.0.0", config.rules.device_patterns.bot_user_agent_prefix);
            tx.location_lat = attacker_geo.0;
            tx.location_long = attacker_geo.1;
            geo = true;
            dev = true;
            ip = true;
        }
        "sequential_takeover" => {
            // Location "sticks" to attacker
            tx.location_lat = attacker_geo.0;
            tx.location_long = attacker_geo.1;
            geo = true;
        }
        _ => {}
    }

    (geo, dev, ip)
}
