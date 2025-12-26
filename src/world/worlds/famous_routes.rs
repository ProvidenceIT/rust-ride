//! Famous pro cycling routes with historical context.
//!
//! T079: Create famous routes data structure
//! T084: Implement famous route loader
//! T086: Add famous routes to route browser

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{RouteDefinition, RouteDifficulty, WorldTheme};
use crate::world::route::{RouteSource, StoredRoute};

/// Country/region for famous routes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteCountry {
    France,
    Italy,
    Spain,
    Belgium,
    Netherlands,
    Switzerland,
    Austria,
    Germany,
    UK,
    USA,
    Colombia,
    Australia,
}

impl RouteCountry {
    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::France => "France",
            Self::Italy => "Italy",
            Self::Spain => "Spain",
            Self::Belgium => "Belgium",
            Self::Netherlands => "Netherlands",
            Self::Switzerland => "Switzerland",
            Self::Austria => "Austria",
            Self::Germany => "Germany",
            Self::UK => "United Kingdom",
            Self::USA => "United States",
            Self::Colombia => "Colombia",
            Self::Australia => "Australia",
        }
    }
}

/// Type of famous route
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FamousRouteType {
    /// Mountain climb (HC or Cat 1-4)
    MountainClimb,
    /// Classic one-day race
    ClassicRace,
    /// Grand Tour stage
    GrandTourStage,
    /// Time trial course
    TimeTrial,
    /// Cobbled section
    Cobbles,
}

/// Historical race information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceHistory {
    /// First year this route was used in racing
    pub first_raced: u16,
    /// Notable races featuring this route
    pub notable_races: Vec<String>,
    /// Famous winners/moments on this route
    pub famous_moments: Vec<String>,
    /// Record time (if applicable)
    pub record_time: Option<RecordTime>,
}

/// Record time on a segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordTime {
    /// Time in seconds
    pub time_seconds: f64,
    /// Rider name
    pub rider: String,
    /// Year set
    pub year: u16,
    /// Average power if known
    pub avg_power_watts: Option<u16>,
}

/// A famous pro cycling route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FamousRoute {
    /// Unique identifier (e.g., "alpe_dhuez")
    pub id: String,
    /// Display name
    pub name: String,
    /// Country/region
    pub country: RouteCountry,
    /// Route type
    pub route_type: FamousRouteType,
    /// Distance in meters
    pub distance_meters: f64,
    /// Elevation gain in meters
    pub elevation_gain_meters: f32,
    /// Average gradient percentage
    pub avg_gradient_percent: f32,
    /// Maximum gradient percentage
    pub max_gradient_percent: f32,
    /// Starting elevation in meters
    pub start_elevation_meters: f32,
    /// Summit/finish elevation in meters
    pub finish_elevation_meters: f32,
    /// Difficulty rating
    pub difficulty: RouteDifficulty,
    /// Description for display
    pub description: String,
    /// Historical context
    pub history: RaceHistory,
    /// Elevation profile points (distance, elevation)
    pub elevation_profile: Vec<(f64, f32)>,
    /// Key landmarks along the route
    pub landmarks: Vec<RouteLandmark>,
}

/// A landmark on a famous route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteLandmark {
    /// Distance from start in meters
    pub distance_meters: f64,
    /// Name of landmark
    pub name: String,
    /// Description
    pub description: String,
}

impl FamousRoute {
    /// Convert to a RouteDefinition for the world system
    pub fn to_route_definition(&self) -> RouteDefinition {
        RouteDefinition {
            id: self.id.clone(),
            name: self.name.clone(),
            distance_meters: self.distance_meters as f32,
            elevation_gain_meters: self.elevation_gain_meters,
            difficulty: self.difficulty,
            is_loop: false,
            waypoints_file: None,
        }
    }

    /// Convert to a StoredRoute for the route browser
    pub fn to_stored_route(&self) -> StoredRoute {
        // Generate a stable UUID by hashing the route id string
        // This ensures the same route always gets the same UUID
        let hash = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            self.id.hash(&mut hasher);
            hasher.finish()
        };
        // Create UUID bytes from hash (deterministic)
        let bytes = [
            (hash >> 56) as u8,
            (hash >> 48) as u8,
            (hash >> 40) as u8,
            (hash >> 32) as u8,
            (hash >> 24) as u8,
            (hash >> 16) as u8,
            (hash >> 8) as u8,
            hash as u8,
            // Second half uses inverted hash for more uniqueness
            (!hash >> 56) as u8,
            (!hash >> 48) as u8,
            (!hash >> 40) as u8,
            (!hash >> 32) as u8,
            (!hash >> 24) as u8,
            (!hash >> 16) as u8,
            (!hash >> 8) as u8,
            !hash as u8,
        ];
        let id = Uuid::from_bytes(bytes);
        let now = Utc::now();

        StoredRoute {
            id,
            name: self.name.clone(),
            description: Some(self.description.clone()),
            source: RouteSource::Famous,
            distance_meters: self.distance_meters,
            elevation_gain_meters: self.elevation_gain_meters,
            max_elevation_meters: self.finish_elevation_meters,
            min_elevation_meters: self.start_elevation_meters,
            avg_gradient_percent: self.avg_gradient_percent,
            max_gradient_percent: self.max_gradient_percent,
            source_file: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get theme based on route type
    pub fn theme(&self) -> WorldTheme {
        match self.route_type {
            FamousRouteType::MountainClimb => WorldTheme::Mountains,
            FamousRouteType::Cobbles => WorldTheme::Countryside,
            _ => WorldTheme::Countryside,
        }
    }
}

/// Famous routes library
pub struct FamousRoutesLibrary {
    routes: Vec<FamousRoute>,
}

impl FamousRoutesLibrary {
    /// Create library with all bundled famous routes
    pub fn new() -> Self {
        Self {
            routes: get_all_famous_routes(),
        }
    }

    /// Get all routes
    pub fn all(&self) -> &[FamousRoute] {
        &self.routes
    }

    /// Get routes by country
    pub fn by_country(&self, country: RouteCountry) -> Vec<&FamousRoute> {
        self.routes
            .iter()
            .filter(|r| r.country == country)
            .collect()
    }

    /// Get routes by type
    pub fn by_type(&self, route_type: FamousRouteType) -> Vec<&FamousRoute> {
        self.routes
            .iter()
            .filter(|r| r.route_type == route_type)
            .collect()
    }

    /// Get routes by difficulty
    pub fn by_difficulty(&self, difficulty: RouteDifficulty) -> Vec<&FamousRoute> {
        self.routes
            .iter()
            .filter(|r| r.difficulty == difficulty)
            .collect()
    }

    /// Find route by ID
    pub fn find(&self, id: &str) -> Option<&FamousRoute> {
        self.routes.iter().find(|r| r.id == id)
    }

    /// Get route count
    pub fn count(&self) -> usize {
        self.routes.len()
    }

    /// Search routes by name
    pub fn search(&self, query: &str) -> Vec<&FamousRoute> {
        let query_lower = query.to_lowercase();
        self.routes
            .iter()
            .filter(|r| r.name.to_lowercase().contains(&query_lower))
            .collect()
    }

    /// Get all famous routes as StoredRoutes for route browser integration
    pub fn as_stored_routes(&self) -> Vec<StoredRoute> {
        self.routes.iter().map(|r| r.to_stored_route()).collect()
    }
}

impl Default for FamousRoutesLibrary {
    fn default() -> Self {
        Self::new()
    }
}

/// Get all bundled famous routes (T080-T083, T085)
pub fn get_all_famous_routes() -> Vec<FamousRoute> {
    vec![
        // France - Tour de France Climbs
        alpe_dhuez(),
        mont_ventoux(),
        col_du_tourmalet(),
        col_du_galibier(),
        col_de_la_madeleine(),
        col_dizoard(),
        // Italy - Giro d'Italia Climbs
        passo_dello_stelvio(),
        passo_gavia(),
        mortirolo(),
        monte_zoncolan(),
        // Spain - Vuelta Climbs
        alto_de_langliru(),
        lagos_de_covadonga(),
        // Belgium - Classics
        mur_de_huy(),
        koppenberg(),
        oude_kwaremont(),
        paterberg(),
        // Other Notable Climbs
        sa_calobra(),
        grossglockner(),
        col_de_joux_plane(),
        col_de_la_croix_de_fer(),
    ]
}

// =============================================================================
// FRENCH CLIMBS (Tour de France)
// =============================================================================

/// L'Alpe d'Huez - Most famous Tour de France climb (T080)
fn alpe_dhuez() -> FamousRoute {
    FamousRoute {
        id: "alpe_dhuez".to_string(),
        name: "L'Alpe d'Huez".to_string(),
        country: RouteCountry::France,
        route_type: FamousRouteType::MountainClimb,
        distance_meters: 13800.0,
        elevation_gain_meters: 1071.0,
        avg_gradient_percent: 7.9,
        max_gradient_percent: 13.0,
        start_elevation_meters: 720.0,
        finish_elevation_meters: 1850.0,
        difficulty: RouteDifficulty::Extreme,
        description: "The most famous climb in cycling, featuring 21 legendary hairpin bends. \
            Known as the 'Dutch Mountain' due to Dutch fans who line the route."
            .to_string(),
        history: RaceHistory {
            first_raced: 1952,
            notable_races: vec!["Tour de France".to_string()],
            famous_moments: vec![
                "1986: Bernard Hinault and Greg LeMond climb together".to_string(),
                "2001: Marco Pantani's spectacular attack".to_string(),
                "2004: Lance Armstrong's iconic look at Jan Ullrich".to_string(),
            ],
            record_time: Some(RecordTime {
                time_seconds: 2194.0, // 36:34
                rider: "Marco Pantani".to_string(),
                year: 1997,
                avg_power_watts: Some(410),
            }),
        },
        elevation_profile: generate_climb_profile(13800.0, 720.0, 1850.0, 21),
        landmarks: vec![
            RouteLandmark {
                distance_meters: 0.0,
                name: "Le Bourg-d'Oisans".to_string(),
                description: "Start of the climb from the valley floor".to_string(),
            },
            RouteLandmark {
                distance_meters: 3500.0,
                name: "Hairpin 21 (La Garde)".to_string(),
                description: "First major hairpin, named for Dutch rider".to_string(),
            },
            RouteLandmark {
                distance_meters: 13800.0,
                name: "Alpe d'Huez Village".to_string(),
                description: "Finish at the ski resort".to_string(),
            },
        ],
    }
}

/// Mont Ventoux - The Beast of Provence (T081)
fn mont_ventoux() -> FamousRoute {
    FamousRoute {
        id: "mont_ventoux".to_string(),
        name: "Mont Ventoux".to_string(),
        country: RouteCountry::France,
        route_type: FamousRouteType::MountainClimb,
        distance_meters: 21500.0,
        elevation_gain_meters: 1617.0,
        avg_gradient_percent: 7.5,
        max_gradient_percent: 12.0,
        start_elevation_meters: 295.0,
        finish_elevation_meters: 1912.0,
        difficulty: RouteDifficulty::Extreme,
        description: "Known as the 'Beast of Provence' or 'Giant of Provence'. \
            The barren lunar landscape near the summit makes it uniquely challenging."
            .to_string(),
        history: RaceHistory {
            first_raced: 1951,
            notable_races: vec![
                "Tour de France".to_string(),
                "Tour de la Provence".to_string(),
            ],
            famous_moments: vec![
                "1967: Tommy Simpson's tragic death on the slopes".to_string(),
                "2000: Marco Pantani vs Lance Armstrong battle".to_string(),
                "2013: Chris Froome's dominant solo attack".to_string(),
            ],
            record_time: Some(RecordTime {
                time_seconds: 3348.0, // 55:48
                rider: "Iban Mayo".to_string(),
                year: 2004,
                avg_power_watts: Some(395),
            }),
        },
        elevation_profile: generate_climb_profile(21500.0, 295.0, 1912.0, 30),
        landmarks: vec![
            RouteLandmark {
                distance_meters: 0.0,
                name: "Bédoin".to_string(),
                description: "Most common starting point".to_string(),
            },
            RouteLandmark {
                distance_meters: 6000.0,
                name: "Saint-Estève".to_string(),
                description: "The steep section begins".to_string(),
            },
            RouteLandmark {
                distance_meters: 15000.0,
                name: "Chalet Reynard".to_string(),
                description: "Last shelter before the moonscape".to_string(),
            },
            RouteLandmark {
                distance_meters: 20000.0,
                name: "Tom Simpson Memorial".to_string(),
                description: "Memorial to the British cyclist".to_string(),
            },
            RouteLandmark {
                distance_meters: 21500.0,
                name: "Observatory Summit".to_string(),
                description: "Weather station at the peak".to_string(),
            },
        ],
    }
}

/// Col du Tourmalet
fn col_du_tourmalet() -> FamousRoute {
    FamousRoute {
        id: "col_du_tourmalet".to_string(),
        name: "Col du Tourmalet".to_string(),
        country: RouteCountry::France,
        route_type: FamousRouteType::MountainClimb,
        distance_meters: 17200.0,
        elevation_gain_meters: 1268.0,
        avg_gradient_percent: 7.4,
        max_gradient_percent: 10.0,
        start_elevation_meters: 850.0,
        finish_elevation_meters: 2115.0,
        difficulty: RouteDifficulty::Extreme,
        description: "The most climbed pass in Tour de France history. \
            Located in the Pyrenees, it has been part of the Tour since 1910."
            .to_string(),
        history: RaceHistory {
            first_raced: 1910,
            notable_races: vec!["Tour de France".to_string()],
            famous_moments: vec![
                "1910: Octave Lapize walks the final kilometers".to_string(),
                "2010: Andy Schleck attacks on the centenary ascent".to_string(),
            ],
            record_time: None,
        },
        elevation_profile: generate_climb_profile(17200.0, 850.0, 2115.0, 25),
        landmarks: vec![
            RouteLandmark {
                distance_meters: 0.0,
                name: "Luz-Saint-Sauveur".to_string(),
                description: "Start from the eastern side".to_string(),
            },
            RouteLandmark {
                distance_meters: 17200.0,
                name: "Col du Tourmalet".to_string(),
                description: "Summit with Octave Lapize statue".to_string(),
            },
        ],
    }
}

/// Col du Galibier
fn col_du_galibier() -> FamousRoute {
    FamousRoute {
        id: "col_du_galibier".to_string(),
        name: "Col du Galibier".to_string(),
        country: RouteCountry::France,
        route_type: FamousRouteType::MountainClimb,
        distance_meters: 18100.0,
        elevation_gain_meters: 1245.0,
        avg_gradient_percent: 6.9,
        max_gradient_percent: 10.0,
        start_elevation_meters: 1400.0,
        finish_elevation_meters: 2642.0,
        difficulty: RouteDifficulty::Extreme,
        description: "One of the highest paved roads in Europe. \
            Often combined with Col du Télégraphe for a legendary double climb."
            .to_string(),
        history: RaceHistory {
            first_raced: 1911,
            notable_races: vec!["Tour de France".to_string()],
            famous_moments: vec![
                "1911: First Tour de France crossing".to_string(),
                "2011: Andy Schleck's legendary attack".to_string(),
            ],
            record_time: None,
        },
        elevation_profile: generate_climb_profile(18100.0, 1400.0, 2642.0, 20),
        landmarks: vec![],
    }
}

/// Col de la Madeleine
fn col_de_la_madeleine() -> FamousRoute {
    FamousRoute {
        id: "col_de_la_madeleine".to_string(),
        name: "Col de la Madeleine".to_string(),
        country: RouteCountry::France,
        route_type: FamousRouteType::MountainClimb,
        distance_meters: 19400.0,
        elevation_gain_meters: 1520.0,
        avg_gradient_percent: 7.8,
        max_gradient_percent: 10.2,
        start_elevation_meters: 480.0,
        finish_elevation_meters: 2000.0,
        difficulty: RouteDifficulty::Extreme,
        description: "A relentless climb with consistent gradient in the French Alps.".to_string(),
        history: RaceHistory {
            first_raced: 1969,
            notable_races: vec!["Tour de France".to_string()],
            famous_moments: vec![],
            record_time: None,
        },
        elevation_profile: generate_climb_profile(19400.0, 480.0, 2000.0, 22),
        landmarks: vec![],
    }
}

/// Col d'Izoard
fn col_dizoard() -> FamousRoute {
    FamousRoute {
        id: "col_dizoard".to_string(),
        name: "Col d'Izoard".to_string(),
        country: RouteCountry::France,
        route_type: FamousRouteType::MountainClimb,
        distance_meters: 15900.0,
        elevation_gain_meters: 1105.0,
        avg_gradient_percent: 6.9,
        max_gradient_percent: 10.0,
        start_elevation_meters: 1200.0,
        finish_elevation_meters: 2360.0,
        difficulty: RouteDifficulty::Challenging,
        description: "Features the famous Casse Déserte moonscape section near the summit."
            .to_string(),
        history: RaceHistory {
            first_raced: 1922,
            notable_races: vec!["Tour de France".to_string()],
            famous_moments: vec!["1949: Fausto Coppi and Gino Bartali duel".to_string()],
            record_time: None,
        },
        elevation_profile: generate_climb_profile(15900.0, 1200.0, 2360.0, 18),
        landmarks: vec![RouteLandmark {
            distance_meters: 13000.0,
            name: "Casse Déserte".to_string(),
            description: "Iconic lunar landscape with memorial stones".to_string(),
        }],
    }
}

// =============================================================================
// ITALIAN CLIMBS (Giro d'Italia)
// =============================================================================

/// Passo dello Stelvio - Highest paved pass in Italy
fn passo_dello_stelvio() -> FamousRoute {
    FamousRoute {
        id: "passo_stelvio".to_string(),
        name: "Passo dello Stelvio".to_string(),
        country: RouteCountry::Italy,
        route_type: FamousRouteType::MountainClimb,
        distance_meters: 24300.0,
        elevation_gain_meters: 1808.0,
        avg_gradient_percent: 7.4,
        max_gradient_percent: 12.0,
        start_elevation_meters: 950.0,
        finish_elevation_meters: 2758.0,
        difficulty: RouteDifficulty::Extreme,
        description: "The highest paved road in Italy with 48 hairpin bends. \
            Known as the 'King of the Mountains' in the Giro d'Italia."
            .to_string(),
        history: RaceHistory {
            first_raced: 1953,
            notable_races: vec!["Giro d'Italia".to_string()],
            famous_moments: vec![
                "1953: First Giro crossing with Fausto Coppi".to_string(),
                "2012: Stage cancelled due to avalanche danger".to_string(),
            ],
            record_time: None,
        },
        elevation_profile: generate_climb_profile(24300.0, 950.0, 2758.0, 48),
        landmarks: vec![
            RouteLandmark {
                distance_meters: 0.0,
                name: "Prato allo Stelvio".to_string(),
                description: "Start from the valley".to_string(),
            },
            RouteLandmark {
                distance_meters: 24300.0,
                name: "Passo Stelvio".to_string(),
                description: "Summit at 2758m".to_string(),
            },
        ],
    }
}

/// Passo Gavia (T082)
fn passo_gavia() -> FamousRoute {
    FamousRoute {
        id: "passo_gavia".to_string(),
        name: "Passo Gavia".to_string(),
        country: RouteCountry::Italy,
        route_type: FamousRouteType::MountainClimb,
        distance_meters: 17300.0,
        elevation_gain_meters: 1363.0,
        avg_gradient_percent: 7.9,
        max_gradient_percent: 16.0,
        start_elevation_meters: 1225.0,
        finish_elevation_meters: 2618.0,
        difficulty: RouteDifficulty::Extreme,
        description: "A brutal Alpine pass with narrow roads and unpredictable weather. \
            Famous for the 1988 Giro stage in a blizzard."
            .to_string(),
        history: RaceHistory {
            first_raced: 1960,
            notable_races: vec!["Giro d'Italia".to_string()],
            famous_moments: vec![
                "1988: Andy Hampsten conquers the legendary blizzard stage".to_string()
            ],
            record_time: None,
        },
        elevation_profile: generate_climb_profile(17300.0, 1225.0, 2618.0, 20),
        landmarks: vec![
            RouteLandmark {
                distance_meters: 0.0,
                name: "Ponte di Legno".to_string(),
                description: "Start of the climb".to_string(),
            },
            RouteLandmark {
                distance_meters: 17300.0,
                name: "Passo Gavia".to_string(),
                description: "Summit at 2618m".to_string(),
            },
        ],
    }
}

/// Mortirolo
fn mortirolo() -> FamousRoute {
    FamousRoute {
        id: "mortirolo".to_string(),
        name: "Passo del Mortirolo".to_string(),
        country: RouteCountry::Italy,
        route_type: FamousRouteType::MountainClimb,
        distance_meters: 12400.0,
        elevation_gain_meters: 1300.0,
        avg_gradient_percent: 10.5,
        max_gradient_percent: 18.0,
        start_elevation_meters: 556.0,
        finish_elevation_meters: 1852.0,
        difficulty: RouteDifficulty::Extreme,
        description: "One of the steepest climbs in pro cycling with brutal gradients.".to_string(),
        history: RaceHistory {
            first_raced: 1990,
            notable_races: vec!["Giro d'Italia".to_string()],
            famous_moments: vec!["1994: Marco Pantani attacks and destroys the field".to_string()],
            record_time: Some(RecordTime {
                time_seconds: 2580.0, // 43:00
                rider: "Marco Pantani".to_string(),
                year: 1994,
                avg_power_watts: Some(420),
            }),
        },
        elevation_profile: generate_climb_profile(12400.0, 556.0, 1852.0, 15),
        landmarks: vec![RouteLandmark {
            distance_meters: 8000.0,
            name: "Pantani Memorial".to_string(),
            description: "Monument to the climber".to_string(),
        }],
    }
}

/// Monte Zoncolan
fn monte_zoncolan() -> FamousRoute {
    FamousRoute {
        id: "monte_zoncolan".to_string(),
        name: "Monte Zoncolan".to_string(),
        country: RouteCountry::Italy,
        route_type: FamousRouteType::MountainClimb,
        distance_meters: 10100.0,
        elevation_gain_meters: 1200.0,
        avg_gradient_percent: 11.9,
        max_gradient_percent: 22.0,
        start_elevation_meters: 580.0,
        finish_elevation_meters: 1750.0,
        difficulty: RouteDifficulty::Extreme,
        description: "One of the hardest climbs in Europe with sustained 20%+ gradients."
            .to_string(),
        history: RaceHistory {
            first_raced: 2003,
            notable_races: vec!["Giro d'Italia".to_string()],
            famous_moments: vec!["2007: Gilberto Simoni conquers the brutal climb".to_string()],
            record_time: None,
        },
        elevation_profile: generate_climb_profile(10100.0, 580.0, 1750.0, 12),
        landmarks: vec![],
    }
}

// =============================================================================
// SPANISH CLIMBS (Vuelta a España)
// =============================================================================

/// Alto de l'Angliru
fn alto_de_langliru() -> FamousRoute {
    FamousRoute {
        id: "alto_angliru".to_string(),
        name: "Alto de l'Angliru".to_string(),
        country: RouteCountry::Spain,
        route_type: FamousRouteType::MountainClimb,
        distance_meters: 12500.0,
        elevation_gain_meters: 1266.0,
        avg_gradient_percent: 10.1,
        max_gradient_percent: 23.5,
        start_elevation_meters: 308.0,
        finish_elevation_meters: 1570.0,
        difficulty: RouteDifficulty::Extreme,
        description: "The most brutal climb in Grand Tour racing with 23.5% maximum gradient."
            .to_string(),
        history: RaceHistory {
            first_raced: 1999,
            notable_races: vec!["Vuelta a España".to_string()],
            famous_moments: vec![
                "1999: First ever professional ascent in the Vuelta".to_string(),
                "2017: Alberto Contador's emotional final climb".to_string(),
            ],
            record_time: None,
        },
        elevation_profile: generate_climb_profile(12500.0, 308.0, 1570.0, 14),
        landmarks: vec![RouteLandmark {
            distance_meters: 9000.0,
            name: "Cueña les Cabres".to_string(),
            description: "The steepest section at 23.5%".to_string(),
        }],
    }
}

/// Lagos de Covadonga
fn lagos_de_covadonga() -> FamousRoute {
    FamousRoute {
        id: "lagos_covadonga".to_string(),
        name: "Lagos de Covadonga".to_string(),
        country: RouteCountry::Spain,
        route_type: FamousRouteType::MountainClimb,
        distance_meters: 12600.0,
        elevation_gain_meters: 1025.0,
        avg_gradient_percent: 8.1,
        max_gradient_percent: 15.0,
        start_elevation_meters: 85.0,
        finish_elevation_meters: 1105.0,
        difficulty: RouteDifficulty::Challenging,
        description: "A beautiful climb to glacial lakes in the Picos de Europa.".to_string(),
        history: RaceHistory {
            first_raced: 1983,
            notable_races: vec!["Vuelta a España".to_string()],
            famous_moments: vec!["1983: First Vuelta summit finish".to_string()],
            record_time: None,
        },
        elevation_profile: generate_climb_profile(12600.0, 85.0, 1105.0, 14),
        landmarks: vec![],
    }
}

// =============================================================================
// BELGIAN CLASSICS
// =============================================================================

/// Mur de Huy
fn mur_de_huy() -> FamousRoute {
    FamousRoute {
        id: "mur_de_huy".to_string(),
        name: "Mur de Huy".to_string(),
        country: RouteCountry::Belgium,
        route_type: FamousRouteType::ClassicRace,
        distance_meters: 1300.0,
        elevation_gain_meters: 130.0,
        avg_gradient_percent: 10.0,
        max_gradient_percent: 26.0,
        start_elevation_meters: 80.0,
        finish_elevation_meters: 210.0,
        difficulty: RouteDifficulty::Challenging,
        description: "The iconic finishing climb of Flèche Wallonne with brutal final ramp."
            .to_string(),
        history: RaceHistory {
            first_raced: 1983,
            notable_races: vec!["Flèche Wallonne".to_string()],
            famous_moments: vec!["Multiple wins by Alejandro Valverde".to_string()],
            record_time: None,
        },
        elevation_profile: generate_climb_profile(1300.0, 80.0, 210.0, 5),
        landmarks: vec![RouteLandmark {
            distance_meters: 1000.0,
            name: "Chapel".to_string(),
            description: "The steepest section begins".to_string(),
        }],
    }
}

/// Koppenberg
fn koppenberg() -> FamousRoute {
    FamousRoute {
        id: "koppenberg".to_string(),
        name: "Koppenberg".to_string(),
        country: RouteCountry::Belgium,
        route_type: FamousRouteType::Cobbles,
        distance_meters: 600.0,
        elevation_gain_meters: 64.0,
        avg_gradient_percent: 11.6,
        max_gradient_percent: 22.0,
        start_elevation_meters: 20.0,
        finish_elevation_meters: 84.0,
        difficulty: RouteDifficulty::Challenging,
        description: "Notorious cobbled climb, so steep riders often walk.".to_string(),
        history: RaceHistory {
            first_raced: 1977,
            notable_races: vec!["Tour of Flanders".to_string()],
            famous_moments: vec!["1987: Jesper Skibby crashes on the cobbles".to_string()],
            record_time: None,
        },
        elevation_profile: generate_climb_profile(600.0, 20.0, 84.0, 4),
        landmarks: vec![],
    }
}

/// Oude Kwaremont
fn oude_kwaremont() -> FamousRoute {
    FamousRoute {
        id: "oude_kwaremont".to_string(),
        name: "Oude Kwaremont".to_string(),
        country: RouteCountry::Belgium,
        route_type: FamousRouteType::Cobbles,
        distance_meters: 2200.0,
        elevation_gain_meters: 89.0,
        avg_gradient_percent: 4.0,
        max_gradient_percent: 11.0,
        start_elevation_meters: 25.0,
        finish_elevation_meters: 114.0,
        difficulty: RouteDifficulty::Moderate,
        description: "Long cobbled climb, key selection point in Tour of Flanders.".to_string(),
        history: RaceHistory {
            first_raced: 1974,
            notable_races: vec!["Tour of Flanders".to_string(), "E3 Harelbeke".to_string()],
            famous_moments: vec![],
            record_time: None,
        },
        elevation_profile: generate_climb_profile(2200.0, 25.0, 114.0, 6),
        landmarks: vec![],
    }
}

/// Paterberg
fn paterberg() -> FamousRoute {
    FamousRoute {
        id: "paterberg".to_string(),
        name: "Paterberg".to_string(),
        country: RouteCountry::Belgium,
        route_type: FamousRouteType::Cobbles,
        distance_meters: 360.0,
        elevation_gain_meters: 44.0,
        avg_gradient_percent: 12.9,
        max_gradient_percent: 20.0,
        start_elevation_meters: 40.0,
        finish_elevation_meters: 84.0,
        difficulty: RouteDifficulty::Challenging,
        description: "Short, brutal cobbled climb often decisive in Flanders races.".to_string(),
        history: RaceHistory {
            first_raced: 1986,
            notable_races: vec!["Tour of Flanders".to_string()],
            famous_moments: vec![],
            record_time: None,
        },
        elevation_profile: generate_climb_profile(360.0, 40.0, 84.0, 3),
        landmarks: vec![],
    }
}

// =============================================================================
// OTHER NOTABLE CLIMBS
// =============================================================================

/// Sa Calobra (Mallorca)
fn sa_calobra() -> FamousRoute {
    FamousRoute {
        id: "sa_calobra".to_string(),
        name: "Sa Calobra".to_string(),
        country: RouteCountry::Spain,
        route_type: FamousRouteType::MountainClimb,
        distance_meters: 9400.0,
        elevation_gain_meters: 682.0,
        avg_gradient_percent: 7.3,
        max_gradient_percent: 12.0,
        start_elevation_meters: 0.0,
        finish_elevation_meters: 682.0,
        difficulty: RouteDifficulty::Challenging,
        description: "Popular training climb in Mallorca with famous 270-degree hairpin."
            .to_string(),
        history: RaceHistory {
            first_raced: 0,
            notable_races: vec!["Challenge Mallorca".to_string()],
            famous_moments: vec![],
            record_time: None,
        },
        elevation_profile: generate_climb_profile(9400.0, 0.0, 682.0, 12),
        landmarks: vec![RouteLandmark {
            distance_meters: 5000.0,
            name: "The Knot".to_string(),
            description: "Famous 270-degree hairpin".to_string(),
        }],
    }
}

/// Grossglockner
fn grossglockner() -> FamousRoute {
    FamousRoute {
        id: "grossglockner".to_string(),
        name: "Grossglockner".to_string(),
        country: RouteCountry::Austria,
        route_type: FamousRouteType::MountainClimb,
        distance_meters: 16900.0,
        elevation_gain_meters: 1512.0,
        avg_gradient_percent: 9.0,
        max_gradient_percent: 12.0,
        start_elevation_meters: 989.0,
        finish_elevation_meters: 2503.0,
        difficulty: RouteDifficulty::Extreme,
        description: "Austria's most famous mountain pass with stunning alpine views.".to_string(),
        history: RaceHistory {
            first_raced: 1971,
            notable_races: vec!["Tour of Austria".to_string()],
            famous_moments: vec![],
            record_time: None,
        },
        elevation_profile: generate_climb_profile(16900.0, 989.0, 2503.0, 20),
        landmarks: vec![],
    }
}

/// Col de Joux Plane
fn col_de_joux_plane() -> FamousRoute {
    FamousRoute {
        id: "col_joux_plane".to_string(),
        name: "Col de Joux Plane".to_string(),
        country: RouteCountry::France,
        route_type: FamousRouteType::MountainClimb,
        distance_meters: 11600.0,
        elevation_gain_meters: 979.0,
        avg_gradient_percent: 8.5,
        max_gradient_percent: 11.5,
        start_elevation_meters: 692.0,
        finish_elevation_meters: 1691.0,
        difficulty: RouteDifficulty::Challenging,
        description: "Tough Alpine climb often used in the Tour de France.".to_string(),
        history: RaceHistory {
            first_raced: 1978,
            notable_races: vec!["Tour de France".to_string()],
            famous_moments: vec!["2000: Lance Armstrong cracks here".to_string()],
            record_time: None,
        },
        elevation_profile: generate_climb_profile(11600.0, 692.0, 1691.0, 14),
        landmarks: vec![],
    }
}

/// Col de la Croix de Fer
fn col_de_la_croix_de_fer() -> FamousRoute {
    FamousRoute {
        id: "col_croix_de_fer".to_string(),
        name: "Col de la Croix de Fer".to_string(),
        country: RouteCountry::France,
        route_type: FamousRouteType::MountainClimb,
        distance_meters: 22400.0,
        elevation_gain_meters: 1500.0,
        avg_gradient_percent: 6.7,
        max_gradient_percent: 10.0,
        start_elevation_meters: 560.0,
        finish_elevation_meters: 2067.0,
        difficulty: RouteDifficulty::Extreme,
        description: "Long climb in the French Alps, often paired with Col du Glandon.".to_string(),
        history: RaceHistory {
            first_raced: 1947,
            notable_races: vec!["Tour de France".to_string()],
            famous_moments: vec![],
            record_time: None,
        },
        elevation_profile: generate_climb_profile(22400.0, 560.0, 2067.0, 25),
        landmarks: vec![RouteLandmark {
            distance_meters: 22400.0,
            name: "Iron Cross".to_string(),
            description: "Summit marked with an iron cross".to_string(),
        }],
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Generate a realistic elevation profile for a climb
fn generate_climb_profile(
    distance: f64,
    start_elev: f32,
    end_elev: f32,
    num_points: usize,
) -> Vec<(f64, f32)> {
    let mut profile = Vec::with_capacity(num_points);
    let elev_gain = end_elev - start_elev;

    for i in 0..num_points {
        let t = i as f64 / (num_points - 1) as f64;
        let dist = t * distance;

        // Add some variation to make it realistic
        let base_elev = start_elev + (elev_gain * t as f32);
        let variation = (t * std::f64::consts::PI * 4.0).sin() as f32 * (elev_gain * 0.02);
        let elev = base_elev + variation;

        profile.push((dist, elev));
    }

    profile
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_famous_routes_count() {
        let routes = get_all_famous_routes();
        assert!(routes.len() >= 20, "Should have at least 20 famous routes");
    }

    #[test]
    fn test_alpe_dhuez() {
        let route = alpe_dhuez();
        assert_eq!(route.id, "alpe_dhuez");
        assert!((route.distance_meters - 13800.0).abs() < 100.0);
        assert!(route.avg_gradient_percent > 7.0);
    }

    #[test]
    fn test_library_search() {
        let lib = FamousRoutesLibrary::new();
        let results = lib.search("ventoux");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "mont_ventoux");
    }

    #[test]
    fn test_library_by_country() {
        let lib = FamousRoutesLibrary::new();
        let french = lib.by_country(RouteCountry::France);
        assert!(french.len() >= 6, "Should have at least 6 French climbs");
    }

    #[test]
    fn test_to_route_definition() {
        let route = alpe_dhuez();
        let def = route.to_route_definition();
        assert_eq!(def.id, "alpe_dhuez");
        assert_eq!(def.name, "L'Alpe d'Huez");
    }

    #[test]
    fn test_to_stored_route() {
        let route = alpe_dhuez();
        let stored = route.to_stored_route();

        assert_eq!(stored.name, "L'Alpe d'Huez");
        assert_eq!(stored.source, RouteSource::Famous);
        assert!((stored.distance_meters - 13800.0).abs() < 100.0);
        assert!(stored.elevation_gain_meters > 1000.0);
        assert!(stored.description.is_some());

        // Verify deterministic UUID generation
        let stored2 = route.to_stored_route();
        assert_eq!(stored.id, stored2.id, "Same route should produce same UUID");
    }

    #[test]
    fn test_library_as_stored_routes() {
        let lib = FamousRoutesLibrary::new();
        let stored_routes = lib.as_stored_routes();

        assert!(
            stored_routes.len() >= 20,
            "Should have at least 20 famous routes as StoredRoutes"
        );

        // Verify all are marked as Famous source
        for route in &stored_routes {
            assert_eq!(route.source, RouteSource::Famous);
        }
    }
}
