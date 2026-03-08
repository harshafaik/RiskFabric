use osmpbf::{ElementReader, Element};
use std::collections::HashMap;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("data/india-260126.osm.pbf");
    
    println!("Mapping Cities to States based on Node addresses (Parallel)...");
    let reader = ElementReader::from_path(path)?;
    
    // Result: Map<State, Map<City, Count>>
    let state_city_counts: HashMap<String, HashMap<String, usize>> = reader.par_map_reduce(
        |element| {
            let mut local_counts: HashMap<String, HashMap<String, usize>> = HashMap::new();
            
            let process_tags = |tags: std::collections::HashMap<&str, &str>, counts: &mut HashMap<String, HashMap<String, usize>>| {
                if let (Some(city), Some(state)) = (tags.get("addr:city"), tags.get("addr:state")) {
                    // Normalize slightly (trim whitespace)
                    let clean_city = city.trim().to_string();
                    let clean_state = state.trim().to_string();
                    
                    if !clean_city.is_empty() && !clean_state.is_empty() {
                        *counts.entry(clean_state)
                            .or_default()
                            .entry(clean_city)
                            .or_insert(0) += 1;
                    }
                }
            };

            match element {
                Element::Node(node) => {
                     process_tags(node.tags().collect(), &mut local_counts);
                }
                Element::DenseNode(node) => {
                     process_tags(node.tags().collect(), &mut local_counts);
                }
                _ => {}
            }
            local_counts
        },
        HashMap::new,
        |mut a, b| {
            // Merge two hashmaps
            for (state, cities) in b {
                let state_entry = a.entry(state).or_default();
                for (city, count) in cities {
                    *state_entry.entry(city).or_insert(0) += count;
                }
            }
            a
        },
    )?;

    println!("\n--- Processing Complete. Generating Report... ---");

    use std::fs::File;
    use std::io::Write;
    let mut file = File::create("data/city_state_report.txt")?;

    // Sort States Alphabetically
    let mut sorted_states: Vec<_> = state_city_counts.keys().collect();
    sorted_states.sort();

    for state in sorted_states {
        if let Some(cities) = state_city_counts.get(state) {
            let total_nodes: usize = cities.values().sum();
            let total_unique_cities = cities.len();

            writeln!(file, "State: {} ({} unique cities, {} total nodes)", state, total_unique_cities, total_nodes)?;
            println!("State: {} ({} unique cities)", state, total_unique_cities);

            // Sort Cities by Node Count (Descending) to show most active ones first
            let mut sorted_cities: Vec<_> = cities.iter().collect();
            sorted_cities.sort_by(|a, b| b.1.cmp(a.1));

            // Print top 50 cities for this state to file
            for (city, count) in sorted_cities {
                writeln!(file, "  - {}: {}", city, count)?;
            }
            writeln!(file)?;
        }
    }
    
    println!("\nDetailed report saved to data/city_state_report.txt");

    Ok(())
}