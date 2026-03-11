use h3o::{CellIndex, Resolution};
use polars::prelude::*;
use riskfabric::config::AppConfig;
use riskfabric::generators::{account_gen, card_gen, customer_gen, transaction_gen};
use riskfabric::models::customer::Customer;
use std::collections::HashMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;

fn local_path(s: &str) -> PlPath {
    PlPath::Local(Arc::from(Path::new(s)))
}

fn main() {
    let config = AppConfig::load();
    let count = config.customer.control.customer_count;
    let total_start = Instant::now();

    fs::create_dir_all("data/output").expect("Could not create the directory");

    println!("🚀 Starting RiskFabric Synthetic Data Generation (Chunked Mode)");

    // --- 1. Customers ---
    let start = Instant::now();
    let customers = customer_gen::generate_customers(count);
    println!("   -> Customer generation took: {:?}", start.elapsed());

    let start_write = Instant::now();
    let customer_ids: Vec<String> = customers.iter().map(|c| c.customer_id.clone()).collect();

    let mut df_customers = df!(
        "customer_id" => &customer_ids,
        "name" => customers.iter().map(|c| c.name.clone()).collect::<Vec<_>>(),
        "age" => customers.iter().map(|c| c.age as u32).collect::<Vec<_>>(),
        "email" => customers.iter().map(|c| c.email.clone()).collect::<Vec<_>>(),
        "state" => customers.iter().map(|c| c.location.state.clone()).collect::<Vec<_>>(),
        "location" => customers.iter().map(|c| c.location.location.clone()).collect::<Vec<_>>(),
        "location_type" => customers.iter().map(|c| c.location.location_type.clone()).collect::<Vec<_>>(),
        "home_latitude" => customers.iter().map(|c| c.location.home_latitude).collect::<Vec<_>>(),
        "home_longitude" => customers.iter().map(|c| c.location.home_longitude).collect::<Vec<_>>(),
        "home_h3r5" => customers.iter().map(|c| c.location.home_h3r5.clone()).collect::<Vec<_>>(),
        "home_h3r7" => customers.iter().map(|c| c.location.home_h3r7.clone()).collect::<Vec<_>>(),
        "credit_score" => customers.iter().map(|c| c.financial.credit_score as u32).collect::<Vec<_>>(),
        "monthly_spend" => customers.iter().map(|c| c.financial.monthly_spend).collect::<Vec<_>>(),
        "customer_risk_score" => customers.iter().map(|c| c.financial.customer_risk_score as f64).collect::<Vec<_>>(),
        "is_fraud" => customers.iter().map(|c| c.financial.is_fraud).collect::<Vec<_>>(),
        "registration_date" => customers.iter().map(|c| c.registration_date.clone()).collect::<Vec<_>>()
    )
    .expect("Failed to create Customer DataFrame");

    let file_cust =
        File::create("data/output/customers.parquet").expect("Could not create customer file");
    ParquetWriter::new(file_cust)
        .finish(&mut df_customers)
        .expect("Write Failed");
    println!(
        "   -> Customer Parquet write took: {:?}",
        start_write.elapsed()
    );

    // --- 2. Accounts ---
    let start = Instant::now();
    let accounts = account_gen::generate_accounts(customer_ids);
    println!("   -> Account generation took: {:?}", start.elapsed());

    let start_write = Instant::now();
    let mut df_accounts = df!(
        "account_id" => accounts.iter().map(|a| a.account_id.clone()).collect::<Vec<_>>(),
        "customer_id" => accounts.iter().map(|a| a.customer_id.clone()).collect::<Vec<_>>(),
        "bank_id" => accounts.iter().map(|a| a.bank_id.clone()).collect::<Vec<_>>(),
        "account_no" => accounts.iter().map(|a| a.account_no.clone()).collect::<Vec<_>>(),
        "account_type" => accounts.iter().map(|a| a.account_no.clone()).collect::<Vec<_>>(),
        "balance" => accounts.iter().map(|a| a.balance).collect::<Vec<_>>(),
        "status" => accounts.iter().map(|a| a.account_status.clone()).collect::<Vec<_>>(),
        "creation_date" => accounts.iter().map(|a| a.creation_date.clone()).collect::<Vec<_>>()
    )
    .expect("Failed to create Account dataframe");

    let file_acct =
        File::create("data/output/accounts.parquet").expect("Could not create account file");
    ParquetWriter::new(file_acct)
        .finish(&mut df_accounts)
        .expect("Write Failed");
    println!(
        "   -> Account Parquet write took: {:?}",
        start_write.elapsed()
    );

    // --- 3. Cards ---
    let start = Instant::now();
    let cards = card_gen::generate_for_accounts(&accounts);
    println!("   -> Card generation took: {:?}", start.elapsed());

    let start_write = Instant::now();
    let mut df_cards = df!(
        "card_id" => cards.iter().map(|c| c.card_id.clone()).collect::<Vec<_>>(),
        "account_id" => cards.iter().map(|c| c.account_id.clone()).collect::<Vec<_>>(),
        "customer_id" => cards.iter().map(|c| c.customer_id.clone()).collect::<Vec<_>>(),
        "card_number" => cards.iter().map(|c| c.card_number.clone()).collect::<Vec<_>>(),
        "card_network" => cards.iter().map(|c| c.card_network.clone()).collect::<Vec<_>>(),
        "card_type" => cards.iter().map(|c| c.card_type.clone()).collect::<Vec<_>>(),
        "status" => cards.iter().map(|c| c.status.clone()).collect::<Vec<_>>(),
        "status_reason" => cards.iter().map(|c| c.status_reason.clone()).collect::<Vec<_>>(),
        "issue_date" => cards.iter().map(|c| c.issue_date.clone()).collect::<Vec<_>>(),
        "activation_date" => cards.iter().map(|c| c.activation_date.clone()).collect::<Vec<_>>(),
        "expiry_date" => cards.iter().map(|c| c.expiry_date.clone()).collect::<Vec<_>>(),
        "contactless_limit" => cards.iter().map(|c| c.contactless_limit.clone()).collect::<Vec<_>>(),
        "daily_atm_limit" => cards.iter().map(|c| c.daily_atm_limit.clone()).collect::<Vec<_>>(),
        "online_limit" => cards.iter().map(|c| c.online_limit.clone()).collect::<Vec<_>>(),
        "international_usage" => cards.iter().map(|c| c.international_usage.clone()).collect::<Vec<_>>(),
        "issuing_bank" => cards.iter().map(|c| c.issuing_bank.clone()).collect::<Vec<_>>(),
        "bank_code" => cards.iter().map(|c| c.bank_code.clone()).collect::<Vec<_>>()
    )
    .expect("Failed to create Card dataframe");

    let file_card = File::create("data/output/cards.parquet").expect("Could not create card file");
    ParquetWriter::new(file_card)
        .finish(&mut df_cards)
        .expect("Write Failed");
    println!("   -> Card Parquet write took: {:?}", start_write.elapsed());

    // --- 4. Transactions (Chunked) ---
    println!("   ... initializing spatial indices and merchant data");
    let file = File::open("data/references/ref_merchants.parquet").expect("Merchant data missing");
    let df_merch = ParquetReader::new(file)
        .finish()
        .expect("Failed to read Parquet");

    let merchants = (
        df_merch
            .column("h3_index")
            .unwrap()
            .str()
            .unwrap()
            .into_no_null_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
        df_merch
            .column("merchant_name")
            .unwrap()
            .str()
            .unwrap()
            .into_no_null_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
        df_merch
            .column("latitude")
            .unwrap()
            .f64()
            .unwrap()
            .into_no_null_iter()
            .collect::<Vec<_>>(),
        df_merch
            .column("longitude")
            .unwrap()
            .f64()
            .unwrap()
            .into_no_null_iter()
            .collect::<Vec<_>>(),
        df_merch
            .column("merchant_category")
            .unwrap()
            .str()
            .unwrap()
            .into_no_null_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
        df_merch
            .column("osm_id")
            .unwrap()
            .i64()
            .unwrap()
            .into_no_null_iter()
            .collect::<Vec<_>>(),
        df_merch
            .column("state")
            .unwrap()
            .str()
            .unwrap()
            .into_no_null_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
    );

    // Build multi-tier spatial indices
    let mut index_res6: HashMap<String, Vec<usize>> = HashMap::new();
    let mut index_res4: HashMap<String, Vec<usize>> = HashMap::new();
    let mut index_state: HashMap<String, Vec<usize>> = HashMap::new();

    for (idx, h3_str) in merchants.0.iter().enumerate() {
        if let Ok(cell) = CellIndex::from_str(h3_str) {
            let p6 = cell.parent(Resolution::Six).unwrap().to_string();
            let p4 = cell.parent(Resolution::Four).unwrap().to_string();
            index_res6.entry(p6).or_default().push(idx);
            index_res4.entry(p4).or_default().push(idx);
        }
        let state = &merchants.6[idx];
        index_state.entry(state.clone()).or_default().push(idx);
    }
    let spatial_indices = (index_res6, index_res4, index_state);

    let customer_map: HashMap<String, &Customer> = customers
        .iter()
        .map(|c| (c.customer_id.clone(), c))
        .collect();

    println!("   ... generating transactions in chunks of 5,000 cards");
    fs::create_dir_all("data/output/tmp").expect("Could not create tmp directory");
    let mut chunk_count = 0;
    let mut total_tx_count = 0;

    for chunk in cards.chunks(5000) {
        chunk_count += 1;
        let (txs, meta) = transaction_gen::generate_transactions_chunk(
            chunk,
            &customer_map,
            &spatial_indices,
            &merchants,
            &config,
        );
        total_tx_count += txs.len();

        let mut df_txs = df!(
            "transaction_id" => txs.iter().map(|t| t.transaction_id.clone()).collect::<Vec<_>>(),
            "card_id" => txs.iter().map(|t| t.card_id.clone()).collect::<Vec<_>>(),
            "account_id" => txs.iter().map(|t| t.account_id.clone()).collect::<Vec<_>>(),
            "customer_id" => txs.iter().map(|t| t.customer_id.clone()).collect::<Vec<_>>(),
            "merchant_id" => txs.iter().map(|t| t.merchant_id.clone()).collect::<Vec<_>>(),
            "merchant_name" => txs.iter().map(|t| t.merchant_name.clone()).collect::<Vec<_>>(),
            "merchant_category" => txs.iter().map(|t| t.merchant_category.clone()).collect::<Vec<_>>(),
            "mcc" => txs.iter().map(|t| t.mcc.clone()).collect::<Vec<_>>(),
            "merchant_country" => txs.iter().map(|t| t.merchant_country.clone()).collect::<Vec<_>>(),
            "amount" => txs.iter().map(|t| t.amount).collect::<Vec<_>>(),
            "currency" => txs.iter().map(|t| t.currency.clone()).collect::<Vec<_>>(),
            "timestamp" => txs.iter().map(|t| t.timestamp.clone()).collect::<Vec<_>>(),
            "transaction_channel" => txs.iter().map(|t| t.transaction_channel.clone()).collect::<Vec<_>>(),
            "card_present" => txs.iter().map(|t| t.card_present).collect::<Vec<_>>(),
            "user_agent" => txs.iter().map(|t| t.user_agent.clone()).collect::<Vec<_>>(),
            "ip_address" => txs.iter().map(|t| t.ip_address.clone()).collect::<Vec<_>>(),
            "status" => txs.iter().map(|t| t.status.clone()).collect::<Vec<_>>(),
            "auth_status" => txs.iter().map(|t| t.auth_status.clone()).collect::<Vec<_>>(),
            "failure_reason" => txs.iter().map(|t| t.failure_reason.clone()).collect::<Vec<_>>(),
            "is_fraud" => txs.iter().map(|t| t.is_fraud).collect::<Vec<_>>(),
            "chargeback" => txs.iter().map(|t| t.chargeback).collect::<Vec<_>>(),
            "chargeback_days" => txs.iter().map(|t| t.chargeback_days).collect::<Vec<_>>(),
            "location_lat" => txs.iter().map(|t| t.location_lat).collect::<Vec<_>>(),
            "location_long" => txs.iter().map(|t| t.location_long).collect::<Vec<_>>(),
            "h3_r7" => txs.iter().map(|t| t.h3_r7.clone()).collect::<Vec<_>>()
        ).expect("Failed to create Tx Chunk DataFrame");

        let file_tx = File::create(format!("data/output/tmp/tx_{}.parquet", chunk_count)).expect("Could not create tmp tx file");
        ParquetWriter::new(file_tx).finish(&mut df_txs).expect("Write Failed");

        let mut df_meta = df!(
            "transaction_id" => meta.iter().map(|m| m.transaction_id.clone()).collect::<Vec<_>>(),
            "fraud_target" => meta.iter().map(|m| m.fraud_target).collect::<Vec<_>>(),
            "fraud_type" => meta.iter().map(|m| m.fraud_type.clone()).collect::<Vec<_>>(),
            "label_noise" => meta.iter().map(|m| m.label_noise.clone()).collect::<Vec<_>>(),
            "injector_version" => meta.iter().map(|m| m.injector_version.clone()).collect::<Vec<_>>(),
            "geo_anomaly" => meta.iter().map(|m| m.geo_anomaly).collect::<Vec<_>>(),
            "device_anomaly" => meta.iter().map(|m| m.device_anomaly).collect::<Vec<_>>(),
            "ip_anomaly" => meta.iter().map(|m| m.ip_anomaly).collect::<Vec<_>>(),
            "burst_session" => meta.iter().map(|m| m.burst_session).collect::<Vec<_>>(),
            "burst_seq" => meta.iter().map(|m| m.burst_seq).collect::<Vec<_>>(),
            "campaign_id" => meta.iter().map(|m| m.campaign_id.clone()).collect::<Vec<_>>(),
            "campaign_type" => meta.iter().map(|m| m.campaign_type.clone()).collect::<Vec<_>>(),
            "campaign_phase" => meta.iter().map(|m| m.campaign_phase.clone()).collect::<Vec<_>>(),
            "campaign_day_number" => meta.iter().map(|m| m.campaign_day_number).collect::<Vec<_>>()
        ).expect("Failed to create Meta Chunk DataFrame");

        let file_meta = File::create(format!("data/output/tmp/meta_{}.parquet", chunk_count)).expect("Could not create tmp meta file");
        ParquetWriter::new(file_meta).finish(&mut df_meta).expect("Write Failed");

        print!(".");
        use std::io::Write;
        std::io::stdout().flush().unwrap();
    }
    println!("\n   -> Total Transactions generated: {}", total_tx_count);

    let start_write = Instant::now();
    
    // Merge Transactions
    println!("   ... merging transaction chunks");
    let mut tx_lfs = Vec::new();
    for i in 1..=chunk_count {
        let path = local_path(&format!("data/output/tmp/tx_{}.parquet", i));
        let lf = LazyFrame::scan_parquet(path, Default::default())
            .expect("Failed to scan tmp tx file");
        tx_lfs.push(lf);
    }
    
    let mut df_final_txs = concat(tx_lfs, UnionArgs::default())
        .expect("Failed to concat tx lfs")
        .collect()
        .expect("Failed to merge tx files");

    let file_txn = File::create("data/output/transactions.parquet").expect("Could not create final transaction file");
    ParquetWriter::new(file_txn).finish(&mut df_final_txs).expect("Write Failed");

    // Merge Metadata
    println!("   ... merging metadata chunks");
    let mut meta_lfs = Vec::new();
    for i in 1..=chunk_count {
        let path = local_path(&format!("data/output/tmp/meta_{}.parquet", i));
        let lf = LazyFrame::scan_parquet(path, Default::default())
            .expect("Failed to scan tmp meta file");
        meta_lfs.push(lf);
    }
    
    let mut df_final_meta = concat(meta_lfs, UnionArgs::default())
        .expect("Failed to concat meta lfs")
        .collect()
        .expect("Failed to merge meta files");

    let file_meta = File::create("data/output/fraud_metadata.parquet").expect("Could not create final metadata file");
    ParquetWriter::new(file_meta).finish(&mut df_final_meta).expect("Write Failed");

    // Clean up tmp directory
    fs::remove_dir_all("data/output/tmp").expect("Could not remove tmp directory");

    println!(
        "   -> Transaction & Metadata Parquet merge took: {:?}",
        start_write.elapsed()
    );

    let total_duration = total_start.elapsed();
    println!("✅ All operations completed in {:?}", total_duration);
    println!("📁 Output files saved to data/output/");
}
