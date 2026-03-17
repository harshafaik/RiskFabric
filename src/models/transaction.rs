use serde::{Deserialize, Serialize};
use crate::config::AppConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerchantInfo {
    pub id: String,
    pub name: String,
    pub category: String,
    pub mcc: String,
    pub lat: f64,
    pub long: f64,
    pub h3_r7: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub transaction_id: String,
    pub card_id: String,
    pub account_id: String,
    pub customer_id: String,

    pub merchant_id: String,
    pub merchant_name: String,
    pub merchant_category: String,
    pub mcc: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlabeledTransaction {
    pub transaction_id: String,
    pub card_id: String,
    pub account_id: String,
    pub customer_id: String,
    pub merchant_id: String,
    pub merchant_name: String,
    pub merchant_category: String,
    pub mcc: String,
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
    pub location_lat: f64,
    pub location_long: f64,
    pub h3_r7: String,
}

impl From<Transaction> for UnlabeledTransaction {
    fn from(t: Transaction) -> Self {
        UnlabeledTransaction {
            transaction_id: t.transaction_id,
            card_id: t.card_id,
            account_id: t.account_id,
            customer_id: t.customer_id,
            merchant_id: t.merchant_id,
            merchant_name: t.merchant_name,
            merchant_category: t.merchant_category,
            mcc: t.mcc,
            merchant_country: t.merchant_country,
            amount: t.amount,
            currency: t.currency,
            timestamp: t.timestamp,
            transaction_channel: t.transaction_channel,
            card_present: t.card_present,
            user_agent: t.user_agent,
            ip_address: t.ip_address,
            status: t.status,
            auth_status: t.auth_status,
            failure_reason: t.failure_reason,
            location_lat: t.location_lat,
            location_long: t.location_long,
            h3_r7: t.h3_r7,
        }
    }
}

impl Transaction {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        transaction_id: String,
        card_id: String,
        account_id: String,
        customer_id: String,
        merchant: MerchantInfo,
        amount: f64,
        timestamp: chrono::DateTime<chrono::Utc>,
        channel: String,
        user_agent: String,
        ip_address: String,
        status_info: (String, String, Option<String>),
        is_fraud: bool,
        card_present: bool,
        config: &AppConfig,
    ) -> Self {
        let (status, auth_status, failure_reason) = status_info;

        Transaction {
            transaction_id,
            card_id,
            account_id,
            customer_id,
            merchant_id: merchant.id,
            merchant_name: merchant.name,
            merchant_category: merchant.category,
            mcc: merchant.mcc,
            merchant_country: config.rules.global.default_country.clone(),
            amount,
            currency: config.rules.global.base_currency.clone(),
            timestamp: timestamp.to_rfc3339(),
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
            location_lat: merchant.lat,
            location_long: merchant.long,
            h3_r7: merchant.h3_r7,
        }
    }
}
