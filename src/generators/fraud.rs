use crate::config::{AppConfig, FraudProfileConfig};
use crate::models::transaction::Transaction;
use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use rand::rngs::StdRng;

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

/// Applies behavioral mutations (UA, IP, Status) based on the fraud profile
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

    // 2. Card Present Logic
    if f_type == "card_not_present" {
        tx.card_present = false;
    }

    // 3. Geo Anomaly
    if rng.random_bool(profile.geo_anomaly_prob) {
        tx.merchant_country = config.tuning.defaults.geo_anomaly_country.clone();
        tx.location_lat = rng.random_range(8.0..37.0);
        tx.location_long = rng.random_range(68.0..97.0);
        geo_anomaly = true;
    }

    // 4. Device Anomaly
    if rng.random_bool(config.tuning.probabilities.device_anomaly) {
        tx.user_agent = config.rules.device_patterns.bot_user_agent_prefix.clone();
        device_anomaly = true;
    }

    // 5. IP Anomaly
    if rng.random_bool(config.tuning.probabilities.ip_anomaly) {
        let prefixes: Vec<&String> = config
            .rules
            .device_patterns
            .known_bad_prefixes
            .keys()
            .collect();
        if !prefixes.is_empty() {
            let selected = prefixes[rng.random_range(0..prefixes.len())];
            tx.ip_address = format!("{}.{}", selected, rng.random_range(1..255));
            ip_anomaly = true;
        }
    }

    // 6. Failure Logic
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
            tx.user_agent = config.rules.device_patterns.bot_user_agent_prefix.clone();
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
