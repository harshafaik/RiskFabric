import redis
import clickhouse_connect
import os
import json
from datetime import datetime

REDIS_HOST = os.getenv("REDIS_HOST", "localhost")
CLICKHOUSE_HOST = os.getenv("CLICKHOUSE_HOST", "localhost")

def seed():
    print("🚀 Seeding Redis from ClickHouse Silver layer...")
    r = redis.Redis(host=REDIS_HOST, port=6379, db=0, decode_responses=True)
    ch = clickhouse_connect.get_client(
        host=CLICKHOUSE_HOST, 
        username='riskfabric_user', 
        password='123',
        database='riskfabric'
    )

    # 1. Seed Customer Stats (Mean/M2 for Welford's)
    print("   -> Seeding customer stats...")
    query = """
    SELECT 
        customer_id, 
        count() as count, 
        avg(amount) as mean, 
        sum((amount - mean) * (amount - mean)) as M2
    FROM fact_transactions_gold
    GROUP BY customer_id
    """
    # Note: If fact_transactions_gold doesn't exist, this will fail. 
    # In a real scenario, we'd handle fallback.
    try:
        results = ch.query(query)
        for row in results.result_rows:
            r.hset(f"cust:{row[0]}:stats", mapping={
                "count": row[1],
                "mean": row[2],
                "M2": row[3]
            })
    except Exception as e:
        print(f"      Warning: Could not seed customer stats: {e}")

    # 1.1 Seed Customer Aggregates (fraud_rate, night_ratio)
    print("   -> Seeding customer aggregate features...")
    query = """
    SELECT 
        customer_id, 
        any(fraud_rate) as fraud_rate, 
        any(night_transaction_ratio) as night_ratio
    FROM fact_transactions_gold
    GROUP BY customer_id
    """
    try:
        results = ch.query(query)
        for row in results.result_rows:
            r.hset(f"cust:{row[0]}:agg", mapping={
                "fraud_rate": row[1],
                "night_ratio": row[2]
            })
    except Exception as e:
        print(f"      Warning: Could not seed customer aggregates: {e}")

    # 1.2 Seed Merchant Aggregates (fraud_rate)
    print("   -> Seeding merchant aggregate features...")
    query = """
    SELECT 
        merchant_id, 
        avg(is_fraud) as fraud_rate
    FROM fact_transactions_gold
    GROUP BY merchant_id
    """
    try:
        results = ch.query(query)
        for row in results.result_rows:
            r.hset(f"merch:{row[0]}:agg", mapping={
                "fraud_rate": row[1]
            })
    except Exception as e:
        print(f"      Warning: Could not seed merchant aggregates: {e}")

    # 2. Seed Card History (Last 10 transactions)
    print("   -> Seeding card history...")
    query = """
    SELECT * FROM (
        SELECT 
            card_id, 
            transaction_id, 
            merchant_category, 
            amount, 
            timestamp,
            location_lat,
            location_long,
            row_number() OVER (PARTITION BY card_id ORDER BY timestamp DESC) as rn
        FROM fact_transactions_gold
    ) WHERE rn <= 10
    """
    try:
        results = ch.query(query)
        for row in results.result_rows:
            card_id = row[0]
            tx_data = {
                "transaction_id": row[1],
                "merchant_category": row[2],
                "amount": row[3],
                "timestamp": row[4].isoformat(),
                "location_lat": row[5],
                "location_long": row[6]
            }
            r.rpush(f"card:{card_id}:history", json.dumps(tx_data))
            
            # Seed last location and timestamp for velocity/time_since
            if row[7] == 1: # Latest transaction
                r.set(f"card:{card_id}:last_ts", row[4].timestamp())
                r.hset(f"card:{card_id}:loc", mapping={"lat": row[5], "lon": row[6]})
                r.set(f"card:{card_id}:seq", row[7]) # This is approximate

    except Exception as e:
        print(f"      Warning: Could not seed card history: {e}")

    print("✅ Redis seeding completed.")

if __name__ == "__main__":
    seed()
