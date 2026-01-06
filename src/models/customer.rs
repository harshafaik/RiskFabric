use chrono::{Datelike, NaiveDate};
use fake::Fake;
use fake::faker::address::en::{CityName, CitySuffix, Latitude, Longitude, ZipCode};
use fake::faker::internet::en::SafeEmail;
use fake::faker::name::en::Name;
use h3o::{LatLng, Resolution};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    // Keys
    pub customer_id: String,
    pub name: String,
    pub age: u8,
    pub email: String,

    // Geography
    pub location: String,
    pub state: String,
    pub location_type: String,
    pub home_latitude: f64,
    pub home_longitude: f64,
    pub home_h3r5: String,
    pub home_h3r7: String,

    //Demographics
    pub credit_score: u16,
    pub monthly_spend: f64,
    pub customer_risk_score: f32,
    pub is_fraud: bool,

    // --- Metadata ---
    pub registration_date: String, // "YYYY-MM-DD"
    pub registration_year: i32,
    pub registration_month: u32,
    pub registration_day: u32,
}

const STATE: [&str; 8] = [
    "Karnataka",
    "Maharashtra",
    "Tamil Nadu",
    "Delhi",
    "Telangana",
    "Uttar Pradesh",
    "West Bengal",
    "Gujarat",
];

const LOCATION: &[&str] = &["Urban", "Semi-Urban", "Rural", "Metro"];

impl Customer {
    pub fn random() -> Self {
        let name: String = Name().fake();
        let customer_id = uuid::Uuid::new_v4().to_string();
        let lat: f64 = (8.0..37.0).fake();
        let long: f64 = (68.0..97.0).fake();

        let coord = LatLng::new(lat, long).expect("Invalid coordinates");
        let h3r5 = coord.to_cell(Resolution::Five).to_string();
        let h3r7 = coord.to_cell(Resolution::Seven).to_string();

        let mut rng = rand::rng();
        let state = STATE[rng.random_range(0..STATE.len())].to_string();
        let location = LOCATION[rng.random_range(0..LOCATION.len())].to_string();

        let end_date = chrono::Utc::now().date_naive();
        let start_date = end_date - chrono::Duration::days(365 * 5);
        let days_between = (end_date - start_date).num_days();
        let random_days = rng.random_range(0..days_between);
        let reg_date = start_date + chrono::Duration::days(random_days);

        Customer {
            customer_id,
            name: name.clone(),
            age: (18..90).fake(),
            email: SafeEmail().fake::<String>(),

            state: state,
            location: format!(
                "{}, {}",
                CityName().fake::<String>(),
                CitySuffix().fake::<String>()
            ),
            location_type: location,
            home_latitude: lat,
            home_longitude: long,
            home_h3r5: h3r5,
            home_h3r7: h3r7,

            credit_score: (300..850).fake(),
            monthly_spend: (1000.0..50000.0).fake(),
            customer_risk_score: (0.01..0.99).fake(),
            is_fraud: rng.random_bool(0.02),

            registration_date: reg_date.to_string(),
            registration_year: reg_date.year(),
            registration_month: reg_date.month(),
            registration_day: reg_date.day(),
        }
    }
}
