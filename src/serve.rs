//! HTMX-driven local HTTP server for the claude-time report.
//!
//! Tiny, synchronous, single-threaded (per-request handle on the main loop).
//! Routes:
//!   GET /                       → full HTML page
//!   GET /window?since=X[&by=Y]  → fragment (rebuilds <main>) for HTMX swap
//!   GET /session/{id}           → fragment for click-to-expand session detail
//!
//! Opens the browser to the bind URL on startup. Ctrl+C to stop.

use anyhow::{anyhow, Result};
use std::io::Cursor;
use tiny_http::{Header, Method, Request, Response, Server};

use crate::config::Config;
use crate::model::SessionRecord;

pub fn run(port: u16) -> Result<()> {
    let bind = format!("127.0.0.1:{port}");
    let server = Server::http(&bind).map_err(|e| anyhow!("bind {bind}: {e}"))?;
    let url = format!("http://{bind}");

    println!("claude-time report at {url}");
    println!("(Ctrl+C to stop)");

    // Best-effort browser open. Failure is fine — the URL is printed above.
    if let Err(err) = open::that_detached(&url) {
        eprintln!("[claude-time serve] could not auto-open browser: {err}");
    }

    for request in server.incoming_requests() {
        if let Err(err) = handle(request) {
            eprintln!("[claude-time serve] request handler: {err:#}");
        }
    }
    Ok(())
}

fn handle(req: Request) -> Result<()> {
    let url = req.url().to_string();
    let method = req.method().clone();
    let cfg = crate::config::load().unwrap_or_default();

    if method != Method::Get {
        return respond(req, 405, "method not allowed");
    }

    // Strip query string for path matching.
    let (path, qs) = match url.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (url.clone(), String::new()),
    };

    match path.as_str() {
        "/" => respond_html(req, &full_page(&cfg, "7d", None)?),
        "/window" => {
            let since = query_param(&qs, "since").unwrap_or_else(|| "7d".to_string());
            let by = query_param(&qs, "by");
            respond_html(req, &full_page(&cfg, &since, by.as_deref())?)
        }
        path if path.starts_with("/session/") => {
            let id = &path["/session/".len()..];
            respond_html(req, &session_fragment(&cfg, id)?)
        }
        "/healthz" => respond(req, 200, "ok"),
        _ => respond(req, 404, "not found"),
    }
}

fn full_page(cfg: &Config, since: &str, by: Option<&str>) -> Result<String> {
    let cutoff = crate::report::parse_since(since)?;
    let mut sessions = crate::report::load_sessions_since(cutoff)?;
    sessions.extend(crate::storage::load_archive_since(cutoff)?);
    Ok(crate::html_report::render(&sessions, cfg, by))
}

fn session_fragment(cfg: &Config, session_id: &str) -> Result<String> {
    // Search both hot (sessions/*.json) and cold (archive.jsonl) for the id.
    let session = find_session(session_id)?;
    let Some(session) = session else {
        return Ok(format!(
            r#"<p class="muted">No session with id <code>{}</code></p>"#,
            escape(session_id)
        ));
    };

    let pinpoints = crate::git_history::pinpoint_waste(&session).unwrap_or_default();
    let hourly = cfg.report.hourly_rate_usd;
    let cost = session.total_cost(hourly);

    let cache_ratio = session
        .cache_hit_ratio()
        .map(|r| format!("{:.0}%", r * 100.0))
        .unwrap_or_else(|| "-".to_string());

    let mut body = String::new();
    body.push_str(&format!(
        r#"<div class="detail-grid">
  <div><span class="muted small">project</span><div>{project}</div></div>
  <div><span class="muted small">model</span><div>{model}</div></div>
  <div><span class="muted small">cwd</span><div><code>{cwd}</code></div></div>
  <div><span class="muted small">duration</span><div>{dur}</div></div>
  <div><span class="muted small">Claude $</span><div>${claude_cost:.4}</div></div>
  <div><span class="muted small">total cost</span><div>${total_cost:.2}</div></div>
  <div><span class="muted small">turns</span><div>{turns}</div></div>
  <div><span class="muted small">tokens in/out</span><div>{in_tokens} / {out_tokens}</div></div>
  <div><span class="muted small">cache hit</span><div>{cache_ratio}</div></div>
  <div><span class="muted small">lines + / -</span><div>{added} / {removed}</div></div>
</div>"#,
        project = escape(session.project.as_deref().unwrap_or("-")),
        model = escape(session.model.as_deref().unwrap_or("-")),
        cwd = escape(&session.cwd),
        dur = session
            .duration_minutes()
            .map(|m| format!("{m:.1} min"))
            .unwrap_or_else(|| "-".to_string()),
        claude_cost = session.total_cost_usd,
        total_cost = cost,
        turns = session.turn_count,
        in_tokens = session.input_tokens,
        out_tokens = session.output_tokens,
        added = session.lines_added,
        removed = session.lines_removed,
    ));

    if !pinpoints.is_empty() {
        body.push_str(r#"<h4>Per-file pinpoints</h4><table class="data"><thead><tr><th>Severity</th><th>File</th><th>Edits</th><th>Lines+</th><th>Retention</th></tr></thead><tbody>"#);
        for p in &pinpoints {
            body.push_str(&format!(
                r#"<tr><td><span class="badge sev-{sev}">{label}</span></td><td><code>{file}</code></td><td class="num">{edits}</td><td class="num">{added}</td><td class="num">{ret:.0}%</td></tr>"#,
                sev = severity_class(p.severity),
                label = p.severity.label(),
                file = escape(&p.file),
                edits = p.edits,
                added = p.lines_added,
                ret = p.retention * 100.0,
            ));
        }
        body.push_str("</tbody></table>");
    } else if !session.per_file_edits.is_empty() {
        body.push_str(r#"<p class="muted small">No waste pinpoints — files edited in this session retained their changes.</p>"#);
    }

    if !session.tool_calls.is_empty() {
        body.push_str(r#"<h4>Tool calls</h4><div class="chiprow">"#);
        for (tool, count) in &session.tool_calls {
            body.push_str(&format!(
                r#"<span class="chip">{tool} <span class="muted">×{count}</span></span>"#,
                tool = escape(tool),
            ));
        }
        body.push_str("</div>");
    }

    Ok(body)
}

fn severity_class(s: crate::model::PinpointSeverity) -> &'static str {
    use crate::model::PinpointSeverity::*;
    match s {
        Severe => "severe",
        Iterated => "iterated",
        Lost => "lost",
    }
}

fn find_session(session_id: &str) -> Result<Option<SessionRecord>> {
    // Try per-session file first.
    let path = crate::paths::session_file(session_id)?;
    if path.exists() {
        if let Ok(raw) = std::fs::read_to_string(&path) {
            if let Ok(r) = serde_json::from_str::<SessionRecord>(&raw) {
                return Ok(Some(r));
            }
        }
    }
    // Fall through to archive.jsonl.
    let archive = crate::paths::archive_jsonl()?;
    if archive.exists() {
        use std::io::{BufRead, BufReader};
        let file = std::fs::File::open(&archive)?;
        for line in BufReader::new(file).lines() {
            let line = line?;
            if let Ok(r) = serde_json::from_str::<SessionRecord>(&line) {
                if r.session_id == session_id {
                    return Ok(Some(r));
                }
            }
        }
    }
    Ok(None)
}

fn query_param(qs: &str, key: &str) -> Option<String> {
    for pair in qs.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == key {
                return Some(percent_decode(v));
            }
        }
    }
    None
}

fn percent_decode(s: &str) -> String {
    // Light-weight: handle the few escapes a query string typically carries.
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'+' {
            out.push(' ');
            i += 1;
            continue;
        }
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
            {
                out.push(byte as char);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn respond_html(req: Request, body: &str) -> Result<()> {
    let resp = Response::new(
        tiny_http::StatusCode(200),
        vec![
            Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap(),
            Header::from_bytes(&b"Cache-Control"[..], &b"no-store"[..]).unwrap(),
        ],
        Cursor::new(body.as_bytes().to_vec()),
        Some(body.len()),
        None,
    );
    req.respond(resp).ok();
    Ok(())
}

fn respond(req: Request, status: u16, body: &str) -> Result<()> {
    let resp = Response::new(
        tiny_http::StatusCode(status),
        vec![Header::from_bytes(&b"Content-Type"[..], &b"text/plain; charset=utf-8"[..]).unwrap()],
        Cursor::new(body.as_bytes().to_vec()),
        Some(body.len()),
        None,
    );
    req.respond(resp).ok();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_query_param() {
        assert_eq!(query_param("since=7d", "since"), Some("7d".into()));
        assert_eq!(
            query_param("by=project&since=30d", "since"),
            Some("30d".into())
        );
        assert_eq!(query_param("by=project", "since"), None);
    }

    #[test]
    fn percent_decode_handles_spaces_and_escapes() {
        assert_eq!(percent_decode("hello+world"), "hello world");
        assert_eq!(percent_decode("a%2Fb"), "a/b");
    }
}
