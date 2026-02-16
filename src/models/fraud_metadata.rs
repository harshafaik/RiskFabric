use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraudMetadata {
    pub transaction_id: String,
    pub fraud_target: bool,
    pub fraud_type: String,
    pub label_noise: String,
    pub injector_version: String,
    pub geo_anomaly: bool,
    pub device_anomaly: bool,
    pub ip_anomaly: bool,
    pub burst_session: bool,
    pub burst_seq: Option<i32>,
    pub campaign_id: Option<String>,
    pub campaign_type: Option<String>,
    pub campaign_phase: Option<String>,
    pub campaign_day_number: Option<i32>,
}
