import redis
import duckdb
import os
import json
import glob
from argparse import ArgumentParser


REDIS_HOST = os.getenv("REDIS_HOST", "localhost")


def find_gold_snapshot(snapshot=None):
    if snapshot:
        path = f"data/gold/{snapshot}/fact_transactions_gold.parquet"
        if os.path.exists(path):
            return path
        raise FileNotFoundError(f"Snapshot not found: {path}")

    snapshots = sorted(glob.glob("data/gold/*/fact_transactions_gold.parquet"), reverse=True)
    if not snapshots:
        raise FileNotFoundError("No Gold snapshots found. Run `cargo run --bin etl -- gold-master` first.")
    return snapshots[0]


def seed():
    parser = ArgumentParser(description="Seed Redis feature cache from Gold Parquet snapshots")
    parser.add_argument("--snapshot", type=str, default=None, help="Specific snapshot directory")
    args = parser.parse_args()

    gold_path = find_gold_snapshot(args.snapshot)
    print(f"🚀 Seeding Redis from Gold snapshot: {gold_path}")

    r = redis.Redis(host=REDIS_HOST, port=6379, db=0, decode_responses=True)
    conn = duckdb.connect()

    # 1. Customer stats (count, mean, M2 for Welford's algorithm)
    print("   -> Seeding customer stats...")
    result = conn.execute(f"""
        WITH stats AS (
            SELECT customer_id, count() as cnt, avg(amount) as mean
            FROM '{gold_path}'
            GROUP BY customer_id
        )
        SELECT s.customer_id, s.cnt, s.mean,
               sum((t.amount - s.mean) * (t.amount - s.mean)) as M2
        FROM '{gold_path}' t
        JOIN stats s ON t.customer_id = s.customer_id
        GROUP BY s.customer_id, s.cnt, s.mean
    """).fetchall()

    for row in result:
        r.hset(f"cust:{row[0]}:stats", mapping={"count": row[1], "mean": row[2], "M2": row[3] if row[3] else 0.0})

    # 2. Customer aggregate features (fraud_rate, night_ratio)
    print("   -> Seeding customer aggregate features...")
    result = conn.execute(f"""
        SELECT customer_id, cf_fraud_rate, cf_night_tx_ratio
        FROM '{gold_path}'
        GROUP BY customer_id, cf_fraud_rate, cf_night_tx_ratio
    """).fetchall()

    for row in result:
        r.hset(f"cust:{row[0]}:agg", mapping={"fraud_rate": row[1] or 0.0, "night_ratio": row[2] or 0.0})

    # 2b. Customer mean transaction hour (for hour_deviation_from_norm)
    print("   -> Seeding customer mean transaction hour...")
    result = conn.execute(f"""
        SELECT customer_id, avg(hour(timestamp)) as mean_hour
        FROM '{gold_path}'
        GROUP BY customer_id
    """).fetchall()

    for row in result:
        r.hset(f"cust:{row[0]}:agg", "mean_hour", row[1] or 0.0)

    # 3. Merchant aggregate features (fraud_rate)
    print("   -> Seeding merchant aggregate features...")
    result = conn.execute(f"""
        SELECT merchant_id, avg(is_fraud) as fraud_rate
        FROM '{gold_path}'
        GROUP BY merchant_id
    """).fetchall()

    for row in result:
        r.hset(f"merch:{row[0]}:agg", mapping={"fraud_rate": row[1] or 0.0})

    # 4. Card history (last 10 transactions per card)
    print("   -> Seeding card history...")
    result = conn.execute(f"""
        WITH ranked AS (
            SELECT card_id, transaction_id, merchant_category, amount, timestamp,
                   location_lat, location_long,
                   row_number() OVER (PARTITION BY card_id ORDER BY timestamp DESC) as rn
            FROM '{gold_path}'
        )
        SELECT * FROM ranked WHERE rn <= 10
    """).fetchall()

    for row in result:
        card_id = row[0]
        tx_data = {
            "transaction_id": str(row[1]),
            "merchant_category": str(row[2]),
            "amount": float(row[3]),
            "timestamp": str(row[4]),
            "location_lat": float(row[5]) if row[5] else 0.0,
            "location_long": float(row[6]) if row[6] else 0.0,
        }
        r.rpush(f"card:{card_id}:history", json.dumps(tx_data))

        if row[7] == 1:
            r.set(f"card:{card_id}:last_ts", str(row[4]))
            r.hset(f"card:{card_id}:loc", mapping={"lat": float(row[5]) if row[5] else 0.0, "lon": float(row[6]) if row[6] else 0.0})
            r.set(f"card:{card_id}:seq", row[7])

    conn.close()
    print("✅ Redis seeding completed.")


if __name__ == "__main__":
    seed()
