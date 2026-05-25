//! Axum HTTP server: login, dashboard, and JSON stats API.
//!
//! Routes:
//!   GET  /            → login page
//!   POST /login       → verify password → set session cookie → redirect /dashboard
//!   GET  /dashboard   → live dashboard HTML (auth required)
//!   GET  /api/stats   → JSON stats (auth required)

use std::sync::{Arc, Mutex};

use axum::{
    extract::{Form, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

use crate::{config::Config, dashboard::AppState};

// ── types ─────────────────────────────────────────────────────────────────────

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
struct ServerState {
    app: Arc<Mutex<AppState>>,
    password_sha256: String,
    session_token: String,
}

#[derive(Deserialize)]
struct LoginForm {
    password: String,
}

// ── public entry point ────────────────────────────────────────────────────────

pub async fn serve(config: Config, state: Arc<Mutex<AppState>>) {
    let session_token = compute_session_token(&config.server.session_secret);

    let srv = ServerState {
        app: state,
        password_sha256: config.server.password_sha256.clone(),
        session_token,
    };

    let router = Router::new()
        .route("/", get(login_page))
        .route("/login", post(login_post))
        .route("/dashboard", get(dashboard_page))
        .route("/api/stats", get(api_stats))
        .with_state(srv);

    let addr = format!("0.0.0.0:{}", config.server.port);
    println!("web3-keygen: dashboard listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("bind {addr}: {e}"));

    axum::serve(listener, router).await.expect("axum serve");
}

// ── auth helpers ──────────────────────────────────────────────────────────────

fn compute_session_token(secret: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(b"authenticated");
    hex::encode(mac.finalize().into_bytes())
}

fn sha256_hex(input: &str) -> String {
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    h.update(input.as_bytes());
    hex::encode(h.finalize())
}

fn session_cookie(headers: &HeaderMap) -> Option<String> {
    let cookie_hdr = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in cookie_hdr.split(';') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("session=") {
            return Some(val.to_owned());
        }
    }
    None
}

fn is_authed(headers: &HeaderMap, expected: &str) -> bool {
    session_cookie(headers)
        .map(|tok| tok == expected)
        .unwrap_or(false)
}

// ── route handlers ────────────────────────────────────────────────────────────

async fn login_page(headers: HeaderMap, State(srv): State<ServerState>) -> Response {
    if is_authed(&headers, &srv.session_token) {
        return Redirect::to("/dashboard").into_response();
    }
    Html(LOGIN_HTML).into_response()
}

async fn login_post(
    State(srv): State<ServerState>,
    Form(form): Form<LoginForm>,
) -> Response {
    let submitted = sha256_hex(&form.password);
    if submitted.eq_ignore_ascii_case(&srv.password_sha256) {
        let cookie = format!(
            "session={}; Path=/; HttpOnly; SameSite=Strict",
            srv.session_token
        );
        (
            StatusCode::SEE_OTHER,
            [
                (header::LOCATION, "/dashboard"),
                (header::SET_COOKIE, &cookie),
            ],
        )
            .into_response()
    } else {
        Redirect::to("/?error=1").into_response()
    }
}

async fn dashboard_page(headers: HeaderMap, State(srv): State<ServerState>) -> Response {
    if !is_authed(&headers, &srv.session_token) {
        return Redirect::to("/").into_response();
    }
    Html(DASHBOARD_HTML).into_response()
}

async fn api_stats(headers: HeaderMap, State(srv): State<ServerState>) -> Response {
    if !is_authed(&headers, &srv.session_token) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    let (generated, checked, hits_len, uptime_secs, hits_list) = {
        let s = srv.app.lock().unwrap();
        (
            s.generated,
            s.checked,
            s.hits.len() as u64,
            s.start.elapsed().as_secs(),
            s.hits.clone(),
        )
    };

    // hits_list is already serializable (HitRecord derives Serialize).
    let body = serde_json::json!({
        "generated":  generated,
        "checked":    checked,
        "hits":       hits_len,
        "uptime_secs": uptime_secs,
        "hits_list":  hits_list,   // each item has: chain, address, public_key, private_key, balance, found_at_secs
    });

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

// ── static HTML ───────────────────────────────────────────────────────────────

const LOGIN_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>web3-keygen / Login</title>
<style>
  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    background: #0d1117;
    color: #e6edf3;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
  }
  .card {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 8px;
    padding: 2rem 2.5rem;
    width: 100%;
    max-width: 380px;
  }
  h1 { font-size: 1.1rem; color: #58a6ff; margin-bottom: 1.5rem; letter-spacing: 0.05em; }
  label { display: block; font-size: 0.8rem; color: #8b949e; margin-bottom: 0.4rem; }
  input[type=password] {
    width: 100%;
    padding: 0.55rem 0.75rem;
    background: #0d1117;
    border: 1px solid #30363d;
    border-radius: 6px;
    color: #e6edf3;
    font-family: inherit;
    font-size: 0.9rem;
    outline: none;
    margin-bottom: 1rem;
  }
  input[type=password]:focus { border-color: #58a6ff; }
  button {
    width: 100%;
    padding: 0.6rem;
    background: #238636;
    border: none;
    border-radius: 6px;
    color: #fff;
    font-family: inherit;
    font-size: 0.9rem;
    cursor: pointer;
    letter-spacing: 0.03em;
  }
  button:hover { background: #2ea043; }
  .err {
    margin-top: 0.9rem;
    color: #f85149;
    font-size: 0.82rem;
    text-align: center;
  }
</style>
</head>
<body>
<div class="card">
  <h1>web3-keygen / Live Scanner</h1>
  <form method="POST" action="/login">
    <label for="pw">Dashboard Password</label>
    <input type="password" id="pw" name="password" autofocus autocomplete="current-password">
    <button type="submit">Sign in</button>
  </form>
  <div id="err-msg" class="err" style="display:none">Incorrect password.</div>
</div>
<script>
  if (window.location.search.includes('error=1')) {
    document.getElementById('err-msg').style.display = 'block';
  }
</script>
</body>
</html>
"#;

const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>web3-keygen / Live Scanner</title>
<style>
  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    background: #0d1117;
    color: #e6edf3;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.88rem;
    min-height: 100vh;
  }
  header {
    background: #161b22;
    border-bottom: 1px solid #30363d;
    padding: 0.75rem 1.5rem;
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  header h1 { font-size: 1rem; color: #58a6ff; letter-spacing: 0.06em; }
  header .dot {
    width: 8px; height: 8px; border-radius: 50%;
    background: #3fb950; display: inline-block;
    animation: pulse 2s ease-in-out infinite;
  }
  @keyframes pulse {
    0%,100% { opacity: 1; } 50% { opacity: 0.4; }
  }
  .cards {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
    padding: 1.2rem 1.5rem;
  }
  .card {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 8px;
    padding: 1rem 1.4rem;
    min-width: 160px;
    flex: 1 1 150px;
  }
  .card .label { color: #8b949e; font-size: 0.75rem; margin-bottom: 0.35rem; }
  .card .value { font-size: 1.4rem; color: #e6edf3; letter-spacing: 0.02em; }
  .card.hit .value { color: #3fb950; }
  .section {
    padding: 0 1.5rem 1.5rem;
  }
  .section h2 {
    font-size: 0.8rem;
    color: #8b949e;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    margin-bottom: 0.75rem;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th {
    text-align: left;
    color: #8b949e;
    font-size: 0.75rem;
    padding: 0.4rem 0.6rem;
    border-bottom: 1px solid #30363d;
    white-space: nowrap;
  }
  td {
    padding: 0.45rem 0.6rem;
    border-bottom: 1px solid #21262d;
    color: #c9d1d9;
    vertical-align: top;
    word-break: break-all;
  }
  td code {
    color: #79c0ff;
    background: #0d1117;
    border-radius: 3px;
    padding: 0.1rem 0.25rem;
    font-size: 0.8rem;
  }
  td.chain { color: #58a6ff; white-space: nowrap; }
  td.addr  { color: #3fb950; }
  td.bal   { color: #f0883e; font-weight: bold; white-space: nowrap; }
  .empty {
    color: #30363d;
    text-align: center;
    padding: 2rem;
    font-size: 0.82rem;
  }
  .ts { color: #484f58; font-size: 0.75rem; white-space: nowrap; }
</style>
</head>
<body>
<header>
  <span class="dot"></span>
  <h1>web3-keygen / Live Scanner</h1>
</header>

<div class="cards">
  <div class="card">
    <div class="label">Generated</div>
    <div class="value" id="generated">—</div>
  </div>
  <div class="card">
    <div class="label">Checked</div>
    <div class="value" id="checked">—</div>
  </div>
  <div class="card hit">
    <div class="label">Hits</div>
    <div class="value" id="hits">—</div>
  </div>
  <div class="card">
    <div class="label">keys / sec</div>
    <div class="value" id="keysec">—</div>
  </div>
  <div class="card">
    <div class="label">RPC calls / sec</div>
    <div class="value" id="rpcsec">—</div>
  </div>
  <div class="card">
    <div class="label">Uptime</div>
    <div class="value" id="uptime">—</div>
  </div>
</div>

<div class="section">
  <h2>Hits &mdash; newest first</h2>
  <div id="hits-container">
    <p class="empty">No hits yet. Scanner running&hellip;</p>
  </div>
</div>

<script>
function fmt(n) {
  return Number(n).toLocaleString();
}
function fmtUptime(s) {
  const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60), ss = s % 60;
  return [h, m, ss].map(v => String(v).padStart(2,'0')).join(':');
}
function fmtTs(unix) {
  const d = new Date(unix * 1000);
  return d.toUTCString().replace('GMT','UTC');
}

async function refresh() {
  try {
    const r = await fetch('/api/stats');
    if (r.status === 401) { window.location = '/'; return; }
    const d = await r.json();
    const up = d.uptime_secs || 1;
    document.getElementById('generated').textContent = fmt(d.generated);
    document.getElementById('checked').textContent   = fmt(d.checked);
    document.getElementById('hits').textContent      = fmt(d.hits);
    document.getElementById('keysec').textContent    = fmt(Math.round(d.generated / up));
    document.getElementById('rpcsec').textContent    = (d.checked / up).toFixed(1);
    document.getElementById('uptime').textContent    = fmtUptime(up);

    const list = (d.hits_list || []).slice().reverse();
    const container = document.getElementById('hits-container');
    if (list.length === 0) {
      container.innerHTML = '<p class="empty">No hits yet. Scanner running&hellip;</p>';
    } else {
      let rows = list.map(h => `
        <tr>
          <td class="ts">${fmtTs(h.found_at_secs)}</td>
          <td class="chain">${h.chain}</td>
          <td class="addr">${h.address}</td>
          <td><code>${h.public_key}</code></td>
          <td><code>${h.private_key}</code></td>
          <td class="bal">${h.balance.toFixed(8)}</td>
        </tr>`).join('');
      container.innerHTML = `
        <table>
          <thead>
            <tr>
              <th>Time Found</th>
              <th>Chain</th>
              <th>Address</th>
              <th>Public Key</th>
              <th>Private Key</th>
              <th>Balance</th>
            </tr>
          </thead>
          <tbody>${rows}</tbody>
        </table>`;
    }
  } catch (e) {
    console.error('stats fetch error:', e);
  }
}

refresh();
setInterval(refresh, 2000);
</script>
</body>
</html>
"#;
