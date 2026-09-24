use axum::{
    extract::{Path, State},
    http::{header, HeaderValue, Method, StatusCode},
    response::{Html, IntoResponse, Json},
    routing::{delete, get, post},
    Router,
};
use local_ip_address::local_ip;
use serde::{Deserialize, Serialize};
use std::{
    env,
    fs,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, RwLock},
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToiletSession {
    pub id: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "userName")]
    pub user_name: String,
    #[serde(rename = "userAvatar")]
    pub user_avatar: String,
    #[serde(rename = "locationName")]
    pub location_name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timestamp: String,
    #[serde(rename = "durationSeconds")]
    pub duration_seconds: u32,
    pub rating: u8,
    #[serde(rename = "poopType")]
    pub poop_type: Option<String>,
    pub notes: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub name: String,
    pub avatar: String,
    pub tagline: String,
}

#[derive(Serialize, Deserialize)]
pub struct AppData {
    pub sessions: Vec<ToiletSession>,
    pub profiles: Vec<UserProfile>,
}

#[derive(Clone)]
pub struct AppState {
    pub data: Arc<RwLock<AppData>>,
    pub db_path: PathBuf,
}

impl AppState {
    fn new(db_path: PathBuf) -> Self {
        let initial_data = if db_path.exists() {
            match fs::read_to_string(&db_path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| Self::default_data()),
                Err(_) => Self::default_data(),
            }
        } else {
            let data = Self::default_data();
            if let Some(parent) = db_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&db_path, serde_json::to_string_pretty(&data).unwrap());
            data
        };

        Self {
            data: Arc::new(RwLock::new(initial_data)),
            db_path,
        }
    }

    fn persist(&self) {
        if let Ok(guard) = self.data.read() {
            if let Ok(json) = serde_json::to_string_pretty(&*guard) {
                let _ = fs::write(&self.db_path, json);
            }
        }
    }

    fn default_data() -> AppData {
        let profiles = vec![
            UserProfile {
                id: "bram".into(),
                name: "Bram".into(),
                avatar: "👑".into(),
                tagline: "Opperpoeper & Troonmeester".into(),
            },
            UserProfile {
                id: "thijs".into(),
                name: "Thijs".into(),
                avatar: "⚡".into(),
                tagline: "De Snelle Sprinter".into(),
            },
            UserProfile {
                id: "lisa".into(),
                name: "Lisa".into(),
                avatar: "🌸".into(),
                tagline: "Zen Troonzitter".into(),
            },
            UserProfile {
                id: "daan".into(),
                name: "Daan".into(),
                avatar: "🚀".into(),
                tagline: "Marathon Scheter".into(),
            },
            UserProfile {
                id: "sanne".into(),
                name: "Sanne".into(),
                avatar: "✨".into(),
                tagline: "Koninklijke Bezoeker".into(),
            },
        ];

        let sessions = vec![
            ToiletSession {
                id: "sess-1".into(),
                user_id: "bram".into(),
                user_name: "Bram".into(),
                user_avatar: "👑".into(),
                location_name: "Bram's Heilige Troon (Thuis)".into(),
                latitude: 52.3676,
                longitude: 4.9041,
                timestamp: "2026-09-24T16:30:00Z".into(),
                duration_seconds: 1692, // 28m 12s - RECORD LONGEST!
                rating: 5,
                poop_type: Some("Koninklijk".into()),
                notes: Some("Volledig YouTube gekeken. Mijn benen sliepen helemaal toen ik opstond. Absolute topervaring.".into()),
                tags: Some(vec!["Zacht papier".into(), "Stilte".into(), "Koninklijk comfort".into()]),
            },
            ToiletSession {
                id: "sess-2".into(),
                user_id: "thijs".into(),
                user_name: "Thijs".into(),
                user_avatar: "⚡".into(),
                location_name: "Station Utrecht Centraal WC".into(),
                latitude: 52.0894,
                longitude: 5.1103,
                timestamp: "2026-09-24T01:30:00Z".into(),
                duration_seconds: 42, // 42s - RECORD SHORTEST!
                rating: 2,
                poop_type: Some("Soepel & Razendsnel".into()),
                notes: Some("Kostte €0,70 bij Sanifair, maar binnen 42 seconden stond ik alweer op spoor 5!".into()),
                tags: Some(vec!["Haast".into(), "Duur toilet".into()]),
            },
            ToiletSession {
                id: "sess-3".into(),
                user_id: "lisa".into(),
                user_name: "Lisa".into(),
                user_avatar: "🌸".into(),
                location_name: "Kantoor Zuidas (12e Verdieping)".into(),
                latitude: 52.3364,
                longitude: 4.8732,
                timestamp: "2026-09-23T15:00:00Z".into(),
                duration_seconds: 495,
                rating: 4,
                poop_type: Some("De Vlotte Boodschap".into()),
                notes: Some("Heerlijke rust op de 12e. 3-laags papier en uitzicht over de stad.".into()),
                tags: Some(vec!["Kantoortijd".into(), "Zacht papier".into()]),
            },
            ToiletSession {
                id: "sess-4".into(),
                user_id: "daan".into(),
                user_name: "Daan".into(),
                user_avatar: "🚀".into(),
                location_name: "Basic-Fit Kleedkamer WC".into(),
                latitude: 52.3702,
                longitude: 4.8952,
                timestamp: "2026-09-23T01:00:00Z".into(),
                duration_seconds: 98,
                rating: 3,
                poop_type: Some("Explosief Avontuur".into()),
                notes: Some("De pre-workout shake deed zijn werk iets te snel.".into()),
                tags: Some(vec!["Pre-workout".into(), "Gym life".into()]),
            },
        ];

        AppData { sessions, profiles }
    }
}

// REST API Handlers
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "app": "Poepende Bram",
        "server": "Rust + Axum (Pterodactyl Ready) 🦀",
        "version": "1.0.0"
    }))
}

async fn get_sessions(State(state): State<AppState>) -> Json<Vec<ToiletSession>> {
    let guard = state.data.read().unwrap();
    Json(guard.sessions.clone())
}

async fn add_session(
    State(state): State<AppState>,
    Json(session): Json<ToiletSession>,
) -> (StatusCode, Json<Vec<ToiletSession>>) {
    let mut guard = state.data.write().unwrap();
    guard.sessions.insert(0, session);
    let list = guard.sessions.clone();
    drop(guard);
    state.persist();
    (StatusCode::CREATED, Json(list))
}

async fn delete_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<Vec<ToiletSession>> {
    let mut guard = state.data.write().unwrap();
    guard.sessions.retain(|s| s.id != id);
    let list = guard.sessions.clone();
    drop(guard);
    state.persist();
    Json(list)
}

async fn get_profiles(State(state): State<AppState>) -> Json<Vec<UserProfile>> {
    let guard = state.data.read().unwrap();
    Json(guard.profiles.clone())
}

async fn add_profile(
    State(state): State<AppState>,
    Json(profile): Json<UserProfile>,
) -> (StatusCode, Json<Vec<UserProfile>>) {
    let mut guard = state.data.write().unwrap();
    guard.profiles.push(profile);
    let list = guard.profiles.clone();
    drop(guard);
    state.persist();
    (StatusCode::CREATED, Json(list))
}

async fn reset_data(State(state): State<AppState>) -> Json<serde_json::Value> {
    let default = AppState::default_data();
    let mut guard = state.data.write().unwrap();
    *guard = default;
    drop(guard);
    state.persist();
    Json(serde_json::json!({ "status": "reset_successful" }))
}

// Single Page Application (SPA) HTML fallback
async fn spa_fallback() -> impl IntoResponse {
    let dist_index = PathBuf::from("dist/index.html");
    match fs::read_to_string(&dist_index) {
        Ok(html) => (
            [(header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8"))],
            Html(html),
        ),
        Err(_) => (
            [(header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8"))],
            Html("<h1>Poepende Bram Server is online!</h1><p>dist/index.html kon niet worden geladen.</p>".to_string()),
        ),
    }
}

#[tokio::main]
async fn main() {
    // Read dynamic port from Pterodactyl (SERVER_PORT or PORT, default 8080)
    let port: u16 = env::var("SERVER_PORT")
        .or_else(|_| env::var("PORT"))
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let dist_dir = PathBuf::from("dist");
    let db_path = PathBuf::from("data/database.json");

    let state = AppState::new(db_path);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers(Any);

    let api_routes = Router::new()
        .route("/health", get(health_check))
        .route("/sessions", get(get_sessions).post(add_session))
        .route("/sessions/:id", delete(delete_session))
        .route("/profiles", get(get_profiles).post(add_profile))
        .route("/reset", post(reset_data))
        .with_state(state.clone());

    let app = Router::new()
        .nest("/api", api_routes)
        .nest_service("/", ServeDir::new(&dist_dir).fallback(axum::routing::get(spa_fallback)))
        .fallback(spa_fallback)
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let local_ip_str = local_ip().map(|ip| ip.to_string()).unwrap_or_else(|_| "127.0.0.1".to_string());

    println!("\n================================================================================");
    println!("  💩 POEPENDE BRAM — PTERODACTYL RUST SERVER 🦀");
    println!("================================================================================");
    println!("  🚀 Server gestart via Cargo op poort {}", port);
    println!("  📁 Frontend web bestanden geserveerd vanuit: {:?}", dist_dir);
    println!("--------------------------------------------------------------------------------");
    println!("  💻 Lokaal:      http://localhost:{}", port);
    println!("  📱 Netwerk IP:  http://{}:{}", local_ip_str, port);
    println!("================================================================================");
    // Explicit completion message for Pterodactyl Startup Detection ("done")
    println!("POEPENDE BRAM SERVER IS READY!");
    println!("================================================================================\n");

    let listener = tokio::net::TcpListener::bind(addr).await.expect("Kan niet binden aan poort");
    axum::serve(listener, app).await.expect("Fout bij uitvoeren van server");
}
