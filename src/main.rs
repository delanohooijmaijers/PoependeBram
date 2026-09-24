use local_ip_address::local_ip;
use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::{self, File},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    thread,
};
use tiny_http::{Header, Method, Response, Server, StatusCode};

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

fn get_mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|s| s.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "ttf" => "font/ttf",
        _ => "application/octet-stream",
    }
}

fn json_response(body: &str, status: u16) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_data(body.as_bytes().to_vec())
        .with_status_code(StatusCode(status))
        .with_header(Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..]).unwrap())
        .with_header(Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap())
        .with_header(Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, DELETE, OPTIONS"[..]).unwrap())
        .with_header(Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"*"[..]).unwrap())
}

fn main() {
    let port: u16 = env::var("SERVER_PORT")
        .or_else(|_| env::var("PORT"))
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let dist_dir = PathBuf::from("dist");
    let db_path = PathBuf::from("data/database.json");
    let state = AppState::new(db_path);

    let bind_addr = format!("0.0.0.0:{}", port);
    let server = Server::http(&bind_addr).expect("Kan niet binden aan poort");
    let server = Arc::new(server);

    let local_ip_str = local_ip().map(|ip| ip.to_string()).unwrap_or_else(|_| "127.0.0.1".to_string());

    println!("\n================================================================================");
    println!("  💩 POEPENDE BRAM — ULTRA-LICHTE RUST SERVER 🦀⚡");
    println!("================================================================================");
    println!("  🚀 Server gestart via tiny_http (Geheugengebruik: ~8 MB RAM)");
    println!("  📁 Frontend web bestanden geserveerd vanuit: {:?}", dist_dir);
    println!("--------------------------------------------------------------------------------");
    println!("  💻 Lokaal:      http://localhost:{}", port);
    println!("  📱 Netwerk IP:  http://{}:{}", local_ip_str, port);
    println!("================================================================================");
    // Pterodactyl console detection string
    println!("POEPENDE BRAM SERVER IS READY!");
    println!("================================================================================\n");

    // Spawn 4 worker threads for handling concurrent HTTP requests
    let num_threads = 4;
    let mut handles = Vec::new();

    for _ in 0..num_threads {
        let server = Arc::clone(&server);
        let state = state.clone();
        let dist_dir = dist_dir.clone();

        let handle = thread::spawn(move || {
            for mut request in server.incoming_requests() {
                let url = request.url().to_string();
                let path_part = url.split('?').next().unwrap_or("/");

                // Handle CORS preflight
                if request.method() == &Method::Options {
                    let resp = Response::empty(200)
                        .with_header(Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap())
                        .with_header(Header::from_bytes(&b"Access-Control-Allow-Methods"[..], &b"GET, POST, DELETE, OPTIONS"[..]).unwrap())
                        .with_header(Header::from_bytes(&b"Access-Control-Allow-Headers"[..], &b"*"[..]).unwrap());
                    let _ = request.respond(resp);
                    continue;
                }

                // REST API: /api/health
                if path_part == "/api/health" && request.method() == &Method::Get {
                    let resp = json_response(
                        r#"{"status":"ok","app":"Poepende Bram","server":"Rust (Ultra-Lightweight ⚡) 🦀","version":"1.0.0"}"#,
                        200,
                    );
                    let _ = request.respond(resp);
                    continue;
                }

                // REST API: /api/sessions
                if path_part == "/api/sessions" {
                    match request.method() {
                        &Method::Get => {
                            let json = {
                                let guard = state.data.read().unwrap();
                                serde_json::to_string(&guard.sessions).unwrap_or_else(|_| "[]".into())
                            };
                            let resp = json_response(&json, 200);
                            let _ = request.respond(resp);
                            continue;
                        }
                        &Method::Post => {
                            let mut body = String::new();
                            let _ = request.as_reader().read_to_string(&mut body);
                            if let Ok(session) = serde_json::from_str::<ToiletSession>(&body) {
                                let mut guard = state.data.write().unwrap();
                                guard.sessions.insert(0, session);
                                let list = guard.sessions.clone();
                                drop(guard);
                                state.persist();
                                let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".into());
                                let resp = json_response(&json, 201);
                                let _ = request.respond(resp);
                                continue;
                            }
                        }
                        _ => {}
                    }
                }

                // REST API: DELETE /api/sessions/:id
                if path_part.starts_with("/api/sessions/") && request.method() == &Method::Delete {
                    let id = path_part.trim_start_matches("/api/sessions/");
                    let mut guard = state.data.write().unwrap();
                    guard.sessions.retain(|s| s.id != id);
                    let list = guard.sessions.clone();
                    drop(guard);
                    state.persist();
                    let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".into());
                    let resp = json_response(&json, 200);
                    let _ = request.respond(resp);
                    continue;
                }

                // REST API: /api/profiles
                if path_part == "/api/profiles" {
                    match request.method() {
                        &Method::Get => {
                            let json = {
                                let guard = state.data.read().unwrap();
                                serde_json::to_string(&guard.profiles).unwrap_or_else(|_| "[]".into())
                            };
                            let resp = json_response(&json, 200);
                            let _ = request.respond(resp);
                            continue;
                        }
                        &Method::Post => {
                            let mut body = String::new();
                            let _ = request.as_reader().read_to_string(&mut body);
                            if let Ok(profile) = serde_json::from_str::<UserProfile>(&body) {
                                let mut guard = state.data.write().unwrap();
                                guard.profiles.push(profile);
                                let list = guard.profiles.clone();
                                drop(guard);
                                state.persist();
                                let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".into());
                                let resp = json_response(&json, 201);
                                let _ = request.respond(resp);
                                continue;
                            }
                        }
                        _ => {}
                    }
                }

                // REST API: /api/reset
                if path_part == "/api/reset" && request.method() == &Method::Post {
                    let default = AppState::default_data();
                    let mut guard = state.data.write().unwrap();
                    *guard = default;
                    drop(guard);
                    state.persist();
                    let resp = json_response(r#"{"status":"reset_successful"}"#, 200);
                    let _ = request.respond(resp);
                    continue;
                }

                // Static file serving from ./dist
                let sanitized_path = path_part.trim_start_matches('/');
                let target_file = dist_dir.join(sanitized_path);

                let (file_to_serve, mime) = if target_file.is_file() {
                    let mime = get_mime_type(&target_file);
                    (target_file, mime)
                } else {
                    // SPA fallback: index.html
                    let index_file = dist_dir.join("index.html");
                    (index_file, "text/html; charset=utf-8")
                };

                if let Ok(file) = File::open(&file_to_serve) {
                    let resp = Response::from_file(file)
                        .with_header(Header::from_bytes(&b"Content-Type"[..], mime.as_bytes()).unwrap())
                        .with_header(Header::from_bytes(&b"Access-Control-Allow-Origin"[..], &b"*"[..]).unwrap());
                    let _ = request.respond(resp);
                } else {
                    let resp = Response::from_string("Poepende Bram Server draait! dist/ niet gevonden.")
                        .with_status_code(StatusCode(404));
                    let _ = request.respond(resp);
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.join();
    }
}
