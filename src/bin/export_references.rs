use polars::prelude::*;
use postgres::{Client, NoTls};
use std::fs::File;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Client::connect("postgres://harshafaik:123@localhost:5432/riskfabric", NoTls)?;
    let output_dir = Path::new("data/references");
    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir)?;
    }

    // 1. Export Residential
    println!("Exporting geo_enriched_residential...");
    let mut osm_ids = Vec::new();
    let mut h3_indices = Vec::new();
    let mut lats = Vec::new();
    let mut lons = Vec::new();
    let mut cities = Vec::new();
    let mut postcodes = Vec::new();
    let mut states = Vec::new();

    for row in client.query("SELECT osm_id, h3_index, latitude, longitude, city, postcode, final_state FROM geo_enriched_residential", &[])? {
        osm_ids.push(row.get::<_, Option<i64>>(0));
        h3_indices.push(row.get::<_, Option<&str>>(1).map(|s| s.to_string()));
        lats.push(row.get::<_, Option<f64>>(2));
        lons.push(row.get::<_, Option<f64>>(3));
        cities.push(row.get::<_, Option<&str>>(4).map(|s| s.to_string()));
        postcodes.push(row.get::<_, Option<&str>>(5).map(|s| s.to_string()));
        states.push(row.get::<_, Option<&str>>(6).map(|s| s.to_string()));
    }

    let mut res_df = df!(
        "osm_id" => osm_ids,
        "h3_index" => h3_indices,
        "latitude" => lats,
        "longitude" => lons,
        "city" => cities,
        "postcode" => postcodes,
        "state" => states
    )?;
    let mut file = File::create(output_dir.join("ref_residential.parquet"))?;
    ParquetWriter::new(&mut file).finish(&mut res_df)?;

    // 2. Export Merchants
    println!("Exporting stg_merchants...");
    let mut m_osm_ids = Vec::new();
    let mut m_h3_indices = Vec::new();
    let mut m_names = Vec::new();
    let mut m_lats = Vec::new();
    let mut m_lons = Vec::new();
    let mut m_cats = Vec::new();
    let mut m_risks = Vec::new();

    for row in client.query("SELECT osm_id, h3_index, merchant_name, lat, lon, merchant_category, risk_level FROM stg_merchants", &[])? {
        m_osm_ids.push(row.get::<_, Option<i64>>(0));
        m_h3_indices.push(row.get::<_, Option<&str>>(1).map(|s| s.to_string()));
        m_names.push(row.get::<_, Option<&str>>(2).map(|s| s.to_string()));
        m_lats.push(row.get::<_, Option<f64>>(3));
        m_lons.push(row.get::<_, Option<f64>>(4));
        m_cats.push(row.get::<_, Option<&str>>(5).map(|s| s.to_string()));
        m_risks.push(row.get::<_, Option<&str>>(6).map(|s| s.to_string()));
    }

    let mut merch_df = df!(
        "osm_id" => m_osm_ids,
        "h3_index" => m_h3_indices,
        "merchant_name" => m_names,
        "lat" => m_lats,
        "lon" => m_lons,
        "merchant_category" => m_cats,
        "risk_level" => m_risks
    )?;
    let mut file = File::create(output_dir.join("ref_merchants.parquet"))?;
    ParquetWriter::new(&mut file).finish(&mut merch_df)?;

    // 3. Export Financial
    println!("Exporting raw_financial...");
    let mut f_osm_ids = Vec::new();
    let mut f_h3_indices = Vec::new();
    let mut f_kinds = Vec::new();
    let mut f_operators = Vec::new();
    let mut f_lats = Vec::new();
    let mut f_lons = Vec::new();
    for row in client.query("SELECT osm_id, h3_index, kind, operator, lat, lon FROM raw_financial", &[])? {
        f_osm_ids.push(row.get::<_, Option<i64>>(0));
        f_h3_indices.push(row.get::<_, Option<&str>>(1).map(|s| s.to_string()));
        f_kinds.push(row.get::<_, Option<&str>>(2).map(|s| s.to_string()));
        f_operators.push(row.get::<_, Option<&str>>(3).map(|s| s.to_string()));
        f_lats.push(row.get::<_, Option<f64>>(4));
        f_lons.push(row.get::<_, Option<f64>>(5));
    }
    let mut fin_df = df!(
        "osm_id" => f_osm_ids,
        "h3_index" => f_h3_indices,
        "kind" => f_kinds,
        "operator" => f_operators,
        "lat" => f_lats,
        "lon" => f_lons
    )?;
    let mut file = File::create(output_dir.join("ref_financial.parquet"))?;
    ParquetWriter::new(&mut file).finish(&mut fin_df)?;

    println!("✨ All reference data exported to Parquet successfully.");
    Ok(())
}
