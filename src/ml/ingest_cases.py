import os
import sys
import json
import psycopg2
import clickhouse_connect
import pandas as pd
from datetime import datetime

# Postgres Connection parameters
PG_HOST = os.getenv("OLTP_POSTGRES_HOST", "localhost")
PG_PORT = int(os.getenv("OLTP_POSTGRES_PORT", "5433")) # local default is 5433, inside docker default is 5432
PG_DB = os.getenv("OLTP_POSTGRES_DB", "riskfabric_oltp")
PG_USER = os.getenv("OLTP_POSTGRES_USER", "riskfabric_oltp_user")
PG_PASSWORD = os.getenv("OLTP_POSTGRES_PASSWORD", "123")

# ClickHouse Connection parameters
CH_HOST = os.getenv("CLICKHOUSE_HOST", "localhost")
CH_USER = "riskfabric_user"
CH_PASSWORD = "123"
CH_DB = "riskfabric"

def get_pg_connection():
    return psycopg2.connect(
        host=PG_HOST,
        port=PG_PORT,
        database=PG_DB,
        user=PG_USER,
        password=PG_PASSWORD
    )

def main():
    print("📥 Starting Case Ingestion Script...")
    
    # Step 1: Read flagged transactions
    flagged_txs = []
    
    # Try ClickHouse first
    try:
        print(f"🔎 Connecting to ClickHouse at {CH_HOST}...")
        ch = clickhouse_connect.get_client(
            host=CH_HOST,
            username=CH_USER,
            password=CH_PASSWORD,
            database=CH_DB
        )
        # Check if table exists and has rows
        tables = ch.command("SHOW TABLES")
        if 'fraud_scores' in tables:
            print("   -> Found 'fraud_scores' table. Querying flagged transactions...")
            df = ch.query_df("SELECT transaction_id, fraud_probability as score, timestamp as flagged_at FROM fraud_scores WHERE flagged = 1 LIMIT 100")
            if not df.empty:
                for _, row in df.iterrows():
                    flagged_txs.append({
                        "transaction_id": str(row['transaction_id']),
                        "score": float(row['score']),
                        "flagged_at": pd.Timestamp(row['flagged_at']).to_pydatetime(),
                        "flag_reasons": {"source": "clickhouse_fraud_scores", "rule": "score > 0.85"}
                    })
                print(f"   -> Successfully retrieved {len(flagged_txs)} flagged transactions from ClickHouse.")
    except Exception as e:
        print(f"⚠️ Could not read from ClickHouse: {e}")

    # Fallback to Parquet if ClickHouse has no data or fails
    if not flagged_txs:
        parquet_path = "data/output/transactions.parquet"
        if os.path.exists(parquet_path):
            print(f"📂 Fallback: Reading transactions from Parquet file '{parquet_path}'...")
            try:
                # Read a subset of data using pandas
                df = pd.read_parquet(parquet_path)
                # Find fraud transactions
                fraud_df = df[df['is_fraud'] == True].head(50)
                if fraud_df.empty:
                    # If no fraud, just take some transactions
                    fraud_df = df.head(50)
                
                for _, row in fraud_df.iterrows():
                    # Generate a probability score (since it's ground truth fraud, give it high probability)
                    score = 0.98 if row.get('is_fraud', False) else 0.12
                    # Handle timestamp conversion
                    ts_str = row['timestamp']
                    try:
                        ts = datetime.fromisoformat(ts_str.replace('Z', '+00:00'))
                    except Exception:
                        ts = datetime.now()
                    
                    flagged_txs.append({
                        "transaction_id": str(row['transaction_id']),
                        "score": score,
                        "flagged_at": ts,
                        "flag_reasons": {
                            "source": "parquet_fallback",
                            "amount": float(row['amount']),
                            "channel": str(row['transaction_channel']),
                            "is_fraud_ground_truth": bool(row.get('is_fraud', False))
                        }
                    })
                print(f"   -> Successfully prepared {len(flagged_txs)} transactions from Parquet fallback.")
            except Exception as pe:
                print(f"⚠️ Parquet fallback failed: {pe}")

    if not flagged_txs:
        print("❌ Error: No transactions could be found to ingest into the OLTP cases database.")
        sys.exit(1)

    # Step 2: Write to Postgres cases table
    print(f"🔌 Connecting to OLTP Postgres at {PG_HOST}:{PG_PORT}...")
    try:
        conn = get_pg_connection()
        conn.autocommit = True
        cur = conn.cursor()
        
        print("📥 Inserting transactions into 'cases' table...")
        inserted_count = 0
        skipped_count = 0
        for tx in flagged_txs:
            try:
                cur.execute("""
                    INSERT INTO cases (transaction_id, score, status, flagged_at, flag_reasons)
                    VALUES (%s, %s, 'pending', %s, %s)
                    ON CONFLICT (transaction_id) DO NOTHING;
                """, (
                    tx['transaction_id'],
                    tx['score'],
                    tx['flagged_at'],
                    json.dumps(tx['flag_reasons'])
                ))
                if cur.rowcount > 0:
                    inserted_count += 1
                else:
                    skipped_count += 1
            except Exception as row_err:
                print(f"   -> Failed to insert tx {tx['transaction_id']}: {row_err}")
        
        print(f"✅ Ingestion complete. Inserted: {inserted_count}, Skipped (already exist): {skipped_count}")
        cur.close()
        conn.close()
    except Exception as pg_err:
        print(f"❌ Postgres connection/write error: {pg_err}")
        sys.exit(1)

if __name__ == "__main__":
    main()
