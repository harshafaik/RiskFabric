use std::collections::HashMap;
use std::io::Write;
use std::str::FromStr;
use std::time::{Duration, Instant};

use h3o::{CellIndex, Resolution};
use polars::prelude::*;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::config::AppConfig;
use crate::generators::{
    account_gen, card_gen, customer_gen, transaction_gen, MerchantData, SpatialIndices,
};
use crate::models::transaction::UnlabeledTransaction;
use crate::pipeline::events::{KafkaStatus, ProgressEvent};

pub struct StreamHandle {
    cancel: CancellationToken,
    task: JoinHandle<Result<(), String>>,
    events_rx: tokio::sync::Mutex<mpsc::Receiver<ProgressEvent>>,
}

impl StreamHandle {
    pub fn stop(&self) {
        self.cancel.cancel();
    }

    pub async fn wait(self) -> Result<(), String> {
        drop(self.cancel);
        match self.task.await {
            Ok(inner) => inner,
            Err(join_err) => Err(format!("Stream task panicked: {}", join_err)),
        }
    }

    pub async fn try_recv(&self) -> Option<ProgressEvent> {
        let mut rx = self.events_rx.lock().await;
        rx.try_recv().ok()
    }
}

async fn stream_loop(
    cancel: CancellationToken,
    events_tx: mpsc::Sender<ProgressEvent>,
    config: AppConfig,
    customer_count: usize,
    residential_ref_path: String,
    merchants_ref_path: String,
    kafka_bootstrap: String,
    kafka_topic: String,
    ground_truth_path: String,
) -> Result<(), String> {
    let rate = config.transactions.transactions.streaming_rate;
    let interval = Duration::from_micros(1_000_000 / rate as u64);

    let _ = events_tx
        .send(ProgressEvent::StreamStatus {
            status: "Generating initial data...".into(),
        })
        .await;

    let seed = config.rules.global.seed as u64;
    let customers = customer_gen::generate_customers(customer_count, seed)
        .map_err(|e| format!("Failed to generate customers: {}", e))?;

    let customer_ids: Vec<String> = customers.iter().map(|c| c.customer_id.clone()).collect();
    let accounts = account_gen::generate_accounts(customer_ids, seed);
    let cards = card_gen::generate_for_accounts(&accounts, seed);

    let customer_map: HashMap<String, _> = customers
        .iter()
        .map(|c| (c.customer_id.clone(), c))
        .collect();

    // Build spatial indices
    let (spatial_indices, merchants) =
        build_indices_async(&merchants_ref_path).map_err(|e| format!("Spatial indices: {}", e))?;

    // Kafka producer
    let _ = events_tx
        .send(ProgressEvent::StreamStatus {
            status: format!("Connecting to Kafka at {}", kafka_bootstrap),
        })
        .await;

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &kafka_bootstrap)
        .set("message.timeout.ms", "5000")
        .create()
        .map_err(|e| format!("Kafka producer error: {}", e))?;

    // Ground truth file
    let mut gt_file = std::fs::File::create(&ground_truth_path)
        .map_err(|e| format!("Cannot create {}: {}", ground_truth_path, e))?;
    writeln!(gt_file, "transaction_id,is_fraud")
        .map_err(|e| format!("Write error: {}", e))?;

    let _ = events_tx
        .send(ProgressEvent::StreamStatus {
            status: "Streaming started".into(),
        })
        .await;

    let mut total_sent: u64 = 0;
    let mut total_fraud: u64 = 0;
    let start = Instant::now();

    loop {
        if cancel.is_cancelled() {
            let _ = events_tx
                .send(ProgressEvent::StreamStatus {
                    status: "Shutting down...".into(),
                })
                .await;
            break;
        }

        let (txs, meta) =
            transaction_gen::generate_transactions_chunk(
                &cards,
                &customer_map,
                &spatial_indices,
                &merchants,
                &config,
            )
            .map_err(|e| format!("Transaction generation error: {}", e))?;

        // Write ground truth
        for m in &meta {
            writeln!(
                gt_file,
                "{},{}",
                m.transaction_id,
                if m.fraud_target { 1 } else { 0 }
            )
            .map_err(|e| format!("GT write error: {}", e))?;
        }
        gt_file
            .flush()
            .map_err(|e| format!("GT flush error: {}", e))?;

        let fraud_in_chunk = meta.iter().filter(|m| m.fraud_target).count() as u64;
        total_fraud += fraud_in_chunk;

        for tx in txs {
            let unlabeled: UnlabeledTransaction = tx.into();
            let payload =
                serde_json::to_string(&unlabeled).map_err(|e| format!("Serialize: {}", e))?;

            let record = FutureRecord::to(&kafka_topic)
                .payload(&payload)
                .key(&unlabeled.transaction_id);

            let send_start = Instant::now();
            let kafka = match producer.send(record, Duration::from_secs(0)).await {
                Ok(_) => KafkaStatus::Connected,
                Err((e, _)) => {
                    let _ = events_tx
                        .send(ProgressEvent::StreamStatus {
                            status: format!("Kafka error: {}", e),
                        })
                        .await;
                    KafkaStatus::Disconnected
                }
            };

            total_sent += 1;

            // Emit tick every 50 records
            if total_sent % 50 == 0 {
                let elapsed = start.elapsed().as_secs_f64();
                let rps = if elapsed > 0.0 {
                    total_sent as f64 / elapsed
                } else {
                    0.0
                };
                let _ = events_tx
                    .send(ProgressEvent::StreamTick {
                        total_sent,
                        total_fraud,
                        records_per_sec: rps,
                        uptime_secs: elapsed as u64,
                        kafka,
                    })
                    .await;
            }

            let elapsed = send_start.elapsed();
            if interval > elapsed {
                tokio::time::sleep(interval - elapsed).await;
            }

            if cancel.is_cancelled() {
                break;
            }
        }

        if cancel.is_cancelled() {
            let _ = events_tx
                .send(ProgressEvent::StreamStatus {
                    status: "Shutting down...".into(),
                })
                .await;
            break;
        }
    }

    gt_file
        .flush()
        .map_err(|e| format!("Final GT flush error: {}", e))?;

    let _ = events_tx
        .send(ProgressEvent::StreamStatus {
            status: format!(
                "Stopped. Total sent: {}, fraud: {}",
                total_sent, total_fraud
            ),
        })
        .await;

    Ok(())
}

fn build_indices_async(path: &str) -> anyhow::Result<(SpatialIndices, MerchantData)> {
    let mut file = std::fs::File::open(path)?;
    let df_merch = ParquetReader::new(&mut file).finish()?;

    let merchants = MerchantData {
        h3_indices: df_merch
            .column("h3_index")?
            .str()?
            .into_no_null_iter()
            .map(|s| s.to_string())
            .collect(),
        names: df_merch
            .column("merchant_name")?
            .str()?
            .into_no_null_iter()
            .map(|s| s.to_string())
            .collect(),
        lats: df_merch.column("latitude")?.f64()?.into_no_null_iter().collect(),
        lons: df_merch.column("longitude")?.f64()?.into_no_null_iter().collect(),
        categories: df_merch
            .column("merchant_category")?
            .str()?
            .into_no_null_iter()
            .map(|s| s.to_string())
            .collect(),
        osm_ids: df_merch.column("osm_id")?.i64()?.into_no_null_iter().collect(),
        states: df_merch
            .column("state")?
            .str()?
            .into_no_null_iter()
            .map(|s| s.to_string())
            .collect(),
    };

    let mut index_res6: HashMap<String, Vec<usize>> = HashMap::new();
    let mut index_res4: HashMap<String, Vec<usize>> = HashMap::new();
    let mut index_state: HashMap<String, Vec<usize>> = HashMap::new();

    for (idx, h3_str) in merchants.h3_indices.iter().enumerate() {
        if let Ok(cell) = CellIndex::from_str(h3_str)
            && let (Some(p6), Some(p4)) =
                (cell.parent(Resolution::Six), cell.parent(Resolution::Four))
        {
            index_res6.entry(p6.to_string()).or_default().push(idx);
            index_res4.entry(p4.to_string()).or_default().push(idx);
        }
        let state = &merchants.states[idx];
        index_state.entry(state.clone()).or_default().push(idx);
    }

    Ok((SpatialIndices { res6: index_res6, res4: index_res4, state: index_state }, merchants))
}

impl crate::pipeline::runner::PipelineRunner {
    pub fn start_stream(
        &self,
        customer_count: usize,
        residential_ref_path: &str,
        merchants_ref_path: &str,
        kafka_bootstrap: &str,
        kafka_topic: &str,
        ground_truth_path: &str,
    ) -> StreamHandle {
        let cancel = CancellationToken::new();
        let (events_tx, events_rx) = mpsc::channel(256);

        let task_cancel = cancel.clone();
        let config = self.config.clone();
        let res_ref = residential_ref_path.to_string();
        let merch_ref = merchants_ref_path.to_string();
        let kafka_bs = kafka_bootstrap.to_string();
        let kafka_topic = kafka_topic.to_string();
        let gt_path = ground_truth_path.to_string();

        let task = tokio::task::spawn(async move {
            let result = stream_loop(
                task_cancel,
                events_tx,
                config,
                customer_count,
                res_ref,
                merch_ref,
                kafka_bs,
                kafka_topic,
                gt_path,
            )
            .await;
            result
        });

        StreamHandle { cancel, task, events_rx: tokio::sync::Mutex::new(events_rx) }
    }
}
