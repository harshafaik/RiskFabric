use polars::prelude::*;
use riskfabric::generators::customer_gen;
use std::fs::File;
use std::time::Instant;

fn main() {
    let count = 100_000;
    println!("🚀 Generating {} customers...", count);
    let start = Instant::now();

    // 1. Generate Data
    let data = customer_gen::generate_bulk(count);
    println!("✅ Generated in {:.2?}", start.elapsed());

    println!("📦 Converting to DataFrame...");

    // 2. Prepare Columns (With Type Casting fixes)
    let customer_ids: Vec<String> = data.iter().map(|c| c.customer_id.clone()).collect();
    let names: Vec<String> = data.iter().map(|c| c.name.clone()).collect();

    // FIX 1: Cast u8 to u32 (Polars likes 32-bit ints better)
    let ages: Vec<u32> = data.iter().map(|c| c.age as u32).collect();

    let emails: Vec<String> = data.iter().map(|c| c.email.clone()).collect();
    let states: Vec<String> = data.iter().map(|c| c.state.clone()).collect();
    let h3s: Vec<String> = data.iter().map(|c| c.home_h3r7.clone()).collect();

    // FIX 2: Cast f32 to f64 (Polars native float is f64)
    let risks: Vec<f64> = data.iter().map(|c| c.customer_risk_score as f64).collect();

    // 3. Create DataFrame
    // Now the types are Vec<u32> and Vec<f64>, which df! supports perfectly.
    let mut df = df!(
        "customer_id" => customer_ids,
        "name" => names,
        "age" => ages,
        "email" => emails,
        "state" => states,
        "h3_index" => h3s,
        "risk_score" => risks
    )
    .expect("Failed to create DataFrame");

    // 4. Save to Parquet
    println!("💾 Saving to disk...");
    let file = File::create("customers.parquet").expect("Could not create file");

    ParquetWriter::new(file)
        .finish(&mut df)
        .expect("Failed to write parquet file");

    println!("🎉 Done! Saved to 'customers.parquet'");
}
