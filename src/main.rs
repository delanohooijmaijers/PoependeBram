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
    #[serde(default, skip_serializing)]
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
                    // Auto-sync: ensure all registered accounts have a profile with matching id and email
                    for a in &data.accounts {
                        if let Some(p) = data.profiles.iter_mut().find(|p| p.id == a.id || p.name.to_lowercase() == a.name.to_lowercase() || p.email.as_ref().map(|e| e.to_lowercase()) == Some(a.email.to_lowercase())) {
                            p.id = a.id.clone();
                            p.name = a.name.clone();
                            p.avatar = a.avatar.clone();
                            p.email = Some(a.email.clone());
                        } else {
                            data.profiles.push(UserProfile {
                                id: a.id.clone(),
                                name: a.name.clone(),
                                avatar: a.avatar.clone(),
                                tagline: "Geregistreerde Poeper".into(),
                                email: Some(a.email.clone()),
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

fn parse_dutch_date_to_iso(raw: &str) -> String {
    let raw = raw.trim();
    if raw.is_empty() {
        return "2024-01-01T12:00:00.000Z".to_string();
    }
    if raw.contains('T') {
        return raw.to_string();
    }

    let parts: Vec<&str> = raw.split_whitespace().collect();
    let date_str = parts.first().copied().unwrap_or(raw);
    let time_str = if parts.len() > 1 {
        let t = parts[1];
        if t.len() == 5 {
            format!("{}:00", t)
        } else {
            t.to_string()
        }
    } else {
        "12:00:00".to_string()
    };

    let delimiter = if date_str.contains('-') {
        '-'
    } else if date_str.contains('/') {
        '/'
    } else if date_str.contains('.') {
        '.'
    } else {
        '-'
    };

    let segments: Vec<&str> = date_str.split(delimiter).collect();
    if segments.len() == 3 {
        let (y, m, d) = if segments[0].len() == 4 {
            (segments[0], segments[1], segments[2])
        } else {
            (segments[2], segments[1], segments[0])
        };
        let mut y_val = y.parse::<u32>().unwrap_or(2024);
        if y_val < 100 {
            y_val += 2000;
        }
        let m_val = m.parse::<u32>().unwrap_or(1).min(12).max(1);
        let d_val = d.parse::<u32>().unwrap_or(1).min(31).max(1);
        return format!("{:04}-{:02}-{:02}T{}.000Z", y_val, m_val, d_val, time_str);
    }

    format!("{}T12:00:00.000Z", raw)
}

fn geocode_dutch_location(loc: &str, index: usize) -> (f64, f64) {
    let lower = loc.to_lowercase();
    if lower.contains("amsterdam") { return (52.3676, 4.9041); }
    if lower.contains("rotterdam") { return (51.9244, 4.4777); }
    if lower.contains("den haag") || lower.contains("'s-gravenhage") || lower.contains("scheveningen") { return (52.0705, 4.3007); }
    if lower.contains("utrecht") { return (52.0907, 5.1214); }
    if lower.contains("eindhoven") { return (51.4416, 5.4697); }
    if lower.contains("groningen") { return (53.2194, 6.5665); }
    if lower.contains("tilburg") { return (51.5555, 5.0913); }
    if lower.contains("almere") { return (52.3508, 5.2647); }
    if lower.contains("breda") { return (51.5719, 4.7683); }
    if lower.contains("nijmegen") { return (51.8126, 5.8372); }
    if lower.contains("enschede") { return (52.2215, 6.8937); }
    if lower.contains("haarlem") { return (52.3874, 4.6462); }
    if lower.contains("arnhem") { return (51.9851, 5.8987); }
    if lower.contains("amersfoort") { return (52.1561, 5.3878); }
    if lower.contains("zaanstad") || lower.contains("zaandam") { return (52.4420, 4.8292); }
    if lower.contains("den bosch") || lower.contains("'s-hertogenbosch") { return (51.6978, 5.3037); }
    if lower.contains("zwolle") { return (52.5168, 6.0830); }
    if lower.contains("leiden") { return (52.1601, 4.4970); }
    if lower.contains("leeuwarden") { return (53.2012, 5.7999); }
    if lower.contains("maastricht") { return (50.8514, 5.6910); }
    if lower.contains("dordrecht") { return (51.8133, 4.6901); }
    if lower.contains("ede") { return (52.0436, 5.6664); }
    if lower.contains("alphen") { return (52.1290, 4.6555); }
    if lower.contains("alkmaar") { return (52.6324, 4.7534); }
    if lower.contains("delft") { return (52.0116, 4.3571); }
    if lower.contains("venlo") { return (51.3704, 6.1724); }
    if lower.contains("deventer") { return (52.2550, 6.1625); }
    if lower.contains("hilversum") { return (52.2292, 5.1764); }
    if lower.contains("gouda") { return (52.0116, 4.7105); }

    let hash = loc.bytes().fold(index as u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    let offset_lat = ((hash % 1000) as f64 / 1000.0 - 0.5) * 0.4;
    let offset_lng = (((hash / 1000) % 1000) as f64 / 1000.0 - 0.5) * 0.6;
    (52.1326 + offset_lat, 5.2913 + offset_lng)
}

fn looks_like_date(s: &str) -> bool {
    let s = s.trim();
    if s.len() >= 8 && (s.contains('-') || s.contains('/') || s.contains('.')) {
        let first_char = s.chars().next().unwrap_or(' ');
        first_char.is_ascii_digit()
    } else {
        false
    }
}

fn parse_duration_string(s: &str) -> u32 {
    let s = s.trim().to_lowercase();
    if s.contains(':') {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 2 {
            let m = parts[0].parse::<u32>().unwrap_or(5);
            let sec = parts[1].parse::<u32>().unwrap_or(0);
            return m * 60 + sec;
        }
    }
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    let num = digits.parse::<u32>().unwrap_or(5);
    if s.contains("sec") {
        num
    } else if num > 60 && !s.contains("min") && !s.contains('m') {
        num
    } else {
        num * 60
    }
}

fn parse_rating_string(s: &str) -> u8 {
    let s = s.trim();
    let stars = s.chars().filter(|&c| c == '⭐' || c == '*').count();
    if stars > 0 {
        return (stars as u8).min(5).max(1);
    }
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse::<u8>().unwrap_or(4).min(5).max(1)
}

fn rand_simple_id() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(12345) % 100_000
}

fn parse_csv_lines(text: &str, user: &str, avatar: &str, user_id: &str) -> Vec<ToiletSession> {
    let mut sessions = Vec::new();
    let lines: Vec<&str> = text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();

    for (idx, line) in lines.iter().enumerate() {
        let lower_line = line.to_lowercase();
        if idx == 0 && (lower_line.contains("datum") || lower_line.contains("date")) && (lower_line.contains("locatie") || lower_line.contains("location")) {
            continue;
        }

        let delimiter = if line.contains('\t') {
            '\t'
        } else if line.contains(';') {
            ';'
        } else if line.contains('|') {
            '|'
        } else {
            ','
        };

        let parts: Vec<&str> = line.split(delimiter).map(|p| p.trim()).collect();
        if parts.is_empty() {
            continue;
        }

        let (raw_date, raw_loc, duration_part, rating_part, notes_part) = if looks_like_date(parts[0]) {
            let d = parts[0];
            let l = parts.get(1).copied().unwrap_or("WC Onbekend");
            let dur = parts.get(2).copied();
            let rat = parts.get(3).copied();
            let not = parts.get(4).copied();
            (d, l, dur, rat, not)
        } else if parts.len() > 1 && looks_like_date(parts[1]) {
            let l = parts[0];
            let d = parts[1];
            let dur = parts.get(2).copied();
            let rat = parts.get(3).copied();
            let not = parts.get(4).copied();
            (d, l, dur, rat, not)
        } else {
            ("2024-01-01", parts[0], parts.get(1).copied(), parts.get(2).copied(), parts.get(3).copied())
        };

        let timestamp = parse_dutch_date_to_iso(raw_date);
        let (lat, lng) = geocode_dutch_location(raw_loc, idx);

        let duration_seconds = parse_duration_string(duration_part.unwrap_or("5"));
        let rating = parse_rating_string(rating_part.unwrap_or("4"));
        let notes = notes_part.filter(|s| !s.is_empty()).map(|s| s.to_string());

        let id = format!("sess-imp-{}-{}", idx, rand_simple_id());
        sessions.push(ToiletSession {
            id,
            user_id: user_id.to_string(),
            user_name: user.to_string(),
            user_avatar: avatar.to_string(),
            location_name: raw_loc.to_string(),
            latitude: lat,
            longitude: lng,
            timestamp,
            duration_seconds,
            rating,
            poop_type: Some("De Vlotte Boodschap".into()),
            notes,
            tags: None,
            user_email: None,
        });
    }

    sessions
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
                                // Protect registered accounts: others cannot post in their name
                                if let Some(acc) = guard.accounts.iter().find(|a| a.id == session.user_id || a.name.to_lowercase() == session.user_name.to_lowercase()) {
                                    let matches_email = session.user_email.as_ref().map(|e| e.trim().to_lowercase()) == Some(acc.email.trim().to_lowercase());
                                    if !matches_email {
                                        let resp = json_response(r#"{"error":"Dit account is beveiligd met een wachtwoord. Log eerst in om sessies te plaatsen."}"#, 403);
                                        let _ = request.respond(resp);
                                        continue;
                                    }
                                }
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

                // REST API: POST /api/sessions/batch
                if path_part == "/api/sessions/batch" && request.method() == &Method::Post {
                    let mut body = String::new();
                    let _ = request.as_reader().read_to_string(&mut body);
                    let mut new_sessions = Vec::new();

                    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&body) {
                        let items_opt = if let Some(arr) = json_val.as_array() {
                            Some(arr.clone())
                        } else if let Some(arr) = json_val.get("sessions").and_then(|s| s.as_array()) {
                            Some(arr.clone())
                        } else {
                            None
                        };

                        if let Some(items) = items_opt {
                            for (idx, item) in items.iter().enumerate() {
                                let loc = item.get("locationName")
                                    .or_else(|| item.get("location"))
                                    .or_else(|| item.get("locatie"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("WC Onbekend")
                                    .to_string();

                                let raw_date = item.get("timestamp")
                                    .or_else(|| item.get("date"))
                                    .or_else(|| item.get("datum"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                let timestamp = parse_dutch_date_to_iso(raw_date);

                                let (lat, lng) = if let (Some(la), Some(lo)) = (
                                    item.get("latitude").and_then(|v| v.as_f64()),
                                    item.get("longitude").and_then(|v| v.as_f64()),
                                ) {
                                    (la, lo)
                                } else {
                                    geocode_dutch_location(&loc, idx)
                                };

                                let duration = item.get("durationSeconds")
                                    .or_else(|| item.get("duration"))
                                    .or_else(|| item.get("duur"))
                                    .and_then(|v| v.as_u64())
                                    .map(|v| v as u32)
                                    .unwrap_or(300);

                                let rating = item.get("rating")
                                    .and_then(|v| v.as_u64())
                                    .map(|v| v.min(5).max(1) as u8)
                                    .unwrap_or(4);

                                let poop_type = item.get("poopType")
                                    .or_else(|| item.get("type"))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .or_else(|| Some("De Vlotte Boodschap".into()));

                                let notes = item.get("notes")
                                    .or_else(|| item.get("notitie"))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());

                                let user_name = item.get("userName")
                                    .or_else(|| item.get("user"))
                                    .or_else(|| item.get("naam"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Bram")
                                    .to_string();

                                let user_avatar = item.get("userAvatar")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| user_name.chars().next().unwrap_or('P').to_uppercase().to_string());

                                let user_id = item.get("userId")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| format!("user-{}", user_name.to_lowercase()));

                                let user_email = item.get("userEmail")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());

                                let sess_id = item.get("id")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| format!("sess-imp-{}-{}", idx, rand_simple_id()));

                                new_sessions.push(ToiletSession {
                                    id: sess_id,
                                    user_id,
                                    user_name,
                                    user_avatar,
                                    location_name: loc,
                                    latitude: lat,
                                    longitude: lng,
                                    timestamp,
                                    duration_seconds: duration,
                                    rating,
                                    poop_type,
                                    notes,
                                    tags: None,
                                    user_email,
                                });
                            }
                        } else if let Some(raw_text) = json_val.get("text").or_else(|| json_val.get("csv")).and_then(|v| v.as_str()) {
                            let default_user = json_val.get("userName").or_else(|| json_val.get("user")).and_then(|v| v.as_str()).unwrap_or("Bram");
                            let default_avatar = json_val.get("userAvatar").and_then(|v| v.as_str()).map(|s| s.to_string()).unwrap_or_else(|| default_user.chars().next().unwrap_or('P').to_uppercase().to_string());
                            let default_user_id = format!("user-{}", default_user.to_lowercase());
                            new_sessions = parse_csv_lines(raw_text, default_user, &default_avatar, &default_user_id);
                        }
                    } else {
                        // Plain text or CSV in raw request body
                        new_sessions = parse_csv_lines(&body, "Bram", "B", "user-bram");
                    }

                    if !new_sessions.is_empty() {
                        let mut guard = state.data.write().unwrap();
                        let count = new_sessions.len();
                        for s in new_sessions {
                            let prof_exists = guard.profiles.iter().any(|p| p.id == s.user_id || p.name.to_lowercase() == s.user_name.to_lowercase());
                            if !prof_exists {
                                guard.profiles.push(UserProfile {
                                    id: s.user_id.clone(),
                                    name: s.user_name.clone(),
                                    avatar: s.user_avatar.clone(),
                                    tagline: "Geregistreerde Poeper".into(),
                                    email: s.user_email.clone(),
                                });
                            }
                            guard.sessions.retain(|existing| existing.id != s.id);
                            guard.sessions.push(s);
                        }
                        guard.sessions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                        let list = guard.sessions.clone();
                        drop(guard);
                        state.persist();
                        let resp = json_response(&serde_json::json!({
                            "success": true,
                            "imported": count,
                            "sessions": list
                        }).to_string(), 200);
                        let _ = request.respond(resp);
                        continue;
                    } else {
                        let resp = json_response(r#"{"success":false,"error":"Geen geldige bezoeken gevonden om te importeren."}"#, 400);
                        let _ = request.respond(resp);
                        continue;
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
                                // Prevent guests from overwriting a registered account's profile
                                let is_registered = guard.accounts.iter().any(|a| a.id == profile.id || a.name.to_lowercase() == profile.name.to_lowercase());
                                if is_registered && profile.email.is_none() {
                                    let resp = json_response(r#"{"error":"Dit profiel is beschermd door een e-mailaccount"}"#, 403);
                                    let _ = request.respond(resp);
                                    continue;
                                }
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
                            // Never expose user accounts or credentials via public GET
                            let resp = json_response("[]", 200);
                            let _ = request.respond(resp);
                            continue;
                        }
                        &Method::Post => {
                            let mut body = String::new();
                            let _ = request.as_reader().read_to_string(&mut body);
                            if let Ok(account) = serde_json::from_str::<UserAccount>(&body) {
                                let mut guard = state.data.write().unwrap();
                                let email_lower = account.email.trim().to_lowercase();
                                if email_lower.is_empty() || !email_lower.contains('@') {
                                    let resp = json_response(r#"{"success":false,"error":"Voer een geldig e-mailadres in"}"#, 400);
                                    let _ = request.respond(resp);
                                    continue;
                                }
                                if guard.accounts.iter().any(|a| a.email.to_lowercase() == email_lower) {
                                    let resp = json_response(r#"{"success":false,"error":"Er bestaat al een account met dit e-mailadres"}"#, 400);
                                    let _ = request.respond(resp);
                                    continue;
                                }
                                guard.accounts.push(account.clone());
                                if let Some(p) = guard.profiles.iter_mut().find(|p| p.id == account.id || p.name.to_lowercase() == account.name.to_lowercase() || p.email.as_ref().map(|e| e.to_lowercase()) == Some(email_lower.clone())) {
                                    p.id = account.id.clone();
                                    p.name = account.name.clone();
                                    p.avatar = account.avatar.clone();
                                    p.email = Some(account.email.clone());
                                } else {
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
                        let mut guard = state.data.write().unwrap();
                        let email_lower = login_req.email.trim().to_lowercase();
                        if let Some(account) = guard.accounts.iter().find(|a| a.email.to_lowercase() == email_lower).cloned() {
                            if let Some(ref acc_pwd) = account.password {
                                if let Some(ref req_pwd) = login_req.password {
                                    if acc_pwd != req_pwd {
                                        let resp = json_response(r#"{"success":false,"error":"Onjuist wachtwoord"}"#, 401);
                                        let _ = request.respond(resp);
                                        continue;
                                    }
                                } else {
                                    let resp = json_response(r#"{"success":false,"error":"Vul je wachtwoord in"}"#, 401);
                                    let _ = request.respond(resp);
                                    continue;
                                }
                            }
                            let profile = if let Some(p) = guard.profiles.iter_mut().find(|p| p.id == account.id || p.name.to_lowercase() == account.name.to_lowercase() || p.email.as_ref().map(|e| e.to_lowercase()) == Some(email_lower.clone())) {
                                p.id = account.id.clone();
                                p.name = account.name.clone();
                                p.avatar = account.avatar.clone();
                                p.email = Some(account.email.clone());
                                p.clone()
                            } else {
                                let new_p = UserProfile {
                                    id: account.id.clone(),
                                    name: account.name.clone(),
                                    avatar: account.avatar.clone(),
                                    tagline: "Geregistreerde Poeper".into(),
                                    email: Some(account.email.clone()),
                                };
                                guard.profiles.push(new_p.clone());
                                new_p
                            };
                            drop(guard);
                            state.persist();
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
                    // Only clear sessions - protect user accounts and registered profiles!
                    let mut guard = state.data.write().unwrap();
                    guard.sessions.clear();
                    drop(guard);
                    state.persist();
                    let resp = json_response(r#"{"status":"reset_successful","message":"Sessies zijn gewist; accounts zijn behouden"}"#, 200);
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
