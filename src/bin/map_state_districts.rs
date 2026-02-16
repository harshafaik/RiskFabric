use osmpbf::{ElementReader, Element};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Mutex, Arc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("data/india-260126.osm.pbf");
    
    // --- STEP 1: Map Member IDs to States ---
    println!("Step 1: building State -> Member Relation map...");
    let reader = ElementReader::from_path(path)?;
    
    // Returns: Map<MemberRelationID, StateName>
    let member_to_state: HashMap<i64, String> = reader.par_map_reduce(
        |element| {
            let mut mappings = HashMap::new();
            if let Element::Relation(relation) = element {
                let tags: HashMap<&str, &str> = relation.tags().collect();
                
                let is_admin = tags.get("boundary") == Some(&"administrative");
                let is_level_4 = tags.get("admin_level") == Some(&"4");
                
                let is_india = tags.get("ISO3166-2").map(|v| v.starts_with("IN-")).unwrap_or(false) ||
                               tags.get("is_in:country_code") == Some(&"IN");

                if is_admin && is_level_4 && is_india {
                    let state_name = tags.get("name").unwrap_or(&"Unnamed State").to_string();
                    
                    for member in relation.members() {
                        // We assume subareas (districts) are members of type Relation
                        if let osmpbf::RelMemberType::Relation = member.member_type {
                            mappings.insert(member.member_id, state_name.clone());
                        }
                    }
                }
            }
            mappings
        },
        || HashMap::new(),
        |mut a, b| {
            a.extend(b);
            a
        },
    )?;
    
    println!("Found {} potential sub-relations linked to states.", member_to_state.len());


    // --- STEP 2: Find Names for those Member IDs (Districts) ---
    println!("Step 2: Resolving District Names...");
    let reader = ElementReader::from_path(path)?;
    
    // We need to look up IDs in member_to_state.
    // Since member_to_state is read-only now, we can wrap it in Arc for sharing (or just clone if small enough, but Arc is better).
    let lookup = Arc::new(member_to_state);
    
    // Result: Map<StateName, Set<DistrictName>>
    let state_district_map: HashMap<String, HashSet<String>> = reader.par_map_reduce(
        move |element| {
            let mut local_map: HashMap<String, HashSet<String>> = HashMap::new();
            
            if let Element::Relation(relation) = element {
                // Check if this relation ID is one we are looking for (i.e., it's a child of a state)
                if let Some(parent_state) = lookup.get(&relation.id()) {
                    let tags: HashMap<&str, &str> = relation.tags().collect();
                    
                    // Verify it is indeed a district (Level 5) to filter out other members like rivers/roads
                    // Although sometimes admin_level might be missing or different, being a subarea member is a strong signal.
                    // But let's check admin_level=5 to be safe and consistent with previous findings.
                    let is_level_5 = tags.get("admin_level") == Some(&"5");
                    
                    if is_level_5 {
                        let district_name = tags.get("name")
                            .or_else(|| tags.get("name:en"))
                            .unwrap_or(&"Unnamed District")
                            .to_string();
                        
                        local_map.entry(parent_state.clone())
                            .or_insert_with(HashSet::new)
                            .insert(district_name);
                    }
                }
            }
            local_map
        },
        || HashMap::new(),
        |mut a, b| {
            for (state, districts) in b {
                a.entry(state).or_insert_with(HashSet::new).extend(districts);
            }
            a
        },
    )?;


    // --- STEP 3: Output ---
    println!("\n--- Resolved State -> District Mapping ---");
    let mut sorted_states: Vec<_> = state_district_map.keys().collect();
    sorted_states.sort();

    use std::fs::File;
    use std::io::Write;
    let mut file = File::create("data/final_state_districts.txt")?;

    for state in sorted_states {
        if let Some(districts) = state_district_map.get(state) {
            let mut sorted_districts: Vec<_> = districts.iter().collect();
            sorted_districts.sort();
            
            println!("{}: {} districts", state, sorted_districts.len());
            writeln!(file, "\nState: {} ({})", state, sorted_districts.len())?;
            
            for district in sorted_districts {
                writeln!(file, "  - {}", district)?;
            }
        }
    }
    
    println!("\nSaved to data/final_state_districts.txt");

    Ok(())
}