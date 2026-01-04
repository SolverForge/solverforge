//! Demo data generators for Vehicle Routing Problem.
//!
//! Provides realistic demo datasets for three cities:
//! - Philadelphia (49 visits, 6 vehicles)
//! - Hartford (30 visits, 6 vehicles)
//! - Firenze (48 visits, 6 vehicles)
//!
//! Uses real street addresses and weighted customer types:
//! - Residential (50%): 17:00-20:00, demand 1-2
//! - Business (30%): 09:00-17:00, demand 3-6
//! - Restaurant (20%): 06:00-10:00, demand 5-10

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::domain::{Location, Vehicle, VehicleRoutePlan, Visit};

/// Vehicle names using phonetic alphabet.
const VEHICLE_NAMES: [&str; 10] = [
    "Alpha", "Bravo", "Charlie", "Delta", "Echo",
    "Foxtrot", "Golf", "Hotel", "India", "Juliet",
];

/// Customer type with time window and demand characteristics.
#[derive(Clone, Copy)]
enum CustomerType {
    /// Evening deliveries (17:00-20:00), small orders
    Residential,
    /// Business hours (09:00-17:00), medium orders
    Business,
    /// Early morning (06:00-10:00), large orders
    Restaurant,
}

impl CustomerType {
    fn time_window(&self) -> (i64, i64) {
        match self {
            CustomerType::Residential => (17 * 3600, 20 * 3600),
            CustomerType::Business => (9 * 3600, 17 * 3600),
            CustomerType::Restaurant => (6 * 3600, 10 * 3600),
        }
    }

    fn demand_range(&self) -> (i32, i32) {
        match self {
            CustomerType::Residential => (1, 2),
            CustomerType::Business => (3, 6),
            CustomerType::Restaurant => (5, 10),
        }
    }

    fn service_duration_range(&self) -> (i64, i64) {
        match self {
            CustomerType::Residential => (5 * 60, 10 * 60),
            CustomerType::Business => (15 * 60, 30 * 60),
            CustomerType::Restaurant => (20 * 60, 40 * 60),
        }
    }

    /// Weighted random selection: 50% residential, 30% business, 20% restaurant.
    fn random(rng: &mut StdRng) -> Self {
        let r: u32 = rng.gen_range(1..=100);
        if r <= 50 {
            CustomerType::Residential
        } else if r <= 80 {
            CustomerType::Business
        } else {
            CustomerType::Restaurant
        }
    }
}

/// Location data with name, coordinates, and optional type.
struct LocationData {
    name: &'static str,
    lat: f64,
    lng: f64,
    customer_type: Option<CustomerType>,
}

/// Demo dataset configuration.
struct DemoConfig {
    seed: u64,
    visit_count: usize,
    vehicle_count: usize,
    vehicle_start_time: i64,
    min_capacity: i32,
    max_capacity: i32,
}

// ============================================================================
// Philadelphia Data
// ============================================================================

const PHILADELPHIA_DEPOTS: &[LocationData] = &[
    LocationData { name: "Central Depot - City Hall", lat: 39.9526, lng: -75.1652, customer_type: None },
    LocationData { name: "South Philly Depot", lat: 39.9256, lng: -75.1697, customer_type: None },
    LocationData { name: "University City Depot", lat: 39.9522, lng: -75.1932, customer_type: None },
    LocationData { name: "North Philly Depot", lat: 39.9907, lng: -75.1556, customer_type: None },
    LocationData { name: "Fishtown Depot", lat: 39.9712, lng: -75.1340, customer_type: None },
    LocationData { name: "West Philly Depot", lat: 39.9601, lng: -75.2175, customer_type: None },
];

const PHILADELPHIA_VISITS: &[LocationData] = &[
    // Restaurants
    LocationData { name: "Reading Terminal Market", lat: 39.9535, lng: -75.1589, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Parc Restaurant", lat: 39.9493, lng: -75.1727, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Zahav", lat: 39.9430, lng: -75.1474, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Vetri Cucina", lat: 39.9499, lng: -75.1659, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Talula's Garden", lat: 39.9470, lng: -75.1709, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Fork", lat: 39.9493, lng: -75.1539, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Morimoto", lat: 39.9488, lng: -75.1559, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Vernick Food & Drink", lat: 39.9508, lng: -75.1718, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Friday Saturday Sunday", lat: 39.9492, lng: -75.1715, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Royal Izakaya", lat: 39.9410, lng: -75.1509, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Laurel", lat: 39.9392, lng: -75.1538, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Marigold Kitchen", lat: 39.9533, lng: -75.1920, customer_type: Some(CustomerType::Restaurant) },
    // Businesses
    LocationData { name: "Comcast Center", lat: 39.9543, lng: -75.1690, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Liberty Place", lat: 39.9520, lng: -75.1685, customer_type: Some(CustomerType::Business) },
    LocationData { name: "BNY Mellon Center", lat: 39.9505, lng: -75.1660, customer_type: Some(CustomerType::Business) },
    LocationData { name: "One Liberty Place", lat: 39.9520, lng: -75.1685, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Aramark Tower", lat: 39.9550, lng: -75.1705, customer_type: Some(CustomerType::Business) },
    LocationData { name: "PSFS Building", lat: 39.9510, lng: -75.1618, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Three Logan Square", lat: 39.9567, lng: -75.1720, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Two Commerce Square", lat: 39.9551, lng: -75.1675, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Penn Medicine", lat: 39.9495, lng: -75.1935, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Children's Hospital", lat: 39.9482, lng: -75.1950, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Drexel University", lat: 39.9566, lng: -75.1899, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Temple University", lat: 39.9812, lng: -75.1554, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Jefferson Hospital", lat: 39.9487, lng: -75.1577, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Pennsylvania Hospital", lat: 39.9445, lng: -75.1545, customer_type: Some(CustomerType::Business) },
    LocationData { name: "FMC Tower", lat: 39.9499, lng: -75.1780, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Cira Centre", lat: 39.9560, lng: -75.1822, customer_type: Some(CustomerType::Business) },
    // Residential
    LocationData { name: "Rittenhouse Square", lat: 39.9496, lng: -75.1718, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Washington Square West", lat: 39.9468, lng: -75.1545, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Society Hill", lat: 39.9425, lng: -75.1478, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Old City", lat: 39.9510, lng: -75.1450, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Northern Liberties", lat: 39.9650, lng: -75.1420, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Fishtown", lat: 39.9712, lng: -75.1340, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Queen Village", lat: 39.9380, lng: -75.1520, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Bella Vista", lat: 39.9395, lng: -75.1598, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Graduate Hospital", lat: 39.9425, lng: -75.1768, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Fairmount", lat: 39.9680, lng: -75.1750, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Spring Garden", lat: 39.9620, lng: -75.1620, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Art Museum Area", lat: 39.9656, lng: -75.1810, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Brewerytown", lat: 39.9750, lng: -75.1850, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "East Passyunk", lat: 39.9310, lng: -75.1605, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Point Breeze", lat: 39.9285, lng: -75.1780, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Pennsport", lat: 39.9320, lng: -75.1450, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Powelton Village", lat: 39.9610, lng: -75.1950, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Spruce Hill", lat: 39.9530, lng: -75.2100, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Cedar Park", lat: 39.9490, lng: -75.2200, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Kensington", lat: 39.9850, lng: -75.1280, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Port Richmond", lat: 39.9870, lng: -75.1120, customer_type: Some(CustomerType::Residential) },
];

// ============================================================================
// Hartford Data
// ============================================================================

const HARTFORD_DEPOTS: &[LocationData] = &[
    LocationData { name: "Downtown Hartford Depot", lat: 41.7658, lng: -72.6734, customer_type: None },
    LocationData { name: "Asylum Hill Depot", lat: 41.7700, lng: -72.6900, customer_type: None },
    LocationData { name: "South End Depot", lat: 41.7400, lng: -72.6750, customer_type: None },
    LocationData { name: "West End Depot", lat: 41.7680, lng: -72.7100, customer_type: None },
    LocationData { name: "Barry Square Depot", lat: 41.7450, lng: -72.6800, customer_type: None },
    LocationData { name: "Clay Arsenal Depot", lat: 41.7750, lng: -72.6850, customer_type: None },
];

const HARTFORD_VISITS: &[LocationData] = &[
    // Restaurants
    LocationData { name: "Max Downtown", lat: 41.7670, lng: -72.6730, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Trumbull Kitchen", lat: 41.7650, lng: -72.6750, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Salute", lat: 41.7630, lng: -72.6740, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Peppercorns Grill", lat: 41.7690, lng: -72.6680, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Feng Asian Bistro", lat: 41.7640, lng: -72.6725, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "On20", lat: 41.7655, lng: -72.6728, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "First and Last Tavern", lat: 41.7620, lng: -72.7050, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Agave Grill", lat: 41.7580, lng: -72.6820, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Bear's Smokehouse", lat: 41.7550, lng: -72.6780, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "City Steam Brewery", lat: 41.7630, lng: -72.6750, customer_type: Some(CustomerType::Restaurant) },
    // Businesses
    LocationData { name: "Travelers Tower", lat: 41.7658, lng: -72.6734, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Hartford Steam Boiler", lat: 41.7680, lng: -72.6700, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Aetna Building", lat: 41.7700, lng: -72.6900, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Connecticut Convention Center", lat: 41.7615, lng: -72.6820, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Hartford Hospital", lat: 41.7547, lng: -72.6858, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Connecticut Children's", lat: 41.7560, lng: -72.6850, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Trinity College", lat: 41.7474, lng: -72.6909, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Connecticut Science Center", lat: 41.7650, lng: -72.6695, customer_type: Some(CustomerType::Business) },
    // Residential
    LocationData { name: "West End Hartford", lat: 41.7680, lng: -72.7000, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Asylum Hill", lat: 41.7720, lng: -72.6850, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Frog Hollow", lat: 41.7580, lng: -72.6900, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Barry Square", lat: 41.7450, lng: -72.6800, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "South End", lat: 41.7400, lng: -72.6750, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Blue Hills", lat: 41.7850, lng: -72.7050, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Parkville", lat: 41.7650, lng: -72.7100, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Behind the Rocks", lat: 41.7550, lng: -72.7050, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Charter Oak", lat: 41.7495, lng: -72.6650, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Sheldon Charter Oak", lat: 41.7510, lng: -72.6700, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Clay Arsenal", lat: 41.7750, lng: -72.6850, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Upper Albany", lat: 41.7780, lng: -72.6950, customer_type: Some(CustomerType::Residential) },
];

// ============================================================================
// Firenze Data
// ============================================================================

const FIRENZE_DEPOTS: &[LocationData] = &[
    LocationData { name: "Centro Storico Depot", lat: 43.7696, lng: 11.2558, customer_type: None },
    LocationData { name: "Santa Maria Novella Depot", lat: 43.7745, lng: 11.2487, customer_type: None },
    LocationData { name: "Campo di Marte Depot", lat: 43.7820, lng: 11.2820, customer_type: None },
    LocationData { name: "Rifredi Depot", lat: 43.7950, lng: 11.2410, customer_type: None },
    LocationData { name: "Novoli Depot", lat: 43.7880, lng: 11.2220, customer_type: None },
    LocationData { name: "Gavinana Depot", lat: 43.7520, lng: 11.2680, customer_type: None },
];

const FIRENZE_VISITS: &[LocationData] = &[
    // Restaurants
    LocationData { name: "Trattoria Mario", lat: 43.7750, lng: 11.2530, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Buca Mario", lat: 43.7698, lng: 11.2505, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Il Latini", lat: 43.7705, lng: 11.2495, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Osteria dell'Enoteca", lat: 43.7680, lng: 11.2545, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Trattoria Sostanza", lat: 43.7735, lng: 11.2470, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "All'Antico Vinaio", lat: 43.7690, lng: 11.2570, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Mercato Centrale", lat: 43.7762, lng: 11.2540, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Cibreo", lat: 43.7702, lng: 11.2670, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Ora d'Aria", lat: 43.7710, lng: 11.2610, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Buca Lapi", lat: 43.7720, lng: 11.2535, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Il Palagio", lat: 43.7680, lng: 11.2550, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Enoteca Pinchiorri", lat: 43.7695, lng: 11.2620, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "La Giostra", lat: 43.7745, lng: 11.2650, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Fishing Lab", lat: 43.7730, lng: 11.2560, customer_type: Some(CustomerType::Restaurant) },
    LocationData { name: "Trattoria Cammillo", lat: 43.7665, lng: 11.2520, customer_type: Some(CustomerType::Restaurant) },
    // Businesses
    LocationData { name: "Palazzo Vecchio", lat: 43.7693, lng: 11.2563, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Uffizi Gallery", lat: 43.7677, lng: 11.2553, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Gucci Garden", lat: 43.7692, lng: 11.2556, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Ferragamo Museum", lat: 43.7700, lng: 11.2530, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Ospedale Santa Maria", lat: 43.7830, lng: 11.2690, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Universita degli Studi", lat: 43.7765, lng: 11.2555, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Palazzo Strozzi", lat: 43.7706, lng: 11.2515, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Biblioteca Nazionale", lat: 43.7660, lng: 11.2650, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Teatro del Maggio", lat: 43.7780, lng: 11.2470, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Palazzo Pitti", lat: 43.7650, lng: 11.2500, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Accademia Gallery", lat: 43.7768, lng: 11.2590, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Ospedale Meyer", lat: 43.7910, lng: 11.2520, customer_type: Some(CustomerType::Business) },
    LocationData { name: "Polo Universitario", lat: 43.7920, lng: 11.2180, customer_type: Some(CustomerType::Business) },
    // Residential
    LocationData { name: "Santo Spirito", lat: 43.7665, lng: 11.2470, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "San Frediano", lat: 43.7680, lng: 11.2420, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Santa Croce", lat: 43.7688, lng: 11.2620, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "San Lorenzo", lat: 43.7755, lng: 11.2540, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "San Marco", lat: 43.7780, lng: 11.2585, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Sant'Ambrogio", lat: 43.7705, lng: 11.2680, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Campo di Marte", lat: 43.7820, lng: 11.2820, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Novoli", lat: 43.7880, lng: 11.2220, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Rifredi", lat: 43.7950, lng: 11.2410, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Le Cure", lat: 43.7890, lng: 11.2580, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Careggi", lat: 43.8020, lng: 11.2530, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Peretola", lat: 43.7960, lng: 11.2050, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Isolotto", lat: 43.7620, lng: 11.2200, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Gavinana", lat: 43.7520, lng: 11.2680, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Galluzzo", lat: 43.7400, lng: 11.2480, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Porta Romana", lat: 43.7610, lng: 11.2560, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Bellosguardo", lat: 43.7650, lng: 11.2350, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Arcetri", lat: 43.7500, lng: 11.2530, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Fiesole", lat: 43.8055, lng: 11.2935, customer_type: Some(CustomerType::Residential) },
    LocationData { name: "Settignano", lat: 43.7850, lng: 11.3100, customer_type: Some(CustomerType::Residential) },
];

// ============================================================================
// Generator Functions
// ============================================================================

fn generate_demo_data(
    name: &str,
    config: &DemoConfig,
    depots: &[LocationData],
    visit_data: &[LocationData],
) -> VehicleRoutePlan {
    let mut rng = StdRng::seed_from_u64(config.seed);

    // Build locations: depots first, then visit locations
    let mut locations = Vec::new();
    let mut location_idx = 0;

    // Add depot locations
    for depot in depots.iter().take(config.vehicle_count) {
        locations.push(Location::new(location_idx, depot.lat, depot.lng));
        location_idx += 1;
    }

    // Shuffle visit data for variety
    let mut shuffled_visits: Vec<_> = visit_data.iter().collect();
    for i in (1..shuffled_visits.len()).rev() {
        let j = rng.gen_range(0..=i);
        shuffled_visits.swap(i, j);
    }

    // Add visit locations
    for visit in shuffled_visits.iter().take(config.visit_count) {
        locations.push(Location::new(location_idx, visit.lat, visit.lng));
        location_idx += 1;
    }

    // Create vehicles
    let vehicles: Vec<_> = (0..config.vehicle_count)
        .map(|i| {
            let capacity = rng.gen_range(config.min_capacity..=config.max_capacity);
            Vehicle::new(
                i,
                VEHICLE_NAMES[i % VEHICLE_NAMES.len()],
                capacity,
                i, // depot location index
            )
            .with_departure_time(config.vehicle_start_time)
        })
        .collect();

    // Create visits
    let depot_count = config.vehicle_count.min(depots.len());
    let visits: Vec<_> = shuffled_visits
        .iter()
        .take(config.visit_count)
        .enumerate()
        .map(|(i, loc_data)| {
            let ctype = loc_data.customer_type.unwrap_or_else(|| CustomerType::random(&mut rng));
            let (min_time, max_time) = ctype.time_window();
            let (min_demand, max_demand) = ctype.demand_range();
            let (min_service, max_service) = ctype.service_duration_range();

            let demand = rng.gen_range(min_demand..=max_demand);
            let service_duration = rng.gen_range(min_service..=max_service);

            Visit::new(i, loc_data.name, depot_count + i)
                .with_demand(demand)
                .with_time_window(min_time, max_time)
                .with_service_duration(service_duration)
        })
        .collect();

    let mut plan = VehicleRoutePlan::new(name, locations, visits, vehicles);
    plan.finalize();
    plan
}

/// Generates Philadelphia demo data (49 visits, 10 vehicles).
///
/// # Examples
///
/// ```
/// use vehicle_routing::demo_data::generate_philadelphia;
///
/// let plan = generate_philadelphia();
/// assert_eq!(plan.name, "Philadelphia");
/// assert_eq!(plan.visits.len(), 49);
/// assert_eq!(plan.vehicles.len(), 10);
/// ```
pub fn generate_philadelphia() -> VehicleRoutePlan {
    let config = DemoConfig {
        seed: 0,
        visit_count: PHILADELPHIA_VISITS.len(),
        vehicle_count: 10,
        vehicle_start_time: 6 * 3600, // 6am
        min_capacity: 15,
        max_capacity: 30,
    };
    generate_demo_data("Philadelphia", &config, PHILADELPHIA_DEPOTS, PHILADELPHIA_VISITS)
}

/// Generates Hartford demo data (30 visits, 10 vehicles).
///
/// # Examples
///
/// ```
/// use vehicle_routing::demo_data::generate_hartford;
///
/// let plan = generate_hartford();
/// assert_eq!(plan.name, "Hartford");
/// assert_eq!(plan.visits.len(), 30);
/// assert_eq!(plan.vehicles.len(), 10);
/// ```
pub fn generate_hartford() -> VehicleRoutePlan {
    let config = DemoConfig {
        seed: 1,
        visit_count: HARTFORD_VISITS.len(),
        vehicle_count: 10,
        vehicle_start_time: 6 * 3600,
        min_capacity: 20,
        max_capacity: 30,
    };
    generate_demo_data("Hartford", &config, HARTFORD_DEPOTS, HARTFORD_VISITS)
}

/// Generates Firenze demo data (48 visits, 10 vehicles).
///
/// # Examples
///
/// ```
/// use vehicle_routing::demo_data::generate_firenze;
///
/// let plan = generate_firenze();
/// assert_eq!(plan.name, "Firenze");
/// assert_eq!(plan.visits.len(), 48);
/// assert_eq!(plan.vehicles.len(), 10);
/// ```
pub fn generate_firenze() -> VehicleRoutePlan {
    let config = DemoConfig {
        seed: 2,
        visit_count: FIRENZE_VISITS.len(),
        vehicle_count: 10,
        vehicle_start_time: 6 * 3600,
        min_capacity: 20,
        max_capacity: 40,
    };
    generate_demo_data("Firenze", &config, FIRENZE_DEPOTS, FIRENZE_VISITS)
}

/// Returns all available demo dataset names.
pub fn available_datasets() -> &'static [&'static str] {
    &["PHILADELPHIA", "HARTFORD", "FIRENZE"]
}

/// Generates demo data by name.
///
/// # Examples
///
/// ```
/// use vehicle_routing::demo_data::generate_by_name;
///
/// let plan = generate_by_name("PHILADELPHIA").unwrap();
/// assert_eq!(plan.name, "Philadelphia");
///
/// assert!(generate_by_name("UNKNOWN").is_none());
/// ```
pub fn generate_by_name(name: &str) -> Option<VehicleRoutePlan> {
    match name.to_uppercase().as_str() {
        "PHILADELPHIA" => Some(generate_philadelphia()),
        "HARTFORD" => Some(generate_hartford()),
        "FIRENZE" => Some(generate_firenze()),
        _ => None,
    }
}
