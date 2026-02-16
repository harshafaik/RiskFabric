use crate::models::account::Account;
use crate::models::card::Card;
use rayon::prelude::*;

pub fn generate_for_accounts(accounts: &Vec<Account>) -> Vec<Card> {
    accounts
        .par_iter()
        .map(|acc| {
            Card::new(
                acc.account_id.clone(),
                acc.customer_id.clone(),
                acc.bank_id.clone(),
            )
        })
        .collect()
}
