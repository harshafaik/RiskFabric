import os
import sys
import uuid
import random
import json
import pandas as pd
import numpy as np
import redis
import xgboost as xgb
import psycopg2
from datetime import datetime, timezone
from model_utils import load_model

# Connect to Redis
REDIS_HOST = os.getenv("REDIS_HOST", "redis")
r = redis.Redis(host=REDIS_HOST, port=6379, db=0, decode_responses=True)

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
    print("🚀 Loading customers and cards from parquet output...")
    try:
        customers_df = pd.read_parquet("data/output/customers.parquet")
        cards_df = pd.read_parquet("data/output/cards.parquet")
        print(f"   -> Loaded {len(customers_df)} customers and {len(cards_df)} cards.")
    except Exception as e:
        print(f"❌ Error loading data files: {e}")
        sys.exit(1)

    print("🧠 Loading model...")
    model = load_model(enable_categorical=True)
    model_features = model.get_booster().feature_names
    model_types = model.get_booster().feature_types
    print(f"   -> Model loaded. Expected features: {model_features}")

    # Generate 100 new transactions
    print("🎲 Generating 100 unlabeled transactions...")
    txs = []
    for _ in range(100):
        cust_row = customers_df.sample(1).iloc[0]
        customer_id = cust_row['customer_id']
        
        cust_cards = cards_df[cards_df['customer_id'] == customer_id]
        if not cust_cards.empty:
            card_id = cust_cards.sample(1).iloc[0]['card_id']
        else:
            card_id = f"card_{uuid.uuid4().hex[:10]}"
            
        merchant_id = f"merch_{uuid.uuid4().hex[:10]}"
        # Generate some high amount deviations to trigger potential fraud scoring
        amount = random.choices(
            [random.uniform(5.0, 100.0), random.uniform(100.0, 1000.0), random.uniform(1000.0, 8000.0)],
            weights=[0.6, 0.3, 0.1]
        )[0]
        
        tx = {
            "transaction_id": str(uuid.uuid4()),
            "card_id": card_id,
            "customer_id": customer_id,
            "merchant_id": merchant_id,
            "amount": amount,
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "transaction_channel": random.choice(CHANNELS),
            "card_present": random.choice([True, False]),
            "location_lat": float(cust_row['home_latitude']) + random.uniform(-0.1, 0.1),
            "location_long": float(cust_row['home_longitude']) + random.uniform(-0.1, 0.1),
            "merchant_category": random.choice(CATEGORIES)
        }
        txs.append(tx)

    # Compute features for each transaction
    print("📈 Computing behavioral features for model input...")
    features_list = []
    for tx in txs:
        card_id = tx['card_id']
        customer_id = tx['customer_id']
        amount = tx['amount']
        
        # 1. Z-Score
        cust_stats = r.hgetall(f"cust:{customer_id}:stats")
        if cust_stats:
            count = int(cust_stats.get('count', 1))
            mean = float(cust_stats.get('mean', amount))
            M2 = float(cust_stats.get('M2', 0.0))
            variance = M2 / count if count > 1 else 0.0
            std = np.sqrt(variance)
            z_score = (amount - mean) / std if std > 0 else 0.0
        else:
            z_score = 0.0
            
        # 2. Time since last transaction
        last_ts_str = r.get(f"card:{card_id}:last_ts")
        now_ts = datetime.fromisoformat(tx['timestamp']).timestamp()
        time_since = (now_ts - float(last_ts_str)) if last_ts_str else 0.0
        
        # 3. Transaction sequence number
        seq_num = r.incr(f"card:{card_id}:seq")
        
        # 4. Spatial velocity
        prev_loc = r.hgetall(f"card:{card_id}:loc")
        velocity = 0.0
        if prev_loc and time_since > 0:
            dist = np.sqrt((tx['location_lat'] - float(prev_loc['lat']))**2 + 
                           (tx['location_long'] - float(prev_loc['lon']))**2) * 111.0
            velocity = dist / (time_since / 3600.0)
            
        # 5. Category switch
        history_key = f"card:{card_id}:history"
        last_tx_raw = r.lindex(history_key, 0)
        prev_tx = json.loads(last_tx_raw) if last_tx_raw else None
        cat_switch = 1 if prev_tx and prev_tx.get('merchant_category') != tx['merchant_category'] else 0
        
        feat = {
            "time_since_last_transaction": time_since,
            "transaction_sequence_number": int(seq_num),
            "spatial_velocity": min(velocity, 1000.0),
            "hour_deviation_from_norm": 0.0,
            "amount_deviation_z_score": z_score,
            "rapid_fire_transaction_flag": 1 if time_since > 0 and time_since < 60 else 0,
            "escalating_amounts_flag": 1 if prev_tx and amount > prev_tx.get('amount', 0.0) else 0,
            "merchant_category_switch_flag": cat_switch,
            "transaction_channel": tx['transaction_channel'],
            "card_present": 1 if tx['card_present'] else 0,
            "merchant_category": tx['merchant_category'],
            "suspicious_cluster_member": 0
        }
        features_list.append(feat)

    # Convert to DataFrame
    df = pd.DataFrame(features_list)

    # Reorder/align columns
    for f in model_features:
        if f not in df.columns:
            df[f] = 0.0
    df = df[model_features]

    # Cast features according to model types
    for i, f_name in enumerate(model_features):
        f_type = model_types[i]
        if f_type == "c":
            df[f_name] = df[f_name].astype('category')
        elif f_type == "float":
            df[f_name] = df[f_name].astype('float32')
        elif f_type == "int":
            df[f_name] = df[f_name].astype('int32')

    # Run predictions
    print("🔮 Testing transactions against the fraud model...")
    probs = model.predict_proba(df)[:, 1]

    # Write to Postgres OLTP database
    print(f"🔌 Connecting to OLTP Postgres at {PG_HOST}:{PG_PORT}...")
    try:
        conn = get_pg_connection()
        conn.autocommit = True
        cur = conn.cursor()
        
        inserted_count = 0
        flagged_count = 0
        for i, tx in enumerate(txs):
            score = float(probs[i])
            is_suspicious = score > 0.5
            status = 'pending' if is_suspicious else 'cleared'
            
            flag_reasons = {
                "source": "on_demand_simulation",
                "amount": tx['amount'],
                "channel": tx['transaction_channel'],
                "merchant_category": tx['merchant_category'],
                "score_threshold_crossed": is_suspicious
            }
            
            cur.execute("""
                INSERT INTO cases (transaction_id, score, status, flagged_at, flag_reasons)
                VALUES (%s, %s, %s, %s, %s)
                ON CONFLICT (transaction_id) DO NOTHING;
            """, (
                tx['transaction_id'],
                score,
                status,
                datetime.now(timezone.utc),
                json.dumps(flag_reasons)
            ))
            if cur.rowcount > 0:
                inserted_count += 1
                if is_suspicious:
                    flagged_count += 1

        print(f"✅ Ingestion complete! Inserted {inserted_count} new operational case records (Flagged/Pending: {flagged_count}).")
        cur.close()
        conn.close()
    except Exception as pg_err:
        print(f"❌ Postgres connection/write error: {pg_err}")
        sys.exit(1)

if __name__ == "__main__":
    main()
