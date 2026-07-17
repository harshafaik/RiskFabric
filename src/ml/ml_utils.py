import os
import glob
import duckdb
import polars as pl


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


def load_gold_dataframe(snapshot=None):
    gold_path = find_gold_snapshot(snapshot)
    conn = duckdb.connect()
    df = pl.from_arrow(conn.execute(f"SELECT * FROM '{gold_path}'").arrow())
    conn.close()
    return df
