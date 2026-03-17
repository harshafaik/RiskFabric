use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub rules: FraudRules,
    pub tuning: FraudTuning,
    pub customer: CustomerConfig,
    pub transactions: TransactionConfig,
    pub product_catalog: ProductCatalog,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraudRules {
    pub global: GlobalConfig,
    pub payment_channels: HashMap<String, ChannelConfig>,
    pub fraud_patterns: HashMap<String, Vec<f64>>,
    pub device_patterns: DevicePatterns,
    pub fraud_injector: FraudInjectorConfig,
    pub fraud_campaigns: HashMap<String, FraudCampaignConfig>,
    pub temporal_patterns: HashMap<String, TemporalPatternConfig>,
    pub failure_reasons_by_type: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub seed: i32,
    pub base_currency: String,
    pub default_country: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub market_share: f64,
    pub risk_level: f64,
    pub user_agents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevicePatterns {
    pub ip_prefixes: Vec<String>,
    pub bot_user_agent_prefix: String,
    pub known_bad_prefixes: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraudInjectorConfig {
    pub target_share: f64,
    pub default_fp_rate: f64,
    pub default_fn_rate: f64,
    pub profiles: HashMap<String, FraudProfileConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraudProfileConfig {
    pub frequency: f64,
    pub amount_pattern: String,
    pub amount_strategy: Option<String>,
    pub amount_multiplier: Option<String>,
    pub channel_bias: HashMap<String, f64>,
    pub geo_anomaly_prob: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraudCampaignConfig {
    pub frequency: f64,
    pub target_campaign_share: Option<f64>,
    pub amount_escalation: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalPatternConfig {
    pub hourly_weights: Vec<f64>,
    pub daily_weights: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraudTuning {
    pub probabilities: ProbabilitiesConfig,
    pub defaults: DefaultsConfig,
    pub campaigns: TuningCampaignConfig,
    pub salts: SaltsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilitiesConfig {
    pub geo_anomaly: f64,
    pub device_anomaly: f64,
    pub ip_anomaly: f64,
    pub failure: f64,
    pub chargeback: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsConfig {
    pub geo_anomaly_country: String,
    pub fallback_failure_reason: String,
    pub chargeback_days: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuningCampaignConfig {
    pub ato_escalation_rate: f64,
    pub coordinated_scam_ip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaltsConfig {
    pub injector: i32,
    pub mutator: i32,
    pub campaign: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerConfig {
    pub names: NamesConfig,
    pub email: EmailConfig,
    pub locations: LocationsConfig,
    pub financials: FinancialsConfig,
    pub registration: RegistrationConfig,
    pub control: GenerationControl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamesConfig {
    pub first_names: Vec<String>,
    pub last_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub domains: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationsConfig {
    pub types: Vec<String>,
    pub metro_cities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialsConfig {
    pub base_spend: HashMap<String, f64>,
    pub credit_score: CreditScoreConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditScoreConfig {
    pub base: i32,
    pub age_weight: f64,
    pub min: u16,
    pub max: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationConfig {
    pub lookback_years: i32,
    pub default_location_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationControl {
    pub customer_count: usize,
    pub transactions_per_customer: RangeControl,
    pub parallelism: ParallelismControl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeControl {
    pub min: usize,
    pub max: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelismControl {
    pub customer_gen_threads: usize,
    pub transaction_gen_threads: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionConfig {
    pub transactions: TransactionBaseConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionBaseConfig {
    pub merchant_categories: Vec<CategoryMccMap>,
    pub amount_range: Vec<f64>,
    pub success_rate: f64,
    pub card_present_probability: f64,
    pub geo_bounds: GeoBounds,
    pub lookback_days: i32,
    pub status_codes: HashMap<String, Vec<String>>,
    pub temporal_patterns: TemporalPatternConfig,
    #[serde(default)]
    pub streaming_mode: bool,
    #[serde(default = "default_streaming_rate")]
    pub streaming_rate: u32,
}

fn default_streaming_rate() -> u32 { 100 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryMccMap {
    pub name: String,
    pub mcc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoBounds {
    pub lat_range: Vec<f64>,
    pub long_range: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductCatalog {
    pub accounts: AccountCatalogConfig,
    pub cards: CardCatalogConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountCatalogConfig {
    pub types: Vec<String>,
    pub creation_window_years: i32,
    pub bank_id_range: Vec<i32>,
    pub balance_range: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardCatalogConfig {
    pub networks: Vec<String>,
    pub types: Vec<String>,
    pub issue_window_years: i32,
    pub expiry_duration_years: i32,
    pub activation_delay_days: Vec<i32>,
    pub active_probability: f64,
    pub limits: CardLimitsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardLimitsConfig {
    pub contactless_default: String,
    pub daily_atm_default: String,
    pub online_default: String,
    pub international_enabled_prob: f64,
}

impl AppConfig {
    pub fn load() -> Self {
        let rules_yaml = fs::read_to_string("data/config/fraud_rules.yaml")
            .expect("Failed to read data/config/fraud_rules.yaml");
        let tuning_yaml = fs::read_to_string("data/config/fraud_tuning.yaml")
            .expect("Failed to read data/config/fraud_tuning.yaml");
        let customer_yaml = fs::read_to_string("data/config/customer_config.yaml")
            .expect("Failed to read data/config/customer_config.yaml");
        let transaction_yaml = fs::read_to_string("data/config/transaction_config.yaml")
            .expect("Failed to read data/config/transaction_config.yaml");
        let product_yaml = fs::read_to_string("data/config/product_catalog.yaml")
            .expect("Failed to read data/config/product_catalog.yaml");

        let rules: FraudRules = serde_yaml::from_str(&rules_yaml)
            .expect("Failed to parse fraud_rules.yaml");
        let tuning: FraudTuning = serde_yaml::from_str(&tuning_yaml)
            .expect("Failed to parse fraud_tuning.yaml");
        let customer: CustomerConfig = serde_yaml::from_str(&customer_yaml)
            .expect("Failed to parse customer_config.yaml");
        let transactions: TransactionConfig = serde_yaml::from_str(&transaction_yaml)
            .expect("Failed to parse transaction_config.yaml");
        let product_catalog: ProductCatalog = serde_yaml::from_str(&product_yaml)
            .expect("Failed to parse product_catalog.yaml");

        AppConfig { 
            rules, 
            tuning, 
            customer, 
            transactions,
            product_catalog
        }
    }
}
