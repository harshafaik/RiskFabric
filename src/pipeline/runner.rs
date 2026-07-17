use std::collections::HashMap;
use std::fs::{self, File};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crossbeam::channel::Sender;
use h3o::{CellIndex, Resolution};
use polars::prelude::*;

use crate::config::AppConfig;
use crate::generators::{
    account_gen, card_gen, customer_gen, transaction_gen, MerchantData, SpatialIndices,
};
use crate::models::customer::Customer;
use crate::pipeline::events::{ProgressEvent, Stage};

pub struct GenerateOutput {
    pub customers: Vec<Customer>,
    pub accounts: Vec<crate::models::account::Account>,
    pub cards: Vec<crate::models::card::Card>,
    pub transactions: Vec<crate::models::transaction::Transaction>,
    pub fraud_metadata: Vec<crate::models::fraud_metadata::FraudMetadata>,
}

pub struct PipelineRunner {
    pub(crate) config: AppConfig,
    pub(crate) progress_tx: Option<Sender<ProgressEvent>>,
}

struct ProgressCounters {
    records: Arc<AtomicU64>,
    fraud: Arc<AtomicU64>,
}

impl ProgressCounters {
    fn new() -> Self {
        ProgressCounters {
            records: Arc::new(AtomicU64::new(0)),
            fraud: Arc::new(AtomicU64::new(0)),
        }
    }

    fn inc_records(&self, n: u64) {
        self.records.fetch_add(n, Ordering::Relaxed);
    }

    fn inc_fraud(&self, n: u64) {
        self.fraud.fetch_add(n, Ordering::Relaxed);
    }

    fn get(&self) -> (u64, u64) {
        (self.records.load(Ordering::Relaxed), self.fraud.load(Ordering::Relaxed))
    }
}

fn spawn_ticker(
    tx: Sender<ProgressEvent>,
    counters: ProgressCounters,
    stage: Stage,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let (records, fraud) = counters.get();
        if tx.send(ProgressEvent::StageProgress { stage, records_generated: records, fraud_count: fraud }).is_err() {
            break;
        }
    })
}

impl PipelineRunner {
    pub fn new(config: AppConfig) -> Self {
        PipelineRunner { config, progress_tx: None }
    }

    pub fn with_progress(mut self, tx: Sender<ProgressEvent>) -> Self {
        self.progress_tx = Some(tx);
        self
    }

    fn send(&self, event: ProgressEvent) {
        if let Some(ref tx) = self.progress_tx {
            let _ = tx.send(event);
        }
    }

    pub fn run_batch(
        &self,
        customer_count: usize,
        residential_ref_path: &str,
        merchants_ref_path: &str,
    ) -> anyhow::Result<GenerateOutput> {
        let total_start = Instant::now();

        // Stage: Customers
        let stage = Stage::Customers;
        self.send(ProgressEvent::StageStarted { stage, total_expected: customer_count as u64 });
        let counters = ProgressCounters::new();
        let ticker = if self.progress_tx.is_some() {
            Some(spawn_ticker(self.progress_tx.clone().unwrap(), ProgressCounters { records: counters.records.clone(), fraud: counters.fraud.clone() }, stage))
        } else {
            None
        };

        let stage_start = Instant::now();
        let seed = self.config.rules.global.seed as u64;
        let customers = customer_gen::generate_customers(customer_count, seed);
        let fraud_customers = customers.iter().filter(|c| c.financial.is_fraud).count() as u64;
        counters.inc_records(customers.len() as u64);
        counters.inc_fraud(fraud_customers);

        drop(ticker); // stops ticker thread
        self.send(ProgressEvent::StageCompleted {
            stage,
            records_generated: customers.len() as u64,
            fraud_count: fraud_customers,
            elapsed_ms: stage_start.elapsed().as_millis() as u64,
        });

        // Stage: Accounts
        let stage = Stage::Accounts;
        let customer_ids: Vec<String> = customers.iter().map(|c| c.customer_id.clone()).collect();
        self.send(ProgressEvent::StageStarted { stage, total_expected: customer_ids.len() as u64 });
        let counters = ProgressCounters::new();
        let ticker = if self.progress_tx.is_some() {
            Some(spawn_ticker(self.progress_tx.clone().unwrap(), ProgressCounters { records: counters.records.clone(), fraud: counters.fraud.clone() }, stage))
        } else { None };

        let stage_start = Instant::now();
        let accounts = account_gen::generate_accounts(customer_ids, seed);
        counters.inc_records(accounts.len() as u64);

        drop(ticker);
        self.send(ProgressEvent::StageCompleted {
            stage,
            records_generated: accounts.len() as u64,
            fraud_count: 0,
            elapsed_ms: stage_start.elapsed().as_millis() as u64,
        });

        // Stage: Cards
        let stage = Stage::Cards;
        self.send(ProgressEvent::StageStarted { stage, total_expected: accounts.len() as u64 });
        let counters = ProgressCounters::new();
        let ticker = if self.progress_tx.is_some() {
            Some(spawn_ticker(self.progress_tx.clone().unwrap(), ProgressCounters { records: counters.records.clone(), fraud: counters.fraud.clone() }, stage))
        } else { None };

        let stage_start = Instant::now();
        let cards = card_gen::generate_for_accounts(&accounts, seed);
        counters.inc_records(cards.len() as u64);

        drop(ticker);
        self.send(ProgressEvent::StageCompleted {
            stage,
            records_generated: cards.len() as u64,
            fraud_count: 0,
            elapsed_ms: stage_start.elapsed().as_millis() as u64,
        });

        // Stage: Spatial Index
        let stage = Stage::SpatialIndex;
        self.send(ProgressEvent::StageStarted { stage, total_expected: 0 });
        let stage_start = Instant::now();

        let (spatial_indices, merchants) = self.build_spatial_indices(merchants_ref_path)?;

        self.send(ProgressEvent::StageCompleted {
            stage,
            records_generated: 0,
            fraud_count: 0,
            elapsed_ms: stage_start.elapsed().as_millis() as u64,
        });

        // Stage: Transactions (chunked)
        let stage = Stage::Transactions;
        self.send(ProgressEvent::StageStarted { stage, total_expected: cards.len() as u64 });

        let customer_map: HashMap<String, &Customer> = customers
            .iter()
            .map(|c| (c.customer_id.clone(), c))
            .collect();

        let counters = ProgressCounters::new();
        let ticker = if self.progress_tx.is_some() {
            Some(spawn_ticker(self.progress_tx.clone().unwrap(), ProgressCounters { records: counters.records.clone(), fraud: counters.fraud.clone() }, stage))
        } else { None };

        let stage_start = Instant::now();
        let mut all_txs: Vec<crate::models::transaction::Transaction> = Vec::new();
        let mut all_meta: Vec<crate::models::fraud_metadata::FraudMetadata> = Vec::new();

        for chunk in cards.chunks(5000) {
            let (txs, meta) = transaction_gen::generate_transactions_chunk(
                chunk,
                &customer_map,
                &spatial_indices,
                &merchants,
                &self.config,
            )?;
            let fraud_in_chunk = txs.iter().filter(|t| t.is_fraud).count() as u64;
            counters.inc_records(txs.len() as u64);
            counters.inc_fraud(fraud_in_chunk);
            all_txs.extend(txs);
            all_meta.extend(meta);
        }

        let total_fraud = all_txs.iter().filter(|t| t.is_fraud).count() as u64;
        drop(ticker);
        self.send(ProgressEvent::StageCompleted {
            stage,
            records_generated: all_txs.len() as u64,
            fraud_count: total_fraud,
            elapsed_ms: stage_start.elapsed().as_millis() as u64,
        });

        let total_elapsed = total_start.elapsed().as_millis() as u64;
        self.send(ProgressEvent::BatchComplete {
            customers: customers.len() as u64,
            accounts: accounts.len() as u64,
            cards: cards.len() as u64,
            transactions: all_txs.len() as u64,
            fraud_transactions: total_fraud,
            total_elapsed_ms: total_elapsed,
        });

        Ok(GenerateOutput {
            customers,
            accounts,
            cards,
            transactions: all_txs,
            fraud_metadata: all_meta,
        })
    }

    pub fn run_and_persist(
        &self,
        customer_count: usize,
        residential_ref_path: &str,
        merchants_ref_path: &str,
        output_dir: &str,
    ) -> anyhow::Result<GenerateOutput> {
        let output = self.run_batch(customer_count, residential_ref_path, merchants_ref_path)?;

        // Merge stage
        let stage = Stage::MergeOutput;
        self.send(ProgressEvent::StageStarted { stage, total_expected: 0 });
        let stage_start = Instant::now();

        fs::create_dir_all(output_dir)?;

        // Write customers
        let cust_path = format!("{}/customers.parquet", output_dir);
        write_customers_parquet(&output.customers, &cust_path)?;

        // Write accounts
        let acct_path = format!("{}/accounts.parquet", output_dir);
        write_accounts_parquet(&output.accounts, &acct_path)?;

        // Write cards
        let card_path = format!("{}/cards.parquet", output_dir);
        write_cards_parquet(&output.cards, &card_path)?;

        // Write transactions
        let tx_path = format!("{}/transactions.parquet", output_dir);
        write_transactions_parquet(&output.transactions, &tx_path)?;

        // Write fraud metadata
        let meta_path = format!("{}/fraud_metadata.parquet", output_dir);
        write_fraud_metadata_parquet(&output.fraud_metadata, &meta_path)?;

        self.send(ProgressEvent::StageCompleted {
            stage,
            records_generated: output.transactions.len() as u64,
            fraud_count: output.fraud_metadata.iter().filter(|m| m.fraud_target).count() as u64,
            elapsed_ms: stage_start.elapsed().as_millis() as u64,
        });

        Ok(output)
    }

    fn build_spatial_indices(
        &self,
        merchants_ref_path: &str,
    ) -> anyhow::Result<(SpatialIndices, MerchantData)> {
        let mut file = File::open(merchants_ref_path)?;
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

        Ok((
            SpatialIndices { res6: index_res6, res4: index_res4, state: index_state },
            merchants,
        ))
    }
}

// Parquet write helpers

fn write_customers_parquet(
    customers: &[Customer],
    path: &str,
) -> anyhow::Result<()> {
    let mut df = df!(
        "customer_id" => customers.iter().map(|c| c.customer_id.clone()).collect::<Vec<_>>(),
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
        "primary_ua" => customers.iter().map(|c| c.device.primary_ua.clone()).collect::<Vec<_>>(),
        "secondary_ua" => customers.iter().map(|c| c.device.secondary_ua.clone()).collect::<Vec<_>>(),
        "isp" => customers.iter().map(|c| c.device.isp.clone()).collect::<Vec<_>>(),
        "ip_subnet" => customers.iter().map(|c| c.device.ip_subnet.clone()).collect::<Vec<_>>(),
        "registration_date" => customers.iter().map(|c| c.registration_date.to_string()).collect::<Vec<_>>(),
    )?;

    let mut file = File::create(path)?;
    ParquetWriter::new(&mut file).finish(&mut df)?;
    Ok(())
}

fn write_accounts_parquet(
    accounts: &[crate::models::account::Account],
    path: &str,
) -> anyhow::Result<()> {
    let mut df = df!(
        "account_id" => accounts.iter().map(|a| a.account_id.clone()).collect::<Vec<_>>(),
        "customer_id" => accounts.iter().map(|a| a.customer_id.clone()).collect::<Vec<_>>(),
        "bank_id" => accounts.iter().map(|a| a.bank_id.clone()).collect::<Vec<_>>(),
        "account_no" => accounts.iter().map(|a| a.account_no.clone()).collect::<Vec<_>>(),
        "account_type" => accounts.iter().map(|a| a.account_type.clone()).collect::<Vec<_>>(),
        "balance" => accounts.iter().map(|a| a.balance).collect::<Vec<_>>(),
        "status" => accounts.iter().map(|a| a.account_status.clone()).collect::<Vec<_>>(),
        "creation_date" => accounts.iter().map(|a| a.creation_date.clone()).collect::<Vec<_>>(),
    )?;
    let mut file = File::create(path)?;
    ParquetWriter::new(&mut file).finish(&mut df)?;
    Ok(())
}

fn write_cards_parquet(
    cards: &[crate::models::card::Card],
    path: &str,
) -> anyhow::Result<()> {
    let mut df = df!(
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
        "contactless_limit" => cards.iter().map(|c| c.contactless_limit.clone().unwrap_or_default()).collect::<Vec<_>>(),
        "daily_atm_limit" => cards.iter().map(|c| c.daily_atm_limit.clone().unwrap_or_default()).collect::<Vec<_>>(),
        "online_limit" => cards.iter().map(|c| c.online_limit.clone().unwrap_or_default()).collect::<Vec<_>>(),
        "international_usage" => cards.iter().map(|c| c.international_usage.clone().unwrap_or_default()).collect::<Vec<_>>(),
        "issuing_bank" => cards.iter().map(|c| c.issuing_bank.clone()).collect::<Vec<_>>(),
        "bank_code" => cards.iter().map(|c| c.bank_code.clone()).collect::<Vec<_>>(),
    )?;
    let mut file = File::create(path)?;
    ParquetWriter::new(&mut file).finish(&mut df)?;
    Ok(())
}

fn write_transactions_parquet(
    transactions: &[crate::models::transaction::Transaction],
    path: &str,
) -> anyhow::Result<()> {
    let mut df = df!(
        "transaction_id" => transactions.iter().map(|t| t.transaction_id.clone()).collect::<Vec<_>>(),
        "card_id" => transactions.iter().map(|t| t.card_id.clone()).collect::<Vec<_>>(),
        "account_id" => transactions.iter().map(|t| t.account_id.clone()).collect::<Vec<_>>(),
        "customer_id" => transactions.iter().map(|t| t.customer_id.clone()).collect::<Vec<_>>(),
        "merchant_id" => transactions.iter().map(|t| t.merchant_id.clone()).collect::<Vec<_>>(),
        "merchant_name" => transactions.iter().map(|t| t.merchant_name.clone()).collect::<Vec<_>>(),
        "merchant_category" => transactions.iter().map(|t| t.merchant_category.clone()).collect::<Vec<_>>(),
        "mcc" => transactions.iter().map(|t| t.mcc.clone()).collect::<Vec<_>>(),
        "merchant_country" => transactions.iter().map(|t| t.merchant_country.clone()).collect::<Vec<_>>(),
        "amount" => transactions.iter().map(|t| t.amount).collect::<Vec<_>>(),
        "currency" => transactions.iter().map(|t| t.currency.clone()).collect::<Vec<_>>(),
        "timestamp" => transactions.iter().map(|t| t.timestamp.clone()).collect::<Vec<_>>(),
        "transaction_channel" => transactions.iter().map(|t| t.transaction_channel.clone()).collect::<Vec<_>>(),
        "card_present" => transactions.iter().map(|t| t.card_present).collect::<Vec<_>>(),
        "user_agent" => transactions.iter().map(|t| t.user_agent.clone()).collect::<Vec<_>>(),
        "ip_address" => transactions.iter().map(|t| t.ip_address.clone()).collect::<Vec<_>>(),
        "status" => transactions.iter().map(|t| t.status.clone()).collect::<Vec<_>>(),
        "auth_status" => transactions.iter().map(|t| t.auth_status.clone()).collect::<Vec<_>>(),
        "failure_reason" => transactions.iter().map(|t| t.failure_reason.clone()).collect::<Vec<_>>(),
        "is_fraud" => transactions.iter().map(|t| t.is_fraud).collect::<Vec<_>>(),
        "chargeback" => transactions.iter().map(|t| t.chargeback).collect::<Vec<_>>(),
        "chargeback_days" => transactions.iter().map(|t| t.chargeback_days).collect::<Vec<_>>(),
        "location_lat" => transactions.iter().map(|t| t.location_lat).collect::<Vec<_>>(),
        "location_long" => transactions.iter().map(|t| t.location_long).collect::<Vec<_>>(),
        "h3_r7" => transactions.iter().map(|t| t.h3_r7.clone()).collect::<Vec<_>>(),
    )?;
    let mut file = File::create(path)?;
    ParquetWriter::new(&mut file).finish(&mut df)?;
    Ok(())
}

fn write_fraud_metadata_parquet(
    metadata: &[crate::models::fraud_metadata::FraudMetadata],
    path: &str,
) -> anyhow::Result<()> {
    let mut df = df!(
        "transaction_id" => metadata.iter().map(|m| m.transaction_id.clone()).collect::<Vec<_>>(),
        "fraud_target" => metadata.iter().map(|m| m.fraud_target).collect::<Vec<_>>(),
        "fraud_type" => metadata.iter().map(|m| m.fraud_type.clone()).collect::<Vec<_>>(),
        "label_noise" => metadata.iter().map(|m| m.label_noise.clone()).collect::<Vec<_>>(),
        "injector_version" => metadata.iter().map(|m| m.injector_version.clone()).collect::<Vec<_>>(),
        "geo_anomaly" => metadata.iter().map(|m| m.geo_anomaly).collect::<Vec<_>>(),
        "device_anomaly" => metadata.iter().map(|m| m.device_anomaly).collect::<Vec<_>>(),
        "ip_anomaly" => metadata.iter().map(|m| m.ip_anomaly).collect::<Vec<_>>(),
        "flags" => metadata.iter().map(|m| m.flags.as_ref().map(|f| f.join(","))).collect::<Vec<_>>(),
        "burst_session" => metadata.iter().map(|m| m.burst_session).collect::<Vec<_>>(),
        "burst_seq" => metadata.iter().map(|m| m.burst_seq).collect::<Vec<_>>(),
        "campaign_id" => metadata.iter().map(|m| m.campaign_id.clone()).collect::<Vec<_>>(),
        "campaign_type" => metadata.iter().map(|m| m.campaign_type.clone()).collect::<Vec<_>>(),
        "campaign_phase" => metadata.iter().map(|m| m.campaign_phase.clone()).collect::<Vec<_>>(),
        "campaign_day_number" => metadata.iter().map(|m| m.campaign_day_number).collect::<Vec<_>>(),
    )?;
    let mut file = File::create(path)?;
    ParquetWriter::new(&mut file).finish(&mut df)?;
    Ok(())
}
