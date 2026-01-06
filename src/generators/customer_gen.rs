use crate::models::customer::Customer;
use rayon::prelude::*;

pub fn generate_bulk(count: usize) -> Vec<Customer> {
    (0..count)
        .into_par_iter()
        .map(|_| Customer::random())
        .collect()
}
