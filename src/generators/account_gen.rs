use crate::models::account::Account;
use rand::{Rng, SeedableRng, rngs::StdRng};
use rayon::prelude::*;

pub fn generate_accounts(customer_ids: Vec<String>, base_seed: u64) -> Vec<Account> {
    let salt_account: u64 = 0x0000_AC02_0000_0002;

    let accounts: Vec<Account> = customer_ids
        .into_par_iter()
        .enumerate()
        .flat_map(|(i, cid)| {
            let mut rng = StdRng::seed_from_u64(base_seed ^ salt_account ^ (i as u64));
            let mut user_accounts = Vec::new();

            user_accounts.push(Account::new(cid.clone(), &mut rng));

            if rng.random_bool(0.5) {
                user_accounts.push(Account::new(cid, &mut rng));
            }

            user_accounts
        })
        .collect();

    println!(
        "   -> Generated {} accounts (Average {:.2} per customer)",
        accounts.len(),
        accounts.len() as f64 / (accounts.len() as f64 * 0.66)
    );

    accounts
}
