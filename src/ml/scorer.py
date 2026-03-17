import json
import os
import time
import math
import pandas as pd
import xgboost as xgb
import redis
from confluent_kafka import Consumer, KafkaError
import clickhouse_connect
from datetime import datetime

# --- Constants & Config ---
KAFKA_BOOTSTRAP = os.getenv("KAFKA_BOOTSTRAP_SERVERS", "localhost:9092")
REDIS_HOST = os.getenv("REDIS_HOST", "localhost")
CLICKHOUSE_HOST = os.getenv("CLICKHOUSE_HOST", "localhost")
TOPIC = "raw_transactions"
GROUP_ID = "fraud_scorer_v1"
MODEL_PATH = "models/fraud_model_v1.json"
THRESHOLD = 0.85

CATEGORIES = {
    "merchant_category": [
        "HOME_GARDEN", "FOOD_AND_DRINK", "MEDICAL", "GROCERY", "GENERAL_RETAIL",
        "LUXURY", "TRANSPORT", "AUTOMOTIVE", "ELECTRONICS", "RETAIL", "SERVICES",
        "TRAVEL", "ALCOHOL", "ENTERTAINMENT", "B2B_WHOLESALE", "GAMBLING", "CHARITY"
    ],
    "transaction_channel": ["upi", "cards", "online", "mobile_wallets", "mobile_banking"]
}

# --- State Management (Redis) ---
r = redis.Redis(host=REDIS_HOST, port=6379, db=0, decode_responses=True)

class WelfordState:
    """Maintains running mean/std for Z-score calculation."""
    def __init__(self, count=0, mean=0.0, M2=0.0):
        self.count = count
        self.mean = mean
        self.M2 = M2

    def update(self, x):
        self.count += 1
        delta = x - self.mean
        self.mean += delta / self.count
        delta2 = x - self.mean
        self.M2 += delta * delta2

    def get_stats(self):
        variance = self.M2 / self.count if self.count > 1 else 0.0
        return self.mean, math.sqrt(variance)

def get_customer_stats(customer_id):
    stats = r.hgetall(f"cust:{customer_id}:stats")
    if not stats:
        return WelfordState()
    return WelfordState(int(stats['count']), float(stats['mean']), float(stats['M2']))

def save_customer_stats(customer_id, state):
    r.hset(f"cust:{customer_id}:stats", mapping={
        "count": state.count,
        "mean": state.mean,
        "M2": state.M2
    })

# --- Feature Engineering ---
def compute_features(tx, redis_client):
    card_id = tx['card_id']
    customer_id = tx['customer_id']
    merchant_id = tx['merchant_id']
    amount = tx['amount']
    ts_str = tx['timestamp']
    ts = datetime.fromisoformat(ts_str.replace('Z', '+00:00'))
    ts_unix = ts.timestamp()

    # 1. Z-Score (Welford's)
    state = get_customer_stats(customer_id)
    mean, std = state.get_stats()
    z_score = (amount - mean) / std if std > 0 else 0.0
    state.update(amount)
    save_customer_stats(customer_id, state)

    # 2. Time Since Last (per card)
    last_ts = redis_client.get(f"card:{card_id}:last_ts")
    time_since = (ts_unix - float(last_ts)) if last_ts else 0.0
    redis_client.set(f"card:{card_id}:last_ts", ts_unix)

    # 3. Burst Detection (Rapid Fire)
    burst_window = 60 # seconds
    r.zadd(f"card:{card_id}:burst", {tx['transaction_id']: ts_unix})
    r.zremrangebyscore(f"card:{card_id}:burst", 0, ts_unix - burst_window)
    burst_count = r.zcard(f"card:{card_id}:burst")
    rapid_fire = 1 if burst_count > 3 else 0

    # 4. History (Sequence & Category Switch)
    history_key = f"card:{card_id}:history"
    last_tx_raw = r.lindex(history_key, 0)
    prev_tx = json.loads(last_tx_raw) if last_tx_raw else None
    
    cat_switch = 1 if prev_tx and prev_tx['merchant_category'] != tx['merchant_category'] else 0
    escalating = 1 if prev_tx and tx['amount'] > prev_tx['amount'] else 0
    
    r.lpush(history_key, json.dumps(tx))
    r.ltrim(history_key, 0, 9) # Keep last 10
    
    seq_num = r.incr(f"card:{card_id}:seq")

    # 5. Spatial Velocity
    prev_loc = r.hgetall(f"card:{card_id}:loc")
    velocity = 0.0
    if prev_loc and time_since > 0:
        dist = math.sqrt((tx['location_lat'] - float(prev_loc['lat']))**2 + 
                         (tx['location_long'] - float(prev_loc['lon']))**2) * 111.0
        velocity = dist / (time_since / 3600.0)
    r.hset(f"card:{card_id}:loc", mapping={"lat": tx['location_lat'], "lon": tx['location_long']})

    # 6. Customer & Merchant Aggregate Features (from Redis)
    cf_stats = r.hgetall(f"cust:{customer_id}:agg")
    mf_stats = r.hgetall(f"merch:{merchant_id}:agg")

    return {
        "t.merchant_category": tx['merchant_category'], # Match Gold table name
        "transaction_channel": tx['transaction_channel'],
        "card_present": 1 if tx['card_present'] else 0,
        "time_since_last_transaction": time_since,
        "transaction_sequence_number": int(seq_num),
        "rapid_fire_transaction_flag": rapid_fire,
        "escalating_amounts_flag": escalating,
        "merchant_category_switch_flag": cat_switch,
        "amount_deviation_z_score": z_score,
        "spatial_velocity": min(velocity, 1000.0),
        "cf_night_tx_ratio": float(cf_stats.get('night_ratio', 0.0)),
        "hour_deviation_from_norm": 0.0 # Placeholder if not in Redis yet
    }

# --- Main Scorer ---
def main():
    print("🚀 Initializing Real-time Scorer...")
    
    # Load Model
    model = xgb.XGBClassifier(enable_categorical=True)
    model.load_model(MODEL_PATH)
    model_features = model.get_booster().feature_names
    print(f"   -> Model loaded: {MODEL_PATH}")
    print(f"   -> Expected features: {model_features}")

    # ClickHouse Client
    ch = clickhouse_connect.get_client(
        host=CLICKHOUSE_HOST, 
        username='riskfabric_user', 
        password='123',
        database='riskfabric'
    )
    ch.command("""
    CREATE TABLE IF NOT EXISTS fraud_scores (
        transaction_id String,
        card_id String,
        customer_id String,
        amount Float64,
        timestamp DateTime,
        kafka_received_at DateTime64(3),
        fraud_probability Float64,
        flagged UInt8,
        scored_at DateTime64(3)
    ) ENGINE = MergeTree()
    ORDER BY (timestamp, card_id)
    """)

    # Kafka Consumer
    consumer = Consumer({
        'bootstrap.servers': KAFKA_BOOTSTRAP,
        'group.id': GROUP_ID,
        'auto.offset.reset': 'earliest'
    })
    consumer.subscribe([TOPIC])

    print(f"   -> Consuming from Kafka: {TOPIC}")

    batch_size = 50
    records = []

    try:
        while True:
            msg = consumer.poll(1.0)
            if msg is None: continue
            if msg.error():
                print(f"Consumer error: {msg.error()}")
                continue
            
            received_at = datetime.now()
            tx = json.loads(msg.value().decode('utf-8'))
            
            start_feat = time.time()
            features = compute_features(tx, r)
            feat_latency = (time.time() - start_feat) * 1000 # ms
            
            records.append({
                "raw": tx,
                "features": features,
                "feat_ms": feat_latency,
                "received_at": received_at
            })

            if len(records) >= batch_size:
                start_pred = time.time()
                # Prepare batch for prediction
                df = pd.DataFrame([r['features'] for r in records])
                
                # Reorder columns to match model's expected features
                model_features = model.get_booster().feature_names
                model_types = model.get_booster().feature_types
                
                for f in model_features:
                    if f not in df.columns:
                        df[f] = 0.0
                df = df[model_features]

                # Cast features according to model types detected from booster
                for i, f_name in enumerate(model_features):
                    f_type = model_types[i]
                    if f_type == "c":
                        df[f_name] = df[f_name].astype('category')
                    elif f_type == "float":
                        df[f_name] = df[f_name].astype('float32')
                    elif f_type == "int":
                        df[f_name] = df[f_name].astype('int32')

                try:
                    probs = model.predict_proba(df)[:, 1]
                except Exception as e:
                    print(f"   -> Prediction failed: {e}")
                    # Fallback: fill categorical with unknown/NaN and retry?
                    # Or just return 0.0 for this batch
                    probs = [0.0] * len(df)
                
                pred_latency = (time.time() - start_pred) * 1000 # ms
                
                start_sink = time.time()
                # Sink to ClickHouse
                ch_batch = []
                now = datetime.now()
                for i, prob in enumerate(probs):
                    rec = records[i]
                    raw = rec['raw']
                    ch_batch.append((
                        raw['transaction_id'],
                        raw['card_id'],
                        raw['customer_id'],
                        raw['amount'],
                        datetime.fromisoformat(raw['timestamp'].replace('Z', '+00:00')),
                        rec['received_at'],
                        float(prob),
                        1 if prob > THRESHOLD else 0,
                        now
                    ))
                
                ch.insert('fraud_scores', ch_batch, column_names=[
                    'transaction_id', 'card_id', 'customer_id', 'amount', 
                    'timestamp', 'kafka_received_at', 'fraud_probability', 'flagged', 'scored_at'
                ])
                sink_latency = (time.time() - start_sink) * 1000 # ms
                
                avg_feat = sum(r['feat_ms'] for r in records) / len(records)
                print(f"⏱️  Latency: Feats: {avg_feat:.2f}ms/tx | Pred: {pred_latency:.2f}ms/batch | Sink: {sink_latency:.2f}ms")
                print(f"   -> Scored batch of {len(records)}, flagged {sum(1 for r in ch_batch if r[6])}")
                records = []

    except KeyboardInterrupt:
        pass
    finally:
        consumer.close()

if __name__ == "__main__":
    main()
