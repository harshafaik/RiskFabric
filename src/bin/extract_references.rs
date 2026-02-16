use h3o::{LatLng, Resolution};
use osmpbf::{Element, ElementReader};
use postgres::{Client, NoTls};
use postgres::binary_copy::BinaryCopyInWriter;
use postgres::types::Type;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
struct ResidentialPoint {
    osm_id: i64,
    h3_index: String,
    lat: f64,
    lon: f64,
    city: Option<String>,
    postcode: Option<String>,
    state: Option<String>,
}

#[derive(Debug, Clone)]
struct MerchantPoint {
    osm_id: i64,
    h3_index: String,
    name: String,
    category: String,
    sub_category: String,
    lat: f64,
    lon: f64,
}

#[derive(Debug, Clone)]
struct FinancialPoint {
    osm_id: i64,
    h3_index: String,
    kind: String, // "atm" or "bank"
    operator: Option<String>,
    lat: f64,
    lon: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pbf_path = Path::new("data/india-260126.osm.pbf");
    let db_url = "postgres://harshafaik:123@localhost:5432/riskfabric";

    if !pbf_path.exists() {
        eprintln!("File not found: {:?}", pbf_path);
        return Ok(());
    }

    println!("Connecting to database...");
    let mut client = Client::connect(db_url, NoTls)?;
    
    // Create Tables
    println!("Creating/Resetting raw tables...");
    client.batch_execute("
        DROP TABLE IF EXISTS raw_residential;
        CREATE TABLE raw_residential (
            osm_id BIGINT,
            h3_index TEXT,
            lat DOUBLE PRECISION,
            lon DOUBLE PRECISION,
            city TEXT,
            postcode TEXT,
            state TEXT
        );

        DROP TABLE IF EXISTS raw_merchants;
        CREATE TABLE raw_merchants (
            osm_id BIGINT,
            h3_index TEXT,
            name TEXT,
            category TEXT,
            sub_category TEXT,
            lat DOUBLE PRECISION,
            lon DOUBLE PRECISION
        );

        DROP TABLE IF EXISTS raw_financial;
        CREATE TABLE raw_financial (
            osm_id BIGINT,
            h3_index TEXT,
            kind TEXT,
            operator TEXT,
            lat DOUBLE PRECISION,
            lon DOUBLE PRECISION
        );
    ")?;

    println!("Starting OSM extraction from {:?}", pbf_path);

    // Thread-safe storage for our extracted data
    let residential_data = Arc::new(Mutex::new(Vec::new()));
    let merchant_data = Arc::new(Mutex::new(Vec::new()));
    let financial_data = Arc::new(Mutex::new(Vec::new()));

    let res_clone = residential_data.clone();
    let merch_clone = merchant_data.clone();
    let fin_clone = financial_data.clone();

    let reader = ElementReader::from_path(pbf_path)?;

    reader.par_map_reduce(
        move |element| {
            let mut local_res = Vec::new();
            let mut local_merch = Vec::new();
            let mut local_fin = Vec::new();

            match element {
                Element::Node(node) => {
                    process_tags(
                        node.id(),
                        node.lat(),
                        node.lon(),
                        node.tags().collect(),
                        &mut local_res,
                        &mut local_merch,
                        &mut local_fin,
                    );
                }
                Element::DenseNode(node) => {
                     process_tags(
                        node.id(),
                        node.lat(),
                        node.lon(),
                        node.tags().collect(),
                        &mut local_res,
                        &mut local_merch,
                        &mut local_fin,
                    );
                }
                _ => {}
            }

            (local_res, local_merch, local_fin)
        },
        || (Vec::new(), Vec::new(), Vec::new()),
        |mut a, b| {
            a.0.extend(b.0);
            a.1.extend(b.1);
            a.2.extend(b.2);
            a
        },
    ).map(|(r, m, f)| {
        res_clone.lock().unwrap().extend(r);
        merch_clone.lock().unwrap().extend(m);
        fin_clone.lock().unwrap().extend(f);
    })?;

    println!("Extraction complete. Writing to Database...");

    // 1. Write Residential
    let res_points = residential_data.lock().unwrap();
    if !res_points.is_empty() {
        println!("Inserting {} residential points...", res_points.len());
        let sink = client.copy_in("COPY raw_residential (osm_id, h3_index, lat, lon, city, postcode, state) FROM STDIN BINARY")?;
        let mut writer = BinaryCopyInWriter::new(sink, &[Type::INT8, Type::TEXT, Type::FLOAT8, Type::FLOAT8, Type::TEXT, Type::TEXT, Type::TEXT]);
        
        for p in res_points.iter() {
            writer.write(&[&p.osm_id, &p.h3_index, &p.lat, &p.lon, &p.city, &p.postcode, &p.state])?;
        }
        writer.finish()?;
    }

    // 2. Write Merchants
    let merch_points = merchant_data.lock().unwrap();
    if !merch_points.is_empty() {
        println!("Inserting {} merchant points...", merch_points.len());
        let sink = client.copy_in("COPY raw_merchants (osm_id, h3_index, name, category, sub_category, lat, lon) FROM STDIN BINARY")?;
        let mut writer = BinaryCopyInWriter::new(sink, &[Type::INT8, Type::TEXT, Type::TEXT, Type::TEXT, Type::TEXT, Type::FLOAT8, Type::FLOAT8]);
        
        for p in merch_points.iter() {
            writer.write(&[&p.osm_id, &p.h3_index, &p.name, &p.category, &p.sub_category, &p.lat, &p.lon])?;
        }
        writer.finish()?;
    }

    // 3. Write Financial
    let fin_points = financial_data.lock().unwrap();
    if !fin_points.is_empty() {
        println!("Inserting {} financial points...", fin_points.len());
        let sink = client.copy_in("COPY raw_financial (osm_id, h3_index, kind, operator, lat, lon) FROM STDIN BINARY")?;
        let mut writer = BinaryCopyInWriter::new(sink, &[Type::INT8, Type::TEXT, Type::TEXT, Type::TEXT, Type::FLOAT8, Type::FLOAT8]);
        
        for p in fin_points.iter() {
            writer.write(&[&p.osm_id, &p.h3_index, &p.kind, &p.operator, &p.lat, &p.lon])?;
        }
        writer.finish()?;
    }

    println!("All done! Data loaded into Postgres.");
    Ok(())
}

fn process_tags(
    id: i64,
    lat: f64,
    lon: f64,
    tags: HashMap<&str, &str>,
    res_out: &mut Vec<ResidentialPoint>,
    merch_out: &mut Vec<MerchantPoint>,
    fin_out: &mut Vec<FinancialPoint>,
) {
    let coord = match LatLng::new(lat, lon) {
        Ok(c) => c,
        Err(_) => return, 
    };
    let h3 = coord.to_cell(Resolution::Eight).to_string();

    // 1. Financial (ATM/Bank)
    if let Some(amenity) = tags.get("amenity") {
        if *amenity == "atm" || *amenity == "bank" {
            fin_out.push(FinancialPoint {
                osm_id: id,
                h3_index: h3.clone(),
                kind: amenity.to_string(),
                operator: tags.get("operator").or(tags.get("brand")).map(|s| s.to_string()),
                lat,
                lon,
            });
        }
    }

    // 2. Merchants
    let mut is_merchant = false;
    let mut category = String::new();
    let mut sub_category = String::new();

    if let Some(shop) = tags.get("shop") {
        is_merchant = true;
        category = "shop".to_string();
        sub_category = shop.to_string();
    } else if let Some(amenity) = tags.get("amenity") {
        match *amenity {
            "restaurant" | "cafe" | "fast_food" | "bar" | "pub" | "fuel" | "cinema" | "pharmacy" => {
                is_merchant = true;
                category = "amenity".to_string();
                sub_category = amenity.to_string();
            }
            _ => {}
        }
    } else if let Some(tourism) = tags.get("tourism") {
        if *tourism == "hotel" || *tourism == "motel" || *tourism == "guest_house" {
            is_merchant = true;
            category = "tourism".to_string();
            sub_category = tourism.to_string();
        }
    }

    if is_merchant {
        let name = tags.get("name").unwrap_or(&"Unknown Merchant").to_string();
        merch_out.push(MerchantPoint {
            osm_id: id,
            h3_index: h3.clone(),
            name,
            category,
            sub_category,
            lat,
            lon,
        });
    }

    // 3. Residential
    let has_addr = tags.contains_key("addr:housenumber") || tags.contains_key("addr:street");
    let is_residential = tags.get("building") == Some(&"residential") || tags.get("landuse") == Some(&"residential");
    
    if !is_merchant && (is_residential || has_addr) {
        res_out.push(ResidentialPoint {
            osm_id: id,
            h3_index: h3,
            lat,
            lon,
            city: tags.get("addr:city").map(|s| s.to_string()),
            postcode: tags.get("addr:postcode").map(|s| s.to_string()),
            state: tags.get("addr:state").map(|s| s.to_string()),
        });
    }
}