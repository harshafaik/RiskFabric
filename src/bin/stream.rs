use h3o::{CellIndex, Resolution};
use polars::prelude::*;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use riskfabric::config::AppConfig;
use riskfabric::generators::{account_gen, card_gen, customer_gen, transaction_gen};
use riskfabric::models::transaction::UnlabeledTransaction;
use std::collections::HashMap;
use std::fs::File;
use std::str::FromStr;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = AppConfig::load();
    config.transactions.transactions.streaming_mode = false; // Generate metadata for verification
    let rate = config.transactions.transactions.streaming_rate;

    println!("🚀 Starting Verification Stream Generator (capturing ground truth)");

    use std::io::Write;
    let mut gt_file = std::fs::File::create("ground_truth.csv")?;
    writeln!(gt_file, "transaction_id,is_fraud")?;

    // ... (rest of setup)

    // 1. Initial Data Setup (simplified from generate.rs)
    let count = 1000; // Sample population for streaming
    let customers = customer_gen::generate_customers(count);
    let customer_ids: Vec<String> = customers.iter().map(|c| c.customer_id.clone()).collect();
    let accounts = account_gen::generate_accounts(customer_ids);
    let cards = card_gen::generate_for_accounts(&accounts);
    let customer_map: HashMap<String, _> = customers
        .iter()
        .map(|c| (c.customer_id.clone(), c))
        .collect();

    // 2. Spatial Indices
    let file = File::open("data/references/ref_merchants.parquet").expect("Merchant data missing");
    let df_merch = ParquetReader::new(file)
        .finish()
        .expect("Failed to read Parquet");
    let merchants = (
        df_merch
            .column("h3_index")?
            .str()?
            .into_no_null_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
        df_merch
            .column("merchant_name")?
            .str()?
            .into_no_null_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
        df_merch
            .column("latitude")?
            .f64()?
            .into_no_null_iter()
            .collect::<Vec<_>>(),
        df_merch
            .column("longitude")?
            .f64()?
            .into_no_null_iter()
            .collect::<Vec<_>>(),
        df_merch
            .column("merchant_category")?
            .str()?
            .into_no_null_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
        df_merch
            .column("osm_id")?
            .i64()?
            .into_no_null_iter()
            .collect::<Vec<_>>(),
        df_merch
            .column("state")?
            .str()?
            .into_no_null_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
    );

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

    // 3. Kafka Producer Setup
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", "localhost:9092")
        .set("message.timeout.ms", "5000")
        .create()?;

    println!("   -> Connected to Kafka at localhost:9092");

    // 4. Streaming Loop
    let mut interval = Duration::from_micros(1_000_000 / rate as u64);
    let mut total_sent = 0;

    loop {
        // Generate a chunk of transactions
        let (txs, meta) = transaction_gen::generate_transactions_chunk(
            &cards,
            &customer_map,
            &spatial_indices,
            &merchants,
            &config,
        );

        // Record ground truth
        for m in meta {
            writeln!(
                gt_file,
                "{},{}",
                m.transaction_id,
                if m.fraud_target { 1 } else { 0 }
            )?;
        }
        gt_file.flush()?;

        for tx in txs {
            let unlabeled: UnlabeledTransaction = tx.into();
            let payload = serde_json::to_string(&unlabeled)?;

            let record = FutureRecord::to("raw_transactions")
                .payload(&payload)
                .key(&unlabeled.transaction_id);

            let start = Instant::now();
            let _ = producer.send(record, Duration::from_secs(0)).await;

            total_sent += 1;
            if total_sent % 100 == 0 {
                println!("   -> Sent {} transactions", total_sent);
            }

            let elapsed = start.elapsed();
            if interval > elapsed {
                sleep(interval - elapsed).await;
            }
        }
    }
}
