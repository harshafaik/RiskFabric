use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input_path = Path::new("data/city_state_report.txt");
    let output_path = Path::new("data/city_state_report_clean.txt");
    
    println!("Normalizing State Names...");

    let file = File::open(input_path)?;
    let reader = BufReader::new(file);

    // Map<StandardStateName, Map<CityName, Count>>
    let mut state_data: HashMap<String, HashMap<String, usize>> = HashMap::new();
    
    let mut current_state = String::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        
        if line.is_empty() { continue; }

        if line.starts_with("State: ") {
            // Parse "State: Maharashtra (90 unique cities...)"
            let parts: Vec<&str> = line.split(" (").collect();
            let raw_state = parts[0].trim_start_matches("State: ").trim();
            
            current_state = normalize_state(raw_state);
        } else if line.starts_with("- ") {
            // Parse "  - Pune: 1500"
            let parts: Vec<&str> = line.trim_start_matches("- ").split(": ").collect();
            if parts.len() >= 2 {
                let city = parts[0].trim().to_string();
                let count: usize = parts[1].parse().unwrap_or(0);
                
                *state_data.entry(current_state.clone())
                    .or_insert_with(HashMap::new)
                    .entry(city)
                    .or_insert(0) += count;
            }
        }
    }

    // Write output
    let mut outfile = File::create(output_path)?;
    let mut sorted_states: Vec<_> = state_data.keys().collect();
    sorted_states.sort();

    for state in sorted_states {
        if let Some(cities) = state_data.get(state) {
            let total_unique = cities.len();
            let total_nodes: usize = cities.values().sum();
            
            writeln!(outfile, "State: {} ({} unique cities, {} total nodes)", state, total_unique, total_nodes)?;
            
            // Sort cities by count
            let mut sorted_cities: Vec<_> = cities.iter().collect();
            sorted_cities.sort_by(|a, b| b.1.cmp(a.1));
            
            for (city, count) in sorted_cities {
                writeln!(outfile, "  - {}: {}", city, count)?;
            }
            writeln!(outfile, "")?;
        }
    }

    println!("Done. Clean report saved to {:?}", output_path);
    Ok(())
}

fn normalize_state(raw: &str) -> String {
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
        "jammu and kashmir" | "jammu & kashmir" | "j and k" | "kashmir" => "Jammu and Kashmir".to_string(),
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
        "tamil nadu" | "tn" | "tamilnadu" | "தமிழ்நாடு" => "Tamil Nadu".to_string(),
        "telangana" | "tg" | "telanagana" => "Telangana".to_string(),
        "tripura" => "Tripura".to_string(),
        "uttar pradesh" | "up" | "utrar pradesh" => "Uttar Pradesh".to_string(),
        "uttarakhand" => "Uttarakhand".to_string(),
        "west bengal" | "wb" | "west-bengal" => "West Bengal".to_string(),
        "dadra and nagar haveli" | "daman and diu" => "Dadra and Nagar Haveli and Daman and Diu".to_string(),
        "andaman and nicobar islands" => "Andaman and Nicobar Islands".to_string(),
        _ => {
            // Capitalize first letter of each word for others
            raw.split_whitespace()
                .map(|w| {
                    let mut c = w.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    }
                })
                .collect::<Vec<String>>()
                .join(" ")
        }
    }
}
