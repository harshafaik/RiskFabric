use crate::models::account::Account;
use crate::models::card::Card;
use rand::{SeedableRng, rngs::StdRng};
use rayon::prelude::*;

pub fn generate_for_accounts(accounts: &Vec<Account>, base_seed: u64) -> Vec<Card> {
    let salt_card: u64 = 0x00CA_0000_0000_0003;

    accounts
        .par_iter()
        .enumerate()
        .map(|(i, acc)| {
            let mut rng = StdRng::seed_from_u64(base_seed ^ salt_card ^ (i as u64));
            Card::new(
                acc.account_id.clone(),
                acc.customer_id.clone(),
                acc.bank_id.clone(),
                &mut rng,
            )
        })
        .collect()
}
