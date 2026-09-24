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
    #[serde(default, rename = "userEmail")]
    pub user_email: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub name: String,
    pub avatar: String,
    pub tagline: String,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserAccount {
    pub id: String,
    pub email: String,
    pub name: String,
    pub avatar: String,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default, rename = "createdAt")]
    pub created_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct AppData {
    pub sessions: Vec<ToiletSession>,
    pub profiles: Vec<UserProfile>,
    #[serde(default)]
    pub accounts: Vec<UserAccount>,
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
                Ok(content) => {
                    let mut data: AppData = serde_json::from_str(&content).unwrap_or_else(|_| Self::default_data());
                    let has_mock = data.sessions.iter().any(|s| s.id.starts_with("sess-"))
                        || data.profiles.iter().any(|p| p.id == "bram" || p.id == "thijs" || p.id == "lisa" || p.id == "daan" || p.id == "sanne");
                    if has_mock {
                        data.sessions.retain(|s| !s.id.starts_with("sess-"));
                        data.profiles.retain(|p| p.id != "bram" && p.id != "thijs" && p.id != "lisa" && p.id != "daan" && p.id != "sanne");
                    }
                    // Auto-sync: ensure any user with sessions exists in profiles
                    for s in &data.sessions {
                        let exists = data.profiles.iter().any(|p| p.id == s.user_id || p.name.to_lowercase() == s.user_name.to_lowercase());
                        if !exists {
                            data.profiles.push(UserProfile {
                                id: s.user_id.clone(),
                                name: s.user_name.clone(),
                                avatar: s.user_avatar.clone(),
                                tagline: "Geregistreerde Poeper".into(),
                                email: s.user_email.clone(),
                            });
                        }
                    }
                    let _ = fs::write(&db_path, serde_json::to_string_pretty(&data).unwrap());
                    data
                }
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
        AppData {
            sessions: Vec::new(),
            profiles: Vec::new(),
            accounts: Vec::new(),
        }
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
    println!("  POEPENDE BRAM -- ULTRA-LICHTE RUST SERVER");
    println!("================================================================================");
    println!("  Server gestart via tiny_http (Geheugengebruik: ~8 MB RAM)");
    println!("  Frontend web bestanden geserveerd vanuit: {:?}", dist_dir);
    println!("--------------------------------------------------------------------------------");
    println!("  Lokaal:      http://localhost:{}", port);
    println!("  Netwerk IP:  http://{}:{}", local_ip_str, port);
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
                        r#"{"status":"ok","app":"Poepende Bram","server":"Rust (Ultra-Lightweight)","version":"1.0.0"}"#,
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
                                // Auto-sync: if session user not in profiles, add them!
                                let prof_exists = guard.profiles.iter().any(|p| p.id == session.user_id || p.name.to_lowercase() == session.user_name.to_lowercase());
                                if !prof_exists {
                                    guard.profiles.push(UserProfile {
                                        id: session.user_id.clone(),
                                        name: session.user_name.clone(),
                                        avatar: session.user_avatar.clone(),
                                        tagline: "Geregistreerde Poeper".into(),
                                        email: session.user_email.clone(),
                                    });
                                }
                                guard.sessions.retain(|s| s.id != session.id);
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
                                if let Some(existing) = guard.profiles.iter_mut().find(|p| p.id == profile.id || p.name.to_lowercase() == profile.name.to_lowercase()) {
                                    *existing = profile;
                                } else {
                                    guard.profiles.push(profile);
                                }
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

                // REST API: /api/accounts
                if path_part == "/api/accounts" {
                    match request.method() {
                        &Method::Get => {
                            let json = {
                                let guard = state.data.read().unwrap();
                                serde_json::to_string(&guard.accounts).unwrap_or_else(|_| "[]".into())
                            };
                            let resp = json_response(&json, 200);
                            let _ = request.respond(resp);
                            continue;
                        }
                        &Method::Post => {
                            let mut body = String::new();
                            let _ = request.as_reader().read_to_string(&mut body);
                            if let Ok(account) = serde_json::from_str::<UserAccount>(&body) {
                                let mut guard = state.data.write().unwrap();
                                let email_lower = account.email.to_lowercase();
                                if guard.accounts.iter().any(|a| a.email.to_lowercase() == email_lower) {
                                    let resp = json_response(r#"{"success":false,"error":"Er bestaat al een account met dit e-mailadres"}"#, 400);
                                    let _ = request.respond(resp);
                                    continue;
                                }
                                guard.accounts.push(account.clone());
                                let prof_exists = guard.profiles.iter().any(|p| p.id == account.id || p.name.to_lowercase() == account.name.to_lowercase());
                                if !prof_exists {
                                    guard.profiles.push(UserProfile {
                                        id: account.id.clone(),
                                        name: account.name.clone(),
                                        avatar: account.avatar.clone(),
                                        tagline: "Geregistreerde Poeper".into(),
                                        email: Some(account.email.clone()),
                                    });
                                }
                                drop(guard);
                                state.persist();
                                let json = serde_json::to_string(&serde_json::json!({
                                    "success": true,
                                    "account": account,
                                })).unwrap_or_else(|_| "{}".into());
                                let resp = json_response(&json, 201);
                                let _ = request.respond(resp);
                                continue;
                            }
                        }
                        _ => {}
                    }
                }

                // REST API: /api/login
                if path_part == "/api/login" && request.method() == &Method::Post {
                    let mut body = String::new();
                    let _ = request.as_reader().read_to_string(&mut body);
                    #[derive(Deserialize)]
                    struct LoginReq {
                        email: String,
                        password: Option<String>,
                    }
                    if let Ok(login_req) = serde_json::from_str::<LoginReq>(&body) {
                        let guard = state.data.read().unwrap();
                        let email_lower = login_req.email.trim().to_lowercase();
                        if let Some(account) = guard.accounts.iter().find(|a| a.email.to_lowercase() == email_lower) {
                            if let Some(ref acc_pwd) = account.password {
                                if let Some(ref req_pwd) = login_req.password {
                                    if acc_pwd != req_pwd {
                                        let resp = json_response(r#"{"success":false,"error":"Onjuist wachtwoord"}"#, 401);
                                        let _ = request.respond(resp);
                                        continue;
                                    }
                                }
                            }
                            let profile = guard.profiles.iter().find(|p| p.id == account.id || p.name.to_lowercase() == account.name.to_lowercase()).cloned();
                            let json = serde_json::to_string(&serde_json::json!({
                                "success": true,
                                "account": account,
                                "profile": profile,
                            })).unwrap_or_else(|_| "{}".into());
                            let resp = json_response(&json, 200);
                            let _ = request.respond(resp);
                            continue;
                        } else {
                            let resp = json_response(r#"{"success":false,"error":"Geen account gevonden met dit e-mailadres"}"#, 404);
                            let _ = request.respond(resp);
                            continue;
                        }
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
