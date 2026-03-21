use chrono::Datelike;
use fake::Fake;
use fake::faker::address::en::{BuildingNumber, StreetName};
use h3o::{LatLng, Resolution};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    pub location: String,
    pub city: Option<String>,
    pub state: String,
    pub location_type: String,
    pub postcode: Option<String>,
    pub home_latitude: f64,
    pub home_longitude: f64,
    pub home_h3r5: String,
    pub home_h3r7: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialProfile {
    pub credit_score: u16,
    pub monthly_spend: f64,
    pub customer_risk_score: f32,
    pub is_fraud: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceProfile {
    pub primary_ua: String,
    pub secondary_ua: Option<String>,
    pub isp: String,
    pub ip_subnet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    // Keys
    pub customer_id: String,
    pub name: String,
    pub age: u8,
    pub email: String,

    pub location: GeoLocation,
    pub financial: FinancialProfile,

    // Device Profile
    pub device: DeviceProfile,

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
        device: DeviceProfile,
    ) -> Self {
        let mut rng = rand::rng();

        // Generate a realistic "First Line" of address
        let house_no: String = BuildingNumber().fake();
        let street: String = StreetName().fake();
        let city_str = geo
            .city
            .clone()
            .unwrap_or_else(|| format!("{} Region", geo.state));

        let pin_str = geo
            .postcode
            .as_ref()
            .map(|p| format!(" - {}", p))
            .unwrap_or_default();

        let mut geo = geo;
        geo.location = format!("No. {}, {}, {}{}", house_no, street, city_str, pin_str);

        let coord =
            LatLng::new(geo.home_latitude, geo.home_longitude).expect("Invalid coordinates");

        geo.home_h3r5 = coord.to_cell(Resolution::Five).to_string();

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
            location: geo,
            financial: fin,
            device,
            registration_date: reg_date.to_string(),
            registration_year: reg_date.year(),
            registration_month: reg_date.month(),
            registration_day: reg_date.day(),
        }
    }
}
