use osmpbf::{ElementReader, Element};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Mutex, Arc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("data/india-260126.osm.pbf");
    
    println!("Mapping Districts to States (Level 5 Relations) using ISO Codes...");
    let reader = ElementReader::from_path(path)?;
    
    // key: State Code/Name, value: Set of District Names
    let state_district_map = Arc::new(Mutex::new(HashMap::<String, HashSet<String>>::new()));
    let map_clone = state_district_map.clone();

    reader.par_map_reduce(
        move |element| {
            if let Element::Relation(relation) = element {
                let tags: HashMap<&str, &str> = relation.tags().collect();
                
                let is_admin = tags.get("boundary") == Some(&"administrative");
                let is_level_5 = tags.get("admin_level") == Some(&"5");

                let is_india = 
                    tags.get("ISO3166-2").map(|v| v.starts_with("IN-")).unwrap_or(false) ||
                    tags.get("is_in:country_code") == Some(&"IN") ||
                    tags.get("addr:country") == Some(&"IN") ||
                    tags.iter().any(|(k, _)| k.starts_with("ref:LGD")) || 
                    tags.iter().any(|(k, _)| k.starts_with("mdds:")) ||
                    tags.contains_key("is_in:state");

                if is_admin && is_level_5 && is_india {
                    let district_name = tags.get("name")
                        .or_else(|| tags.get("name:en"))
                        .unwrap_or(&"Unnamed District")
                        .to_string();

                    // Strategy to find State:
                    // 1. is_in:state tag
                    // 2. addr:state tag
                    // 3. Extract from ISO3166-2 (e.g., IN-KA-...)
                    
                    let state_key = if let Some(state) = tags.get("is_in:state") {
                        state.to_string()
                    } else if let Some(state) = tags.get("addr:state") {
                        state.to_string()
                    } else if let Some(iso) = tags.get("ISO3166-2") {
                        // ISO format is usually IN-SS-DD where SS is state code.
                        // We extract the first two parts: IN-SS
                        let parts: Vec<&str> = iso.split('-').collect();
                        if parts.len() >= 2 {
                             format!("{}-{}", parts[0], parts[1]) // e.g. IN-KA
                        } else {
                            "Unknown State".to_string()
                        }
                    } else {
                        "Unknown State".to_string()
                    };
                    
                    let mut map = map_clone.lock().unwrap();
                    map.entry(state_key).or_default().insert(district_name);
                }
            }
        },
        || (), 
        |_, _| (), 
    )?;

    let final_map = state_district_map.lock().unwrap();
    
    println!("\n--- State -> District Mapping ---");
    let mut sorted_states: Vec<_> = final_map.keys().collect();
    sorted_states.sort();

    use std::fs::File;
    use std::io::Write;
    let mut file = File::create("data/state_district_map.txt")?;

    for state in sorted_states {
        if let Some(districts) = final_map.get(state) {
            let mut sorted_districts: Vec<_> = districts.iter().collect();
            sorted_districts.sort();
            
            println!("{}: {} districts", state, sorted_districts.len());
            writeln!(file, "\nState: {} ({})", state, sorted_districts.len())?;
            
            for district in sorted_districts {
                writeln!(file, "  - {}", district)?;
            }
        }
    }
    
    println!("\nDone. Detailed map saved to data/state_district_map.txt");

    Ok(())
}