import os
import sys
import uuid
import argparse
import io
import pandas as pd
import numpy as np
import xgboost as xgb
import psycopg2
from datetime import datetime, timezone
from model_utils import load_model

RNG_SEED = 42

# Connect to Postgres
PG_HOST = os.getenv("OLTP_POSTGRES_HOST", "oltp-postgres")
PG_PORT = int(os.getenv("OLTP_POSTGRES_PORT", "5432"))
PG_DB = os.getenv("OLTP_POSTGRES_DB", "riskfabric_oltp")
PG_USER = os.getenv("OLTP_POSTGRES_USER", "riskfabric_oltp_user")
PG_PASSWORD = os.getenv("OLTP_POSTGRES_PASSWORD", "123")


CATEGORIES = [
    "HOME_GARDEN", "FOOD_AND_DRINK", "MEDICAL", "GROCERY", "GENERAL_RETAIL",
    "LUXURY", "TRANSPORT", "AUTOMOTIVE", "ELECTRONICS", "RETAIL", "SERVICES",
    "TRAVEL", "ALCOHOL", "ENTERTAINMENT", "B2B_WHOLESALE", "GAMBLING", "CHARITY"
]
CHANNELS = ["upi", "cards", "online", "mobile_wallets", "mobile_banking"]

def get_pg_connection():
    return psycopg2.connect(
        host=PG_HOST,
        port=PG_PORT,
        database=PG_DB,
        user=PG_USER,
        password=PG_PASSWORD
    )

def main():
    parser = argparse.ArgumentParser(description="Generate 1M unlabeled transactions and score them")
    parser.add_argument("--seed", type=int, default=RNG_SEED, help="Random seed for reproducibility")
    args = parser.parse_args()
    rng = np.random.default_rng(args.seed)

    print("🚀 Loading cards from parquet output...")
    try:
        cards_df = pd.read_parquet("data/output/cards.parquet")
        print(f"   -> Loaded {len(cards_df)} cards definitions.")
    except Exception as e:
        print(f"❌ Error loading cards parquet file: {e}")
        sys.exit(1)

    print("🧠 Loading XGBoost model...")
    model = load_model(enable_categorical=True)
    model_features = model.get_booster().feature_names
    model_types = model.get_booster().feature_types
    print(f"   -> Model loaded. Expected features: {model_features}")

    # Generate 1,000,000 transactions in bulk
    n_records = 1000000
    print(f"🎲 Generating {n_records:,} unlabeled transactions in memory...")
    
    # Sample cards in bulk (gives us customer_id & card_id match)
    sampled_cards = cards_df.sample(n=n_records, replace=True, random_state=args.seed)
    
    # Generate UUIDs
    print("   -> Generating UUIDs...")
    tx_ids = [str(uuid.uuid4()) for _ in range(n_records)]
    
    # Generate other fields
    print("   -> Generating transaction context...")
    amounts_p = rng.choice([1, 2, 3], size=n_records, p=[0.75, 0.20, 0.05])
    amounts = np.zeros(n_records)
    amounts[amounts_p == 1] = rng.uniform(5.0, 100.0, size=np.sum(amounts_p == 1))
    amounts[amounts_p == 2] = rng.uniform(100.0, 1000.0, size=np.sum(amounts_p == 2))
    amounts[amounts_p == 3] = rng.uniform(1000.0, 8000.0, size=np.sum(amounts_p == 3))
    
    channels = rng.choice(CHANNELS, size=n_records)
    card_presents = rng.choice([0, 1], size=n_records)
    categories = rng.choice(CATEGORIES, size=n_records)
    
    # Generate behavioral features directly in numpy
    print("📈 Generating behavioral features...")
    time_since_last = rng.exponential(scale=7200.0, size=n_records) # average 2 hours
    seq_nums = rng.integers(1, 500, size=n_records)
    velocities = rng.exponential(scale=15.0, size=n_records)
    z_scores = rng.normal(loc=0.0, scale=1.5, size=n_records)
    # Inflate Z-scores occasionally to trigger fraud scoring
    high_z_mask = rng.random(n_records) < 0.05
    z_scores[high_z_mask] = rng.uniform(3.0, 8.0, size=np.sum(high_z_mask))
    
    rapid_fires = (time_since_last < 60).astype(int)
    escalating = rng.choice([0, 1], size=n_records, p=[0.6, 0.4])
    cat_switches = rng.choice([0, 1], size=n_records, p=[0.7, 0.3])
    
    # Create features DataFrame
    print("📊 Assembling features DataFrame...")
    features_df = pd.DataFrame({
        "time_since_last_transaction": time_since_last,
        "transaction_sequence_number": seq_nums,
        "spatial_velocity": np.minimum(velocities, 1000.0),
        "hour_deviation_from_norm": 0.0,
        "amount_deviation_z_score": z_scores,
        "rapid_fire_transaction_flag": rapid_fires,
        "escalating_amounts_flag": escalating,
        "merchant_category_switch_flag": cat_switches,
        "transaction_channel": channels,
        "card_present": card_presents,
        "merchant_category": categories,
        "suspicious_cluster_member": 0
    })
    
    # Align and cast columns for XGBoost
    for f in model_features:
        if f not in features_df.columns:
            features_df[f] = 0.0
    features_df = features_df[model_features]

    for i, f_name in enumerate(model_features):
        f_type = model_types[i]
        if f_type == "c":
            features_df[f_name] = features_df[f_name].astype('category')
        elif f_type == "float":
            features_df[f_name] = features_df[f_name].astype('float32')
        elif f_type == "int":
            features_df[f_name] = features_df[f_name].astype('int32')

    # Run bulk predictions
    print("🔮 Running batch predictions against the model...")
    probs = model.predict_proba(features_df)[:, 1]
    print(f"   -> Completed predictions. Average score: {np.mean(probs):.4f}")

    # Prepare data for COPY
    print("📝 Preparing tab-separated format for fast Postgres COPY...")
    
    data_buffer = io.StringIO()
    base_time = datetime.now(timezone.utc)
    
    statuses = np.where(probs > 0.85, 'pending', 'cleared')
    flagged_times = [base_time.isoformat() for _ in range(n_records)]
    
    reasons_list = [
        f'{{"source":"1m_gen","amount":{amt:.2f},"channel":"{chn}","merchant_category":"{cat}"}}'
        for amt, chn, cat in zip(amounts, channels, categories)
    ]
    
    print("   -> Writing formatted records to memory buffer...")
    for idx in range(n_records):
        line = f"{tx_ids[idx]}\t{probs[idx]:.6f}\t{statuses[idx]}\t{flagged_times[idx]}\t{reasons_list[idx]}\n"
        data_buffer.write(line)
        
    data_buffer.seek(0)
    
    # Connect to Postgres
    print(f"🔌 Connecting to OLTP Postgres at {PG_HOST}:{PG_PORT}...")
    try:
        conn = get_pg_connection()
        conn.autocommit = True
        cur = conn.cursor()
        
        print("📥 Copying data into PostgreSQL using COPY protocol...")
        cur.copy_from(
            data_buffer, 
            'cases', 
            columns=('transaction_id', 'score', 'status', 'flagged_at', 'flag_reasons')
        )
        
        cur.execute("SELECT count(*), sum(case when status='pending' then 1 else 0 end) from cases;")
        total, pending = cur.fetchone()
        
        print(f"✅ Ingestion complete! Added {n_records:,} new operational records.")
        print(f"📊 PostgreSQL Cases Summary:")
        print(f"   -> Total cases: {total:,}")
        print(f"   -> Pending (flagged) cases: {pending:,}")
        
        cur.close()
        conn.close()
    except Exception as pg_err:
        print(f"❌ Postgres connection/COPY error: {pg_err}")
        sys.exit(1)

if __name__ == "__main__":
    main()
