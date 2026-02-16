use crate::models::account::Account;
use rayon::prelude::*;

pub fn generate_accounts(customer_ids: Vec<String>) -> Vec<Account> {
    let accounts: Vec<Account> = customer_ids
        .into_par_iter()
        .flat_map(|cid| {
            let mut user_accounts = Vec::new();

            // Primary Account
            user_accounts.push(Account::new(cid.clone()));

            // Optional Secondary Account (50% chance)
            if rand::random() {
                user_accounts.push(Account::new(cid));
            }

            user_accounts
        })
        .collect(); // <--- Data is materialized here

    // 2. NOW we know the real count
    println!(
        "   -> Generated {} accounts (Average {:.2} per customer)",
        accounts.len(),
        accounts.len() as f64 / (accounts.len() as f64 * 0.66)
    );

    // 3. Return the vector
    accounts
}
