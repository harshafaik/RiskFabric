use crate::models::transaction::Transaction;
use crate::models::fraud_metadata::FraudMetadata;
use crate::config::AppConfig;
use std::collections::HashMap;
use rayon::prelude::*;
use rand::{Rng, SeedableRng, rngs::StdRng};

pub struct FraudInjector<'a> {
    pub config: &'a AppConfig,
}

impl<'a> FraudInjector<'a> {
    pub fn new(config: &'a AppConfig) -> Self {
        Self { config }
    }

    fn seeded_rng(&self, id: &str, salt: u64) -> StdRng {
        let mut seed = [0u8; 32];
        let id_bytes = id.as_bytes();
        for i in 0..32.min(id_bytes.len()) {
            seed[i] = id_bytes[i];
        }
        let s = self.config.rules.global.seed as u64 + salt;
        seed[24..32].copy_from_slice(&s.to_le_bytes());
        StdRng::from_seed(seed)
    }

    pub fn pick_fraud_type<R: Rng>(&self, rng: &mut R) -> String {
        let profiles = &self.config.rules.fraud_injector.profiles;
        if profiles.is_empty() {
            return "UNKNOWN".to_string();
        }

        let r: f64 = rng.random();
        let total_weight: f64 = profiles.values().map(|p| p.frequency).sum();

        let mut cumulative = 0.0;
        for (fraud_type, profile) in profiles {
            let normalized_weight = profile.frequency / total_weight;
            cumulative += normalized_weight;
            if r <= cumulative {
                return fraud_type.clone();
            }
        }

        profiles.keys().next().unwrap().clone()
    }

    pub fn inject(&self, transactions: &mut Vec<Transaction>) -> HashMap<String, FraudMetadata> {
        let target_share = self.config.rules.fraud_injector.target_share;
        let default_fn = self.config.rules.fraud_injector.default_fn_rate;
        let default_fp = self.config.rules.fraud_injector.default_fp_rate;
        let injector_salt = self.config.tuning.salts.injector as u64;

        transactions.par_iter_mut().filter_map(|tx| {
            let mut rng = self.seeded_rng(&tx.transaction_id, injector_salt);
            
            let r_target: f64 = rng.random();
            let fraud_target = r_target < target_share;

            let r_fn: f64 = rng.random();
            let r_fp: f64 = rng.random();

            let mut is_fraud_label = fraud_target;
            let mut label_noise = "none".to_string();

            if fraud_target && r_fn < default_fn {
                is_fraud_label = false;
                label_noise = "fn".to_string();
            } else if !fraud_target && r_fp < default_fp {
                is_fraud_label = true;
                label_noise = "fp".to_string();
            }

            tx.is_fraud = is_fraud_label;

            if fraud_target || is_fraud_label {
                let mut fraud_type = "none".to_string();
                if fraud_target {
                    fraud_type = self.pick_fraud_type(&mut rng);
                }

                Some((tx.transaction_id.clone(), FraudMetadata {
                    transaction_id: tx.transaction_id.clone(),
                    fraud_target,
                    fraud_type,
                    label_noise,
                    injector_version: "v1_rust_tuned".to_string(),
                    geo_anomaly: false,
                    device_anomaly: false,
                    ip_anomaly: false,
                    burst_session: false,
                    burst_seq: None,
                    campaign_id: None,
                    campaign_type: None,
                    campaign_phase: None,
                    campaign_day_number: None,
                }))
            } else {
                None
            }
        }).collect()
    }
}

pub struct FraudMutator<'a> {
    pub config: &'a AppConfig,
}

impl<'a> FraudMutator<'a> {
    pub fn new(config: &'a AppConfig) -> Self {
        Self { config }
    }

    fn seeded_rng(&self, id: &str, salt: u64) -> StdRng {
        let mut seed = [0u8; 32];
        let id_bytes = id.as_bytes();
        for i in 0..32.min(id_bytes.len()) {
            seed[i] = id_bytes[i];
        }
        let s = self.config.rules.global.seed as u64 + salt;
        seed[24..32].copy_from_slice(&s.to_le_bytes());
        StdRng::from_seed(seed)
    }

    pub fn apply(&self, transactions: &mut Vec<Transaction>, metadata_map: &mut HashMap<String, FraudMetadata>) {
        let mutator_salt = self.config.tuning.salts.mutator as u64;
        let prob = &self.config.tuning.probabilities;
        let defaults = &self.config.tuning.defaults;

        let tx_updates: Vec<(String, bool, bool, bool, Option<String>, Option<i32>)> = transactions.par_iter_mut().filter_map(|tx| {
            if let Some(meta) = metadata_map.get(&tx.transaction_id) {
                if !meta.fraud_target {
                    return None;
                }

                let mut rng = self.seeded_rng(&tx.transaction_id, mutator_salt);
                
                let mut geo_anomaly = false;
                let mut device_anomaly = false;
                let mut ip_anomaly = false;
                let mut failure_reason = None;
                let mut chargeback_days = None;

                if let Some(profile) = self.config.rules.fraud_injector.profiles.get(&meta.fraud_type) {
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

                    tx.card_present = false; 
                    
                    if let Some(amounts) = self.config.rules.fraud_patterns.get(&profile.amount_pattern) {
                        tx.amount = amounts[rng.random_range(0..amounts.len())];
                    }
                }

                if rng.random_bool(prob.geo_anomaly) {
                    tx.merchant_country = defaults.geo_anomaly_country.clone();
                    geo_anomaly = true;
                    tx.location_lat = rng.random_range(8.0..37.0); 
                    tx.location_long = rng.random_range(68.0..97.0); 
                }

                if rng.random_bool(prob.device_anomaly) {
                    tx.user_agent = self.config.rules.device_patterns.bot_user_agent_prefix.clone();
                    device_anomaly = true;
                }

                if rng.random_bool(prob.ip_anomaly) && !self.config.rules.device_patterns.known_bad_prefixes.is_empty() {
                    let prefixes: Vec<&String> = self.config.rules.device_patterns.known_bad_prefixes.keys().collect();
                    let selected = prefixes[rng.random_range(0..prefixes.len())];
                    tx.ip_address = format!("{}.{}", selected, rng.random_range(1..255));
                    ip_anomaly = true;
                }

                if rng.random_bool(prob.failure) {
                    tx.status = "Failed".to_string();
                    tx.auth_status = "declined".to_string();
                    
                    if let Some(reasons) = self.config.rules.failure_reasons_by_type.get(&meta.fraud_type) {
                        failure_reason = Some(reasons[rng.random_range(0..reasons.len())].clone());
                    } else {
                        failure_reason = Some(defaults.fallback_failure_reason.clone());
                    }
                }
                tx.failure_reason = failure_reason.clone();

                if rng.random_bool(prob.chargeback) {
                    tx.chargeback = true;
                    tx.chargeback_days = Some(defaults.chargeback_days);
                    chargeback_days = Some(defaults.chargeback_days);
                }

                Some((tx.transaction_id.clone(), geo_anomaly, device_anomaly, ip_anomaly, failure_reason, chargeback_days))
            } else {
                None
            }
        }).collect();

        for (id, geo, dev, ip, _fail, _cb_days) in tx_updates {
            if let Some(meta) = metadata_map.get_mut(&id) {
                meta.geo_anomaly = geo;
                meta.device_anomaly = dev;
                meta.ip_anomaly = ip;
            }
        }
    }
}

pub struct CampaignInjector<'a> {
    pub config: &'a AppConfig,
}

impl<'a> CampaignInjector<'a> {
    pub fn new(config: &'a AppConfig) -> Self {
        Self { config }
    }

    fn seeded_rng(&self, id: &str, salt: u64) -> StdRng {
        let mut seed = [0u8; 32];
        let id_bytes = id.as_bytes();
        for i in 0..32.min(id_bytes.len()) {
            seed[i] = id_bytes[i];
        }
        let s = self.config.rules.global.seed as u64 + salt;
        seed[24..32].copy_from_slice(&s.to_le_bytes());
        StdRng::from_seed(seed)
    }

    pub fn inject_campaigns(&self, transactions: &mut Vec<Transaction>, metadata_map: &mut HashMap<String, FraudMetadata>) {
        let campaign_salt = self.config.tuning.salts.campaign as u64;
        
        let mut card_to_tx_ids: HashMap<String, Vec<String>> = HashMap::new();
        for tx in transactions.iter() {
            if let Some(meta) = metadata_map.get(&tx.transaction_id) {
                if meta.fraud_target {
                    card_to_tx_ids.entry(tx.card_id.clone()).or_default().push(tx.transaction_id.clone());
                }
            }
        }

        let share = self.config.rules.fraud_campaigns.values().filter_map(|c| c.target_campaign_share).next().unwrap_or(0.15);
        
        let mut card_to_campaign: HashMap<String, (String, String)> = HashMap::new();
        for card_id in card_to_tx_ids.keys() {
            let mut rng = self.seeded_rng(card_id, campaign_salt);
            if rng.random_bool(share) {
                let campaigns: Vec<&String> = self.config.rules.fraud_campaigns.keys().collect();
                let campaign_type = campaigns[rng.random_range(0..campaigns.len())].clone();
                let campaign_id = format!("camp_{}_{}", campaign_type, &card_id[..8]);
                card_to_campaign.insert(card_id.clone(), (campaign_id, campaign_type));
            }
        }

        for tx in transactions.iter_mut() {
            if let Some((camp_id, camp_type)) = card_to_campaign.get(&tx.card_id) {
                if let Some(meta) = metadata_map.get_mut(&tx.transaction_id) {
                    meta.campaign_id = Some(camp_id.clone());
                    meta.campaign_type = Some(camp_type.clone());
                }
            }
        }

        self.apply_campaign_mutations(transactions, metadata_map);
    }

    fn apply_campaign_mutations(&self, transactions: &mut Vec<Transaction>, metadata_map: &mut HashMap<String, FraudMetadata>) {
        let mut campaign_groups: HashMap<String, Vec<String>> = HashMap::new();
        for tx in transactions.iter() {
            if let Some(meta) = metadata_map.get(&tx.transaction_id) {
                if let Some(camp_id) = &meta.campaign_id {
                    campaign_groups.entry(camp_id.clone()).or_default().push(tx.transaction_id.clone());
                }
            }
        }

        let campaign_params = &self.config.tuning.campaigns;

        for (_camp_id, tx_ids) in campaign_groups {
            let camp_type = metadata_map.get(&tx_ids[0]).and_then(|m| m.campaign_type.clone()).unwrap_or_default();
            
            let mut members: Vec<&mut Transaction> = transactions.iter_mut()
                .filter(|t| tx_ids.contains(&t.transaction_id))
                .collect();
            members.sort_by_key(|t| t.timestamp.clone());

            match camp_type.as_str() {
                "coordinated_upi_scam" => {
                    let shared_ip = campaign_params.coordinated_scam_ip.clone();
                    let shared_ua = self.config.rules.device_patterns.bot_user_agent_prefix.clone();
                    
                    for tx in members.iter_mut() {
                        tx.ip_address = shared_ip.clone();
                        tx.user_agent = shared_ua.clone();
                        tx.transaction_channel = "upi".to_string();
                        tx.card_present = false;
                        if let Some(meta) = metadata_map.get_mut(&tx.transaction_id) {
                            meta.ip_anomaly = true;
                            meta.device_anomaly = true;
                        }
                    }
                }
                "sequential_ato" => {
                    let escalation = campaign_params.ato_escalation_rate;
                    
                    let mut current_amount = 0.0;
                    for (i, tx) in members.iter_mut().enumerate() {
                        if i == 0 {
                            current_amount = tx.amount;
                        } else {
                            current_amount *= 1.0 + escalation;
                            tx.amount = current_amount;
                        }
                        if let Some(meta) = metadata_map.get_mut(&tx.transaction_id) {
                            meta.burst_session = true;
                            meta.burst_seq = Some(i as i32 + 1);
                        }
                    }
                }
                _ => {}
            }

            let total = members.len();
            for (i, tx) in members.iter().enumerate() {
                if let Some(meta) = metadata_map.get_mut(&tx.transaction_id) {
                    let phase = if i < total / 3 { "early" } else if i < 2 * total / 3 { "middle" } else { "late" };
                    meta.campaign_phase = Some(phase.to_string());
                    meta.campaign_day_number = Some((i / 5) as i32 + 1);
                }
            }
        }
    }
}
