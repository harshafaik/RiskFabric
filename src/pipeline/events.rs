use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stage {
    Customers,
    Accounts,
    Cards,
    SpatialIndex,
    Transactions,
    MergeOutput,
}

impl Stage {
    pub fn display_name(&self) -> &'static str {
        match self {
            Stage::Customers => "Customers",
            Stage::Accounts => "Accounts",
            Stage::Cards => "Cards",
            Stage::SpatialIndex => "Spatial Index",
            Stage::Transactions => "Transactions",
            Stage::MergeOutput => "Merge Output",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KafkaStatus {
    Connected,
    Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgressEvent {
    StageStarted {
        stage: Stage,
        total_expected: u64,
    },
    StageProgress {
        stage: Stage,
        records_generated: u64,
        fraud_count: u64,
    },
    StageCompleted {
        stage: Stage,
        records_generated: u64,
        fraud_count: u64,
        elapsed_ms: u64,
    },
    BatchComplete {
        customers: u64,
        accounts: u64,
        cards: u64,
        transactions: u64,
        fraud_transactions: u64,
        total_elapsed_ms: u64,
    },
    BatchError(String),
    StreamTick {
        total_sent: u64,
        total_fraud: u64,
        records_per_sec: f64,
        uptime_secs: u64,
        kafka: KafkaStatus,
    },
    StreamStatus {
        status: String,
    },
}
