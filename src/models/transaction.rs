use chrono::{Duration, Utc};
use fake::Fake;
use fake::faker::company::en::CompanyName;
use h3o::{LatLng, Resolution};
use rand::Rng;
use serde::{Deserialize, Serialize};
use crate::config::AppConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub transaction_id: String,
    pub card_id: String,
    pub account_id: String,
    pub customer_id: String,

    pub merchant_id: String,
    pub merchant_name: String,
    pub merchant_category: String,
    pub merchant_country: String,

    pub amount: f64,
    pub currency: String,
    pub timestamp: String,
    pub transaction_channel: String,
    pub card_present: bool,
    
    pub user_agent: String,
    pub ip_address: String,
    
    pub status: String,
    pub auth_status: String,
    pub failure_reason: Option<String>,
    pub is_fraud: bool,
    
    pub chargeback: bool,
    pub chargeback_days: Option<i32>,

    pub location_lat: f64,
    pub location_long: f64,
    pub h3_r7: String,
}

impl Transaction {
    pub fn new_from_merchant(
        card_id: String,
        account_id: String,
        customer_id: String,
        merchant_id: String,
        merchant_name: String,
        merchant_category: String,
        lat: f64,
        long: f64,
        h3_r7: String,
        config: &AppConfig,
    ) -> Self {
        let mut rng = rand::rng();
        
        let transaction_id = uuid::Uuid::new_v4().to_string();
        let amount = rng.random_range(10.0..50000.0);
        let currency = config.rules.global.base_currency.clone();

        let end_date = Utc::now();
        let start_date = end_date - Duration::days(365);
        let random_seconds = rng.random_range(0..(end_date - start_date).num_seconds());
        let transaction_date = start_date + Duration::seconds(random_seconds);
        
        let (status, auth_status, failure_reason) = if rng.random_bool(1.0 - config.tuning.probabilities.failure) { 
            ("Success".to_string(), "approved".to_string(), None)
        } else { 
            ("Failed".to_string(), "declined".to_string(), Some(config.tuning.defaults.fallback_failure_reason.clone())) 
        };
        let is_fraud = false;

        // Determine channel based on market_share in config
        let mut channel = "online".to_string();
        let mut user_agent = "Mozilla/5.0".to_string();
        let mut card_present = false;
        
        let total_weight: f64 = config.rules.payment_channels.values().map(|c| c.market_share).sum();
        let mut r: f64 = rng.random_range(0.0..total_weight);
        
        for (name, chan_config) in &config.rules.payment_channels {
            if r < chan_config.market_share {
                channel = name.clone();
                
                if channel.contains("card") {
                    if rng.random_bool(0.30) {
                        card_present = true;
                        user_agent = format!("POS-Terminal-{:04X}", rng.random_range(0..0xFFFF));
                    } else if !chan_config.user_agents.is_empty() {
                        let ua_idx = rng.random_range(0..chan_config.user_agents.len());
                        user_agent = chan_config.user_agents[ua_idx].clone();
                    }
                } else if !chan_config.user_agents.is_empty() {
                    let ua_idx = rng.random_range(0..chan_config.user_agents.len());
                    user_agent = chan_config.user_agents[ua_idx].clone();
                }
                break;
            }
            r -= chan_config.market_share;
        }

        // Generate IP from device_patterns
        let ip_address = if !config.rules.device_patterns.ip_prefixes.is_empty() {
            let prefix_idx = rng.random_range(0..config.rules.device_patterns.ip_prefixes.len());
            let prefix = &config.rules.device_patterns.ip_prefixes[prefix_idx];
            format!("{}{}.{}", prefix, rng.random_range(1..255), rng.random_range(1..255))
        } else {
            "127.0.0.1".to_string()
        };

        Transaction {
            transaction_id,
            card_id,
            account_id,
            customer_id,
            merchant_id,
            merchant_name,
            merchant_category,
            merchant_country: config.rules.global.default_country.clone(),
            amount,
            currency,
            timestamp: transaction_date.to_rfc3339(),
            transaction_channel: channel,
            card_present,
            user_agent,
            ip_address,
            status,
            auth_status,
            failure_reason,
            is_fraud,
            chargeback: false,
            chargeback_days: None,
            location_lat: lat,
            location_long: long,
            h3_r7,
        }
    }

    pub fn new(card_id: String, account_id: String, customer_id: String, config: &AppConfig) -> Self {
        let mut rng = rand::rng();
        
        let transaction_id = uuid::Uuid::new_v4().to_string();
        let merchant_name: String = CompanyName().fake();
        let merchant_category = vec!["GROCERY", "FOOD_AND_BEVERAGE", "GENERAL_RETAIL", "SERVICES", "ENTERTAINMENT"]
            .into_iter()
            .nth(rng.random_range(0..5))
            .unwrap()
            .to_string();
        
        let amount = rng.random_range(10.0..50000.0);
        let currency = config.rules.global.base_currency.clone();

        let end_date = Utc::now();
        let start_date = end_date - Duration::days(365);
        let random_seconds = rng.random_range(0..(end_date - start_date).num_seconds());
        let transaction_date = start_date + Duration::seconds(random_seconds);
        
        let (status, auth_status, failure_reason) = if rng.random_bool(1.0 - config.tuning.probabilities.failure) { 
            ("Success".to_string(), "approved".to_string(), None)
        } else { 
            ("Failed".to_string(), "declined".to_string(), Some(config.tuning.defaults.fallback_failure_reason.clone())) 
        };
        let is_fraud = false;
        
        let lat: f64 = rng.random_range(8.0..37.0);
        let long: f64 = rng.random_range(68.0..97.0);
        
        let coord = LatLng::new(lat, long).expect("Invalid coordinates");
        let h3_r7 = coord.to_cell(Resolution::Seven).to_string();

        let mut channel = "online".to_string();
        let mut user_agent = "Mozilla/5.0".to_string();
        let mut card_present = false;
        
        let total_weight: f64 = config.rules.payment_channels.values().map(|c| c.market_share).sum();
        let mut r: f64 = rng.random_range(0.0..total_weight);
        
        for (name, chan_config) in &config.rules.payment_channels {
            if r < chan_config.market_share {
                channel = name.clone();
                if channel.contains("card") {
                    if rng.random_bool(0.30) {
                        card_present = true;
                        user_agent = format!("POS-Terminal-{:04X}", rng.random_range(0..0xFFFF));
                    } else if !chan_config.user_agents.is_empty() {
                        let ua_idx = rng.random_range(0..chan_config.user_agents.len());
                        user_agent = chan_config.user_agents[ua_idx].clone();
                    }
                } else if !chan_config.user_agents.is_empty() {
                    let ua_idx = rng.random_range(0..chan_config.user_agents.len());
                    user_agent = chan_config.user_agents[ua_idx].clone();
                }
                break;
            }
            r -= chan_config.market_share;
        }

        let ip_address = if !config.rules.device_patterns.ip_prefixes.is_empty() {
            let prefix_idx = rng.random_range(0..config.rules.device_patterns.ip_prefixes.len());
            let prefix = &config.rules.device_patterns.ip_prefixes[prefix_idx];
            format!("{}{}.{}", prefix, rng.random_range(1..255), rng.random_range(1..255))
        } else {
            "127.0.0.1".to_string()
        };

        Transaction {
            transaction_id,
            card_id,
            account_id,
            customer_id,
            merchant_id: "PLACEHOLDER".to_string(),
            merchant_name,
            merchant_category,
            merchant_country: config.rules.global.default_country.clone(),
            amount,
            currency,
            timestamp: transaction_date.to_rfc3339(),
            transaction_channel: channel,
            card_present,
            user_agent,
            ip_address,
            status,
            auth_status,
            failure_reason,
            is_fraud,
            chargeback: false,
            chargeback_days: None,
            location_lat: lat,
            location_long: long,
            h3_r7,
        }
    }
}
