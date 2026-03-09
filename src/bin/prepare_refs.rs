use clap::{Parser, Subcommand};
use h3o::{LatLng, Resolution};
use osmpbf::{Element, ElementReader};
use postgres::binary_copy::BinaryCopyInWriter;
use postgres::types::Type;
use postgres::{Client, NoTls};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Parser)]
#[command(name = "riskfabric-prepare-refs")]
#[command(about = "Unified OSM processing and reference data tool for RiskFabric", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract nodes (residential, merchants, financial) from OSM PBF into Postgres
    ExtractNodes {
        #[arg(short, long, default_value = "data/raw/india-260126.osm.pbf")]
        pbf: String,
        #[arg(
            short,
            long,
            default_value = "postgres://harshafaik:123@localhost:5432/riskfabric"
        )]
        db: String,
    },
    /// Map Cities to States based on Node addresses
    MapCityState {
        #[arg(short, long, default_value = "data/india-260126.osm.pbf")]
        pbf: String,
    },
    /// Map Districts to States using ISO Codes
    ParseDistricts {
        #[arg(short, long, default_value = "data/india-260126.osm.pbf")]
        pbf: String,
    },
    /// Map Member IDs to States (Relation analysis)
    MapStateDistricts {
        #[arg(short, long, default_value = "data/india-260126.osm.pbf")]
        pbf: String,
    },
    /// Normalize State Names in the city-state report
    NormalizeStates,
    /// Compare Cities and Districts lists
    CompareCityDistrict,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::ExtractNodes { pbf, db } => run_extract_nodes(&pbf, &db)?,
        Commands::MapCityState { pbf } => run_map_city_state(&pbf)?,
        Commands::ParseDistricts { pbf } => run_parse_districts(&pbf)?,
        Commands::MapStateDistricts { pbf } => run_map_state_districts(&pbf)?,
        Commands::NormalizeStates => run_normalize_states()?,
        Commands::CompareCityDistrict => run_compare_city_district()?,
    }

    Ok(())
}

// --- LOGIC FROM extract_references.rs ---

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
    city: Option<String>,
    postcode: Option<String>,
    state: Option<String>,
}

#[derive(Debug, Clone)]
struct FinancialPoint {
    osm_id: i64,
    h3_index: String,
    kind: String, // "atm" or "bank"
    operator: Option<String>,
    lat: f64,
    lon: f64,
    city: Option<String>,
    postcode: Option<String>,
    state: Option<String>,
}

fn run_extract_nodes(pbf_path_str: &str, db_url: &str) -> Result<(), Box<dyn Error>> {
    let pbf_path = Path::new(pbf_path_str);
    if !pbf_path.exists() {
        return Err(format!("File not found: {:?}", pbf_path).into());
    }

    println!("Connecting to database...");
    let mut client = Client::connect(db_url, NoTls)?;

    println!("Creating/Resetting raw tables...");
    client.batch_execute(
        "
        DROP TABLE IF EXISTS raw_residential CASCADE;
        CREATE TABLE raw_residential (
            osm_id BIGINT,
            h3_index TEXT,
            lat DOUBLE PRECISION,
            lon DOUBLE PRECISION,
            city TEXT,
            postcode TEXT,
            state TEXT
        );

        DROP TABLE IF EXISTS raw_merchants CASCADE;
        CREATE TABLE raw_merchants (
            osm_id BIGINT,
            h3_index TEXT,
            name TEXT,
            category TEXT,
            sub_category TEXT,
            lat DOUBLE PRECISION,
            lon DOUBLE PRECISION,
            city TEXT,
            postcode TEXT,
            state TEXT
        );

        DROP TABLE IF EXISTS raw_financial CASCADE;
        CREATE TABLE raw_financial (
            osm_id BIGINT,
            h3_index TEXT,
            kind TEXT,
            operator TEXT,
            lat DOUBLE PRECISION,
            lon DOUBLE PRECISION,
            city TEXT,
            postcode TEXT,
            state TEXT
        );
    ",
    )?;

    println!("Starting OSM extraction from {:?}", pbf_path);

    let residential_data = Arc::new(Mutex::new(Vec::new()));
    let merchant_data = Arc::new(Mutex::new(Vec::new()));
    let financial_data = Arc::new(Mutex::new(Vec::new()));

    let res_clone = residential_data.clone();
    let merch_clone = merchant_data.clone();
    let fin_clone = financial_data.clone();

    let reader = ElementReader::from_path(pbf_path)?;

    reader
        .par_map_reduce(
            move |element| {
                let mut local_res = Vec::new();
                let mut local_merch = Vec::new();
                let mut local_fin = Vec::new();

                match element {
                    Element::Node(node) => {
                        process_tags_extract(
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
                        process_tags_extract(
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
        )
        .map(|(r, m, f)| {
            res_clone.lock().unwrap().extend(r);
            merch_clone.lock().unwrap().extend(m);
            fin_clone.lock().unwrap().extend(f);
        })?;

    println!("Extraction complete. Writing to Database...");

    let res_points = residential_data.lock().unwrap();
    if !res_points.is_empty() {
        println!("Inserting {} residential points...", res_points.len());
        let sink = client.copy_in("COPY raw_residential (osm_id, h3_index, lat, lon, city, postcode, state) FROM STDIN BINARY")?;
        let mut writer = BinaryCopyInWriter::new(
            sink,
            &[
                Type::INT8,
                Type::TEXT,
                Type::FLOAT8,
                Type::FLOAT8,
                Type::TEXT,
                Type::TEXT,
                Type::TEXT,
            ],
        );
        for p in res_points.iter() {
            writer.write(&[
                &p.osm_id,
                &p.h3_index,
                &p.lat,
                &p.lon,
                &p.city,
                &p.postcode,
                &p.state,
            ])?;
        }
        writer.finish()?;
    }

    let merch_points = merchant_data.lock().unwrap();
    if !merch_points.is_empty() {
        println!("Inserting {} merchant points...", merch_points.len());
        let sink = client.copy_in("COPY raw_merchants (osm_id, h3_index, name, category, sub_category, lat, lon, city, postcode, state) FROM STDIN BINARY")?;
        let mut writer = BinaryCopyInWriter::new(
            sink,
            &[
                Type::INT8,
                Type::TEXT,
                Type::TEXT,
                Type::TEXT,
                Type::TEXT,
                Type::FLOAT8,
                Type::FLOAT8,
                Type::TEXT,
                Type::TEXT,
                Type::TEXT,
            ],
        );
        for p in merch_points.iter() {
            writer.write(&[
                &p.osm_id,
                &p.h3_index,
                &p.name,
                &p.category,
                &p.sub_category,
                &p.lat,
                &p.lon,
                &p.city,
                &p.postcode,
                &p.state,
            ])?;
        }
        writer.finish()?;
    }

    let fin_points = financial_data.lock().unwrap();
    if !fin_points.is_empty() {
        println!("Inserting {} financial points...", fin_points.len());
        let sink = client.copy_in(
            "COPY raw_financial (osm_id, h3_index, kind, operator, lat, lon, city, postcode, state) FROM STDIN BINARY",
        )?;
        let mut writer = BinaryCopyInWriter::new(
            sink,
            &[
                Type::INT8,
                Type::TEXT,
                Type::TEXT,
                Type::TEXT,
                Type::FLOAT8,
                Type::FLOAT8,
                Type::TEXT,
                Type::TEXT,
                Type::TEXT,
            ],
        );
        for p in fin_points.iter() {
            writer.write(&[
                &p.osm_id,
                &p.h3_index,
                &p.kind,
                &p.operator,
                &p.lat,
                &p.lon,
                &p.city,
                &p.postcode,
                &p.state,
            ])?;
        }
        writer.finish()?;
    }

    println!("All done! Data loaded into Postgres.");
    Ok(())
}

fn process_tags_extract(
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

    if let Some(amenity) = tags.get("amenity")
        && (*amenity == "atm" || *amenity == "bank")
    {
        fin_out.push(FinancialPoint {
            osm_id: id,
            h3_index: h3.clone(),
            kind: amenity.to_string(),
            operator: tags
                .get("operator")
                .or(tags.get("brand"))
                .map(|s| s.to_string()),
            lat,
            lon,
            city: tags.get("addr:city").map(|s| s.to_string()),
            postcode: tags.get("addr:postcode").map(|s| s.to_string()),
            state: tags.get("addr:state").map(|s| s.to_string()),
        });
    }

    let mut is_merchant = false;
    let mut category = String::new();
    let mut sub_category = String::new();

    if let Some(shop) = tags.get("shop") {
        is_merchant = true;
        category = "shop".to_string();
        sub_category = shop.to_string();
    } else if let Some(amenity) = tags.get("amenity") {
        match *amenity {
            "restaurant" | "cafe" | "fast_food" | "bar" | "pub" | "fuel" | "cinema"
            | "pharmacy" => {
                is_merchant = true;
                category = "amenity".to_string();
                sub_category = amenity.to_string();
            }
            _ => {}
        }
    } else if let Some(tourism) = tags.get("tourism")
        && (*tourism == "hotel" || *tourism == "motel" || *tourism == "guest_house")
    {
        is_merchant = true;
        category = "tourism".to_string();
        sub_category = tourism.to_string();
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
            city: tags.get("addr:city").map(|s| s.to_string()),
            postcode: tags.get("addr:postcode").map(|s| s.to_string()),
            state: tags.get("addr:state").map(|s| s.to_string()),
        });
    }

    let has_addr = tags.contains_key("addr:housenumber") || tags.contains_key("addr:street");
    let is_residential =
        tags.get("building") == Some(&"residential") || tags.get("landuse") == Some(&"residential");

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

// --- LOGIC FROM map_city_state.rs ---

fn run_map_city_state(pbf_path_str: &str) -> Result<(), Box<dyn Error>> {
    let path = Path::new(pbf_path_str);
    println!("Mapping Cities to States based on Node addresses...");
    let reader = ElementReader::from_path(path)?;

    let state_city_counts: HashMap<String, HashMap<String, usize>> = reader.par_map_reduce(
        |element| {
            let mut local_counts: HashMap<String, HashMap<String, usize>> = HashMap::new();
            let mut process_tags = |tags: HashMap<&str, &str>| {
                if let (Some(city), Some(state)) = (tags.get("addr:city"), tags.get("addr:state")) {
                    let clean_city = city.trim().to_string();
                    let clean_state = state.trim().to_string();
                    if !clean_city.is_empty() && !clean_state.is_empty() {
                        *local_counts
                            .entry(clean_state)
                            .or_default()
                            .entry(clean_city)
                            .or_insert(0) += 1;
                    }
                }
            };
            match element {
                Element::Node(node) => process_tags(node.tags().collect()),
                Element::DenseNode(node) => process_tags(node.tags().collect()),
                _ => {}
            }
            local_counts
        },
        HashMap::new,
        |mut a, b| {
            for (state, cities) in b {
                let state_entry = a.entry(state).or_default();
                for (city, count) in cities {
                    *state_entry.entry(city).or_insert(0) += count;
                }
            }
            a
        },
    )?;

    let mut file = File::create("data/city_state_report.txt")?;
    let mut sorted_states: Vec<_> = state_city_counts.keys().collect();
    sorted_states.sort();

    for state in sorted_states {
        if let Some(cities) = state_city_counts.get(state) {
            let total_nodes: usize = cities.values().sum();
            let total_unique_cities = cities.len();
            writeln!(
                file,
                "State: {} ({} unique cities, {} total nodes)",
                state, total_unique_cities, total_nodes
            )?;
            let mut sorted_cities: Vec<_> = cities.iter().collect();
            sorted_cities.sort_by(|a, b| b.1.cmp(a.1));
            for (city, count) in sorted_cities {
                writeln!(file, "  - {}: {}", city, count)?;
            }
            writeln!(file)?;
        }
    }
    println!("Detailed report saved to data/city_state_report.txt");
    Ok(())
}

// --- LOGIC FROM parse_osm.rs ---

fn run_parse_districts(pbf_path_str: &str) -> Result<(), Box<dyn Error>> {
    let path = Path::new(pbf_path_str);
    println!("Mapping Districts to States (Level 5 Relations) using ISO Codes...");
    let reader = ElementReader::from_path(path)?;
    let state_district_map = Arc::new(Mutex::new(HashMap::<String, HashSet<String>>::new()));
    let map_clone = state_district_map.clone();

    reader.par_map_reduce(
        move |element| {
            if let Element::Relation(relation) = element {
                let tags: HashMap<&str, &str> = relation.tags().collect();
                let is_admin = tags.get("boundary") == Some(&"administrative");
                let is_level_5 = tags.get("admin_level") == Some(&"5");
                let is_india = tags
                    .get("ISO3166-2")
                    .map(|v| v.starts_with("IN-"))
                    .unwrap_or(false)
                    || tags.get("is_in:country_code") == Some(&"IN")
                    || tags.get("addr:country") == Some(&"IN")
                    || tags.iter().any(|(k, _)| k.starts_with("ref:LGD"))
                    || tags.iter().any(|(k, _)| k.starts_with("mdds:"))
                    || tags.contains_key("is_in:state");

                if is_admin && is_level_5 && is_india {
                    let district_name = tags
                        .get("name")
                        .or_else(|| tags.get("name:en"))
                        .unwrap_or(&"Unnamed District")
                        .to_string();
                    let state_key = if let Some(state) = tags.get("is_in:state") {
                        state.to_string()
                    } else if let Some(state) = tags.get("addr:state") {
                        state.to_string()
                    } else if let Some(iso) = tags.get("ISO3166-2") {
                        let parts: Vec<&str> = iso.split('-').collect();
                        if parts.len() >= 2 {
                            format!("{}-{}", parts[0], parts[1])
                        } else {
                            "Unknown State".to_string()
                        }
                    } else {
                        "Unknown State".to_string()
                    };
                    map_clone
                        .lock()
                        .unwrap()
                        .entry(state_key)
                        .or_default()
                        .insert(district_name);
                }
            }
        },
        || (),
        |_, _| (),
    )?;

    let final_map = state_district_map.lock().unwrap();
    let mut file = File::create("data/state_district_map.txt")?;
    let mut sorted_states: Vec<_> = final_map.keys().collect();
    sorted_states.sort();

    for state in sorted_states {
        if let Some(districts) = final_map.get(state) {
            let mut sorted_districts: Vec<_> = districts.iter().collect();
            sorted_districts.sort();
            writeln!(file, "\nState: {} ({})", state, sorted_districts.len())?;
            for district in sorted_districts {
                writeln!(file, "  - {}", district)?;
            }
        }
    }
    println!("Detailed map saved to data/state_district_map.txt");
    Ok(())
}

// --- LOGIC FROM map_state_districts.rs ---

fn run_map_state_districts(pbf_path_str: &str) -> Result<(), Box<dyn Error>> {
    let path = Path::new(pbf_path_str);
    println!("Step 1: building State -> Member Relation map...");
    let reader = ElementReader::from_path(path)?;

    let member_to_state: HashMap<i64, String> = reader.par_map_reduce(
        |element| {
            let mut mappings = HashMap::new();
            if let Element::Relation(relation) = element {
                let tags: HashMap<&str, &str> = relation.tags().collect();
                let is_admin = tags.get("boundary") == Some(&"administrative");
                let is_level_4 = tags.get("admin_level") == Some(&"4");
                let is_india = tags
                    .get("ISO3166-2")
                    .map(|v| v.starts_with("IN-"))
                    .unwrap_or(false)
                    || tags.get("is_in:country_code") == Some(&"IN");

                if is_admin && is_level_4 && is_india {
                    let state_name = tags.get("name").unwrap_or(&"Unnamed State").to_string();
                    for member in relation.members() {
                        if let osmpbf::RelMemberType::Relation = member.member_type {
                            mappings.insert(member.member_id, state_name.clone());
                        }
                    }
                }
            }
            mappings
        },
        HashMap::new,
        |mut a, b| {
            a.extend(b);
            a
        },
    )?;

    println!("Step 2: Resolving District Names...");
    let reader = ElementReader::from_path(path)?;
    let lookup = Arc::new(member_to_state);

    let state_district_map: HashMap<String, HashSet<String>> = reader.par_map_reduce(
        move |element| {
            let mut local_map: HashMap<String, HashSet<String>> = HashMap::new();
            if let Element::Relation(relation) = element {
                if let Some(parent_state) = lookup.get(&relation.id()) {
                    let tags: HashMap<&str, &str> = relation.tags().collect();
                    if tags.get("admin_level") == Some(&"5") {
                        let district_name = tags
                            .get("name")
                            .or_else(|| tags.get("name:en"))
                            .unwrap_or(&"Unnamed District")
                            .to_string();
                        local_map
                            .entry(parent_state.clone())
                            .or_default()
                            .insert(district_name);
                    }
                }
            }
            local_map
        },
        HashMap::new,
        |mut a, b| {
            for (state, districts) in b {
                a.entry(state).or_default().extend(districts);
            }
            a
        },
    )?;

    let mut file = File::create("data/final_state_districts.txt")?;
    let mut sorted_states: Vec<_> = state_district_map.keys().collect();
    sorted_states.sort();

    for state in sorted_states {
        if let Some(districts) = state_district_map.get(state) {
            let mut sorted_districts: Vec<_> = districts.iter().collect();
            sorted_districts.sort();
            writeln!(file, "\nState: {} ({})", state, sorted_districts.len())?;
            for district in sorted_districts {
                writeln!(file, "  - {}", district)?;
            }
        }
    }
    println!("Saved to data/final_state_districts.txt");
    Ok(())
}

// --- LOGIC FROM normalize_state_names.rs ---

fn run_normalize_states() -> Result<(), Box<dyn Error>> {
    let input_path = Path::new("data/city_state_report.txt");
    let output_path = Path::new("data/city_state_report_clean.txt");
    println!("Normalizing State Names...");
    let file = File::open(input_path)?;
    let reader = BufReader::new(file);
    let mut state_data: HashMap<String, HashMap<String, usize>> = HashMap::new();
    let mut current_state = String::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("State: ") {
            let parts: Vec<&str> = line.split(" (").collect();
            let raw_state = parts[0].trim_start_matches("State: ").trim();
            current_state = normalize_state_name(raw_state);
        } else if line.starts_with("- ") {
            let parts: Vec<&str> = line.trim_start_matches("- ").split(": ").collect();
            if parts.len() >= 2 {
                let city = parts[0].trim().to_string();
                let count: usize = parts[1].parse().unwrap_or(0);
                *state_data
                    .entry(current_state.clone())
                    .or_default()
                    .entry(city)
                    .or_insert(0) += count;
            }
        }
    }

    let mut outfile = File::create(output_path)?;
    let mut sorted_states: Vec<_> = state_data.keys().collect();
    sorted_states.sort();
    for state in sorted_states {
        if let Some(cities) = state_data.get(state) {
            writeln!(
                outfile,
                "State: {} ({} unique cities, {} total nodes)",
                state,
                cities.len(),
                cities.values().sum::<usize>()
            )?;
            let mut sorted_cities: Vec<_> = cities.iter().collect();
            sorted_cities.sort_by(|a, b| b.1.cmp(a.1));
            for (city, count) in sorted_cities {
                writeln!(outfile, "  - {}: {}", city, count)?;
            }
            writeln!(outfile)?;
        }
    }
    Ok(())
}

fn normalize_state_name(raw: &str) -> String {
    let lower = raw.to_lowercase();
    match lower.as_str() {
        "andhra pradesh" | "ap" | "andra pradesh" => "Andhra Pradesh".to_string(),
        "arunachal pradesh" => "Arunachal Pradesh".to_string(),
        "assam" | "as" => "Assam".to_string(),
        "bihar" => "Bihar".to_string(),
        "chandigarh" => "Chandigarh".to_string(),
        "chhattisgarh" => "Chhattisgarh".to_string(),
        "delhi" | "dl" | "delhi-nct" | "ncr" => "Delhi".to_string(),
        "goa" => "Goa".to_string(),
        "gujarat" | "gj" | "gu" | "gujrat" => "Gujarat".to_string(),
        "haryana" | "hr" => "Haryana".to_string(),
        "himachal pradesh" => "Himachal Pradesh".to_string(),
        "jammu and kashmir" | "jammu & kashmir" | "j and k" | "kashmir" => {
            "Jammu and Kashmir".to_string()
        }
        "jharkhand" => "Jharkhand".to_string(),
        "karnataka" | "ka" => "Karnataka".to_string(),
        "kerala" | "kl" | "kera" => "Kerala".to_string(),
        "ladakh" => "Ladakh".to_string(),
        "madhya pradesh" | "mp" => "Madhya Pradesh".to_string(),
        "maharashtra" | "mh" | "maharastra" => "Maharashtra".to_string(),
        "manipur" => "Manipur".to_string(),
        "meghalaya" => "Meghalaya".to_string(),
        "mizoram" => "Mizoram".to_string(),
        "nagaland" => "Nagaland".to_string(),
        "odisha" | "or" | "odhisa" => "Odisha".to_string(),
        "puducherry" => "Puducherry".to_string(),
        "punjab" | "punjab, india" => "Punjab".to_string(),
        "rajasthan" | "rj" => "Rajasthan".to_string(),
        "sikkim" => "Sikkim".to_string(),
        "tamil nadu" | "tn" | "tamilnadu" | "தமிழ்நாடு" => {
            "Tamil Nadu".to_string()
        }
        "telangana" | "tg" | "telanagana" => "Telangana".to_string(),
        "tripura" => "Tripura".to_string(),
        "uttar pradesh" | "up" | "utrar pradesh" => "Uttar Pradesh".to_string(),
        "uttarakhand" => "Uttarakhand".to_string(),
        "west bengal" | "wb" | "west-bengal" => "West Bengal".to_string(),
        "dadra and nagar haveli" | "daman and diu" => {
            "Dadra and Nagar Haveli and Daman and Diu".to_string()
        }
        "andaman and nicobar islands" => "Andaman and Nicobar Islands".to_string(),
        _ => raw
            .split_whitespace()
            .map(|w| {
                let mut c = w.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect::<Vec<String>>()
            .join(" "),
    }
}

// --- LOGIC FROM compare_city_district.rs ---

fn run_compare_city_district() -> Result<(), Box<dyn Error>> {
    let districts_path = Path::new("data/districts_list.txt");
    let cities_path = Path::new("data/city_state_report_clean.txt");
    println!("Loading Districts...");
    let districts = load_names_simple(districts_path)?;
    println!("Loading Cities...");
    let cities = load_city_names_simple(cities_path)?;
    let mut intersection = HashSet::new();
    for city in &cities {
        if districts.contains(city) {
            intersection.insert(city.clone());
        }
    }
    println!(
        "Matches (City Name == District Name): {}",
        intersection.len()
    );
    Ok(())
}

fn load_names_simple(path: &Path) -> Result<HashSet<String>, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut set = HashSet::new();
    for line in reader.lines() {
        let line = line?;
        if !line.trim().is_empty() {
            set.insert(line.trim().to_lowercase());
        }
    }
    Ok(set)
}

fn load_city_names_simple(path: &Path) -> Result<HashSet<String>, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut set = HashSet::new();
    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.starts_with("- ") {
            let parts: Vec<&str> = line.trim_start_matches("- ").split(": ").collect();
            if !parts.is_empty() {
                set.insert(parts[0].trim().to_lowercase());
            }
        }
    }
    Ok(set)
}
