use chrono::{Duration, Utc};
use fake::Fake;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    //
    pub card_id: String,
    pub account_id: String,
    pub customer_id: String,

    pub card_number: String,
    pub card_network: String,
    pub card_type: String,

    pub status: String,
    pub status_reason: String,

    pub issue_date: String,
    pub activation_date: String,
    pub expiry_date: String,

    pub contactless_limit: String,
    pub daily_atm_limit: String,
    pub online_limit: String,

    pub international_usage: String,
    pub issuing_bank: String,
    pub bank_code: String,
}
impl Card {
    pub fn new(account_id: String, customer_id: String, bank_id: String) -> Self {
        let mut rng = rand::rng();

        let network = ["VISA", "Mastercard", "RuPay"];
        let selected_network = network[rng.random_range(0..network.len())].to_string();

        let types = ["Debit", "Credit"];
        let selected_types = types[rng.random_range(0..types.len())].to_string();

        let days_since_issue = rng.random_range(1..(365 * 4));
        let issue_dt = Utc::now() - Duration::days(days_since_issue);
        let expiry_dt = issue_dt + Duration::days(365 * 3);
        let now = Utc::now();
        let days_to_expiry = (expiry_dt - now).num_days();
        let activation_dt = issue_dt + Duration::days(rng.random_range(2..5));

        let status = if days_to_expiry <= 0 {
            "Expired"
        } else {
            if rng.random_bool(0.90) {
                "Active"
            } else {
                "Blocked"
            }
        };

        let status_reason = match status {
            "Active" => "Normal usage",
            "Blocked" => "Suspected Fraud", // Or "Lost/Stolen"
            "Expired" => "Card Validity Ended",
            _ => "Unknown",
        };
        Card {
            card_id: uuid::Uuid::new_v4().to_string(),
            account_id,
            customer_id,
            card_number: (4000_0000_0000_0000_u64..4999_9999_9999_9999_u64)
                .fake::<u64>()
                .to_string(),
            card_network: selected_network,
            card_type: selected_types,

            status: status.to_string(),
            status_reason: status_reason.to_string(),

            issue_date: issue_dt.format("%Y-%m-%d").to_string(),
            activation_date: activation_dt.format("%Y-%m-%d").to_string(),
            expiry_date: expiry_dt.format("%Y-%m-%d").to_string(),

            contactless_limit: "".to_string(), // Placeholder - will be set properly later
            daily_atm_limit: "".to_string(),   // Placeholder - will be set properly later
            online_limit: "".to_string(),      // Placeholder - will be set properly later

            international_usage: "".to_string(), // Placeholder - will be set properly later
            issuing_bank: format!("Bank of {}", bank_id),
            bank_code: bank_id,
        }
    }
}
