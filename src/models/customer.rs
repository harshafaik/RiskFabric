use chrono::Datelike;
use fake::Fake;
use fake::faker::address::en::{BuildingNumber, StreetName};
use h3o::{LatLng, Resolution};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    pub state: String,
    pub city: Option<String>,
    pub lat: f64,
    pub long: f64,
    pub h3_r7: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialProfile {
    pub credit_score: u16,
    pub monthly_spend: f64,
    pub customer_risk_score: f32,
    pub is_fraud: bool,
}

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

impl Customer {
    pub fn new(
        customer_id: String,
        name: String,
        age: u8,
        email: String,
        geo: GeoLocation,
        fin: FinancialProfile,
    ) -> Self {
        let mut rng = rand::rng();

        // Generate a realistic "First Line" of address
        let house_no: String = BuildingNumber().fake();
        let street: String = StreetName().fake();
        let city_str = geo
            .city
            .clone()
            .unwrap_or_else(|| format!("{} Region", geo.state));

        let location = format!("No. {}, {}, {}", house_no, street, city_str);

        let coord = LatLng::new(geo.lat, geo.long).expect("Invalid coordinates");
        let h3r5 = coord.to_cell(Resolution::Five).to_string();

        let end_date = chrono::Utc::now().date_naive();
        let start_date = end_date - chrono::Duration::days(365 * 5);
        let days_between = (end_date - start_date).num_days();
        let random_days = rng.random_range(0..days_between);
        let reg_date = start_date + chrono::Duration::days(random_days);

        Customer {
            customer_id,
            name,
            age,
            email,
            state: geo.state,
            location,
            location_type: "Urban".to_string(), // Usually overridden
            home_latitude: geo.lat,
            home_longitude: geo.long,
            home_h3r5: h3r5,
            home_h3r7: geo.h3_r7,
            credit_score: fin.credit_score,
            monthly_spend: fin.monthly_spend,
            customer_risk_score: fin.customer_risk_score,
            is_fraud: fin.is_fraud,
            registration_date: reg_date.to_string(),
            registration_year: reg_date.year(),
            registration_month: reg_date.month(),
            registration_day: reg_date.day(),
        }
    }
}
