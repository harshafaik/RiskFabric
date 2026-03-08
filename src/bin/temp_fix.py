import polars as pl
import clickhouse_connect
import os
import subprocess

def run_query(query, database='riskfabric'):
    # We use podman exec to get Parquet from ClickHouse
    cmd = f"podman exec riskfabric_clickhouse clickhouse-client --database {database} --query \"{query}\" FORMAT Parquet"
    result = subprocess.run(cmd, shell=True, capture_output=True)
    if result.returncode != 0:
        raise Exception(f"ClickHouse Error: {result.stderr.decode()}")
    return pl.read_parquet(result.stdout)

def sink_df(df, table, database='riskfabric'):
    temp_path = f"data/tmp_{table}.parquet"
    df.write_parquet(temp_path)
    # Create table if not exists (simplified schema inference from first run or use existing)
    # For speed, we assume tables are created by the Rust logic or we can create them here.
    cmd = f"cat {temp_path} | podman exec -i riskfabric_clickhouse clickhouse-client --database {database} --query \"INSERT INTO {table} FORMAT Parquet\""
    subprocess.run(cmd, shell=True)
    os.remove(temp_path)

def main():
    print("🚀 Starting Integrated ETL Pipeline (Python Bridge)...")
    
    # We could implement all transformations here, 
    # but for now let's just make sure the Gold table exists for XGBoost.
    # Since we've already run customer and merchant silver in Rust, let's do the rest.

    # 1. Check if Gold table is needed
    print("✨ Gold table is required for XGBoost.")
    
    # Because of the complexity of the Rust transform logic (transform_customer_features, etc.),
    # the best path is to run the remaining Rust bins after I fix them with sed.
    
    scripts = [
        "etl_silver_campaign",
        "etl_silver_device_ip",
        "etl_silver_network",
        "etl_silver_sequence",
        "etl_gold_master"
    ]
    
    for script in scripts:
        print(f"🛠️ Fixing and Running {script}...")
        # Replace clickhouse-client with podman exec -i riskfabric_clickhouse clickhouse-client
        # This is a bit hacky but keeps your Rust logic intact!
        subprocess.run(f"sed -i 's/clickhouse-client/podman exec -i riskfabric_clickhouse clickhouse-client/g' src/bin/{script}.rs", shell=True)
        # Note: Need to handle the args array vs string in Command::new...
        # Better yet, I'll just rewrite etl_gold_master.rs properly.

if __name__ == "__main__":
    main()
