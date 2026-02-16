use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let districts_path = Path::new("data/districts_list.txt");
    let cities_path = Path::new("data/city_state_report_clean.txt");
    
    println!("Loading Districts...");
    let districts = load_names(districts_path)?;
    println!("Loaded {} unique District names.", districts.len());

    println!("Loading Cities...");
    let cities = load_city_names(cities_path)?;
    println!("Loaded {} unique City names.", cities.len());

    // Comparison
    let mut intersection = HashSet::new();
    let mut only_cities = HashSet::new();
    let mut only_districts = HashSet::new();

    for city in &cities {
        if districts.contains(city) {
            intersection.insert(city.clone());
        } else {
            only_cities.insert(city.clone());
        }
    }

    for district in &districts {
        if !cities.contains(district) {
            only_districts.insert(district.clone());
        }
    }

    println!("\n--- Comparison Results ---");
    println!("Matches (City Name == District Name): {}", intersection.len());
    println!("Cities that are NOT District names: {}", only_cities.len());
    println!("Districts with NO matching City name: {}", only_districts.len());

    // Output sample matches
    println!("\n--- Sample Matches (First 20) ---");
    for (i, name) in intersection.iter().take(20).enumerate() {
        println!( "{}. {}", i+1, name);
    }

    Ok(())
}

fn load_names(path: &Path) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut set = HashSet::new();
    for line in reader.lines() {
        let line = line?;
        if !line.trim().is_empty() {
            set.insert(line.trim().to_lowercase()); // Compare in lowercase
        }
    }
    Ok(set)
}

fn load_city_names(path: &Path) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut set = HashSet::new();
    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.starts_with("- ") {
             // "- Pune: 1500"
             let parts: Vec<&str> = line.trim_start_matches("- ").split(": ").collect();
             if !parts.is_empty() {
                 set.insert(parts[0].trim().to_lowercase());
             }
        }
    }
    Ok(set)
}
