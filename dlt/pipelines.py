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
    source = sql_database("postgresql://harshafaik:123@riskfabric_postgres:5432/riskfabric")
    source.with_resources("mart_residential", "mart_merchants")
    
    info = pipeline.run(source, loader_file_format="parquet", write_disposition="replace")
    print(info)

if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1:
        if sys.argv[1] == "export":
            export_references()
    else:
        print("Usage: python dlt/pipelines.py [export]")
