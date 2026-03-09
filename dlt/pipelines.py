import dlt
from dlt.sources.sql_database import sql_database
import os

def export_references():
    """Reads enriched data from Postgres and exports to Parquet for the generator."""
    pipeline = dlt.pipeline(
        pipeline_name="postgres_to_parquet",
        destination="filesystem",
        dataset_name="references"
    )

    # Note: We configure the 'filesystem' destination to write directly 
    # to data/references in Parquet format.
    os.environ["DESTINATION__FILESYSTEM__BUCKET_URL"] = "file://./data/references"
    os.environ["DESTINATION__FILESYSTEM__LAYOUT"] = "{table_name}.parquet"
    
    # Source from our new 'mart_' tables
    source = sql_database("postgresql://harshafaik:123@postgres:5432/riskfabric")
    source.with_resources("mart_residential", "mart_merchants")
    
    info = pipeline.run(source, loader_file_format="parquet", write_disposition="replace")
    print(info)

def ingest_to_clickhouse():
    """Ingests generated Parquet files from data/output into ClickHouse."""
    pipeline = dlt.pipeline(
        pipeline_name="parquet_to_clickhouse",
        destination="clickhouse",
        dataset_name="riskfabric"
    )

    # dlt will automatically discover schemas from the Parquet files
    # and create corresponding tables in ClickHouse.
    def parquet_source():
        import glob
        for file in glob.glob("data/output/*.parquet"):
            table_name = os.path.basename(file).replace(".parquet", "")
            yield dlt.resource(file, name=f"bronze_{table_name}")

    info = pipeline.run(
        parquet_source(),
        credentials="clickhouse://default:@clickhouse:8123/riskfabric",
        write_disposition="replace"
    )
    print(info)

if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1:
        if sys.argv[1] == "export":
            export_references()
        elif sys.argv[1] == "ingest":
            ingest_to_clickhouse()
    else:
        print("Usage: python dlt/pipelines.py [export|ingest]")
