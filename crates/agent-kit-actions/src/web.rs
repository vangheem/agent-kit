use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use agent_kit_core::action::{
    ActionContext, ActionDescriptor, ActionError, ActionOutput, Effect, ServiceKind,
};
use agent_kit_core::blob::PutBlob;
use serde_json::{Value, json};
use url::{Host, Url};

use crate::util::{self, ensure_mock, require_object, require_string};

const FETCH_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_BODY: usize = 2 * 1024 * 1024;

pub(crate) fn search_descriptor() -> ActionDescriptor {
    ActionDescriptor {
        id: "web.search".into(),
        title: "Web search".into(),
        summary: "Search the web and return titles, urls, and snippets.".into(),
        input_schema: json!({
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": { "type": "string" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 20 }
            }
        }),
        output_schema: json!({
            "type": "object",
            "required": ["results"],
            "properties": {
                "results": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["title", "url", "snippet"],
                        "properties": {
                            "title": { "type": "string" },
                            "url": { "type": "string" },
                            "snippet": { "type": "string" }
                        }
                    }
                }
            }
        }),
        requires: vec![ServiceKind::WebSearch],
        effect: Effect::Read,
    }
}

pub(crate) async fn search(ctx: &ActionContext, input: Value) -> Result<ActionOutput, ActionError> {
    require_object(&input)?;
    ensure_mock(ctx, ServiceKind::WebSearch).await?;
    let query = require_string(&input, "query")?;
    let limit = util::optional_limit(&input, 5, 20)?;
    let results = (1..=limit)
        .map(|index| {
            json!({
                "title": format!("{query} result {index}"),
                "url": format!(
                    "https://example.test/s/{:08x}/{index}",
                    util::stable_hash(&query) as u32
                ),
                "snippet": format!("Mock result {index} for {query}"),
            })
        })
        .collect::<Vec<_>>();
    Ok(util::ok(json!({ "results": results })))
}

pub(crate) fn fetch_descriptor() -> ActionDescriptor {
    ActionDescriptor {
        id: "web.fetch".into(),
        title: "Fetch page".into(),
        summary: "Fetch a public http or https URL and store the raw body.".into(),
        input_schema: json!({
            "type": "object",
            "required": ["url"],
            "properties": {
                "url": { "type": "string" }
            }
        }),
        output_schema: json!({
            "type": "object",
            "required": ["title", "url", "text", "content_type"],
            "properties": {
                "title": { "type": "string" },
                "url": { "type": "string" },
                "text": { "type": "string" },
                "content_type": { "type": "string" }
            }
        }),
        requires: Vec::new(),
        effect: Effect::Read,
    }
}

pub(crate) async fn fetch(ctx: &ActionContext, input: Value) -> Result<ActionOutput, ActionError> {
    require_object(&input)?;
    let url = validated_url(&input)?;
    if let Some(ip) = literal_ip(&url)
        && address_is_blocked(ip)
    {
        return Err(not_allowed());
    }
    if let Some(Host::Domain(domain)) = url.host()
        && host_name_blocked(domain)
    {
        return Err(not_allowed());
    }
    match tokio::time::timeout(FETCH_TIMEOUT, fetch_and_store(ctx, url)).await {
        Ok(result) => result,
        Err(_) => Err(ActionError::Provider {
            message: "fetch timed out".into(),
        }),
    }
}

fn validated_url(input: &Value) -> Result<Url, ActionError> {
    let raw = require_string(input, "url")?;
    if raw.len() > 2048 {
        return Err(util::invalid("url is too long"));
    }
    let url = Url::parse(&raw).map_err(|_| util::invalid("url is not valid"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(util::invalid("url scheme must be http or https"));
    }
    if url.host().is_none() {
        return Err(util::invalid("url host is required"));
    }
    canonicalize(url)
}

fn canonicalize(mut url: Url) -> Result<Url, ActionError> {
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_fragment(None);
    let host = match url.host() {
        Some(Host::Domain(domain)) => Some(domain.trim_end_matches('.').to_ascii_lowercase()),
        Some(_) => None,
        None => return Err(util::invalid("url host is required")),
    };
    if let Some(host) = host {
        if host.is_empty() || host_name_blocked(&host) {
            return Err(not_allowed());
        }
        url.set_host(Some(&host))
            .map_err(|_| util::invalid("url host is not valid"))?;
    }
    let port = url
        .port_or_known_default()
        .ok_or_else(|| util::invalid("url port is required"))?;
    if port == 0 {
        return Err(util::invalid("url port is not allowed"));
    }
    Ok(url)
}

async fn fetch_and_store(ctx: &ActionContext, url: Url) -> Result<ActionOutput, ActionError> {
    let addrs = resolve_checked(&url).await?;
    let mut builder = reqwest::Client::builder()
        .timeout(FETCH_TIMEOUT)
        .connect_timeout(FETCH_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .user_agent("agent-kit/0.1");
    if let Some(domain) = url.domain() {
        builder = builder.resolve(domain, addrs[0]);
    }
    let client = builder.build().map_err(|_| ActionError::Internal {
        message: "http client failed".into(),
    })?;
    let response = client
        .get(url.clone())
        .send()
        .await
        .map_err(|_| ActionError::Provider {
            message: "fetch failed".into(),
        })?;
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(sanitize_content_type)
        .unwrap_or_else(|| "application/octet-stream".to_owned());
    if response
        .content_length()
        .is_some_and(|len| len > MAX_BODY as u64)
    {
        return Err(ActionError::Provider {
            message: "response body exceeds 2 MiB".into(),
        });
    }
    let bytes = read_capped(response).await?;
    let meta = ctx
        .blobs
        .put(PutBlob {
            owner_user_id: ctx.user_id,
            content_type: content_type.clone(),
            bytes: bytes.clone(),
        })
        .await
        .map_err(|_| ActionError::Internal {
            message: "blob storage failed".into(),
        })?;
    let (title, text) = response_text(&content_type, &bytes);
    let title = if title.is_empty() {
        url.to_string()
    } else {
        title
    };
    Ok(ActionOutput {
        body: json!({
            "title": title,
            "url": url.as_str(),
            "text": text,
            "content_type": content_type,
        }),
        input_blob_id: None,
        output_blob_id: Some(meta.id),
    })
}

async fn resolve_checked(url: &Url) -> Result<Vec<SocketAddr>, ActionError> {
    let port = url
        .port_or_known_default()
        .ok_or_else(|| util::invalid("url port is required"))?;
    match url.host() {
        Some(Host::Ipv4(ip)) => {
            if ipv4_is_blocked(ip) {
                return Err(not_allowed());
            }
            Ok(vec![SocketAddr::new(IpAddr::V4(ip), port)])
        }
        Some(Host::Ipv6(ip)) => {
            if ipv6_is_blocked(ip) {
                return Err(not_allowed());
            }
            Ok(vec![SocketAddr::new(IpAddr::V6(ip), port)])
        }
        Some(Host::Domain(domain)) => {
            let host = domain.trim_end_matches('.').to_ascii_lowercase();
            if host_name_blocked(&host) {
                return Err(not_allowed());
            }
            let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), port))
                .await
                .map_err(|_| ActionError::Provider {
                    message: "host did not resolve".into(),
                })?
                .collect();
            if addrs.is_empty() {
                return Err(ActionError::Provider {
                    message: "host did not resolve".into(),
                });
            }
            if addrs.iter().any(|addr| address_is_blocked(addr.ip())) {
                return Err(not_allowed());
            }
            Ok(addrs)
        }
        None => Err(util::invalid("url host is required")),
    }
}

async fn read_capped(mut response: reqwest::Response) -> Result<Vec<u8>, ActionError> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| ActionError::Provider {
        message: "fetch failed".into(),
    })? {
        if bytes.len().saturating_add(chunk.len()) > MAX_BODY {
            return Err(ActionError::Provider {
                message: "response body exceeds 2 MiB".into(),
            });
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn literal_ip(url: &Url) -> Option<IpAddr> {
    match url.host() {
        Some(Host::Ipv4(ip)) => Some(IpAddr::V4(ip)),
        Some(Host::Ipv6(ip)) => Some(IpAddr::V6(ip)),
        _ => None,
    }
}

pub(crate) fn address_is_blocked(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ipv4_is_blocked(ip),
        IpAddr::V6(ip) => ipv6_is_blocked(ip),
    }
}

fn ipv4_is_blocked(ip: Ipv4Addr) -> bool {
    let [a, b, _, _] = ip.octets();
    a == 0
        || a == 10
        || a == 127
        || (a == 169 && b == 254)
        || (a == 172 && (b & 0xf0) == 16)
        || (a == 192 && b == 168)
        || a >= 224
        || ip == Ipv4Addr::new(169, 254, 169, 254)
}

fn ipv6_is_blocked(ip: Ipv6Addr) -> bool {
    if let Some(mapped) = embedded_v4(ip) {
        return ipv4_is_blocked(mapped);
    }
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || is_unique_local(ip)
        || is_link_local_v6(ip)
}

fn embedded_v4(ip: Ipv6Addr) -> Option<Ipv4Addr> {
    let octets = ip.octets();
    let mapped =
        octets[..10].iter().all(|byte| *byte == 0) && octets[10] == 0xff && octets[11] == 0xff;
    let compatible =
        octets[..12].iter().all(|byte| *byte == 0) && octets[12..].iter().any(|byte| *byte != 0);
    if mapped || compatible {
        Some(Ipv4Addr::new(
            octets[12], octets[13], octets[14], octets[15],
        ))
    } else {
        None
    }
}

fn is_unique_local(ip: Ipv6Addr) -> bool {
    (ip.octets()[0] & 0xfe) == 0xfc
}

fn is_link_local_v6(ip: Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xffc0) == 0xfe80
}

fn host_name_blocked(domain: &str) -> bool {
    let host = domain.trim_end_matches('.').to_ascii_lowercase();
    host == "localhost"
        || host.ends_with(".localhost")
        || host == "metadata.google.internal"
        || host.ends_with(".metadata.google.internal")
}

fn not_allowed() -> ActionError {
    util::invalid("url is not allowed")
}

fn sanitize_content_type(raw: &str) -> String {
    let base = raw.split(';').next().unwrap_or("").trim();
    if base.is_empty()
        || base.len() > 128
        || !base
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && byte != b'"' && byte != b'\\')
    {
        "application/octet-stream".to_owned()
    } else {
        base.to_ascii_lowercase()
    }
}

fn response_text(content_type: &str, bytes: &[u8]) -> (String, String) {
    let ct = content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if ct.starts_with("image/")
        || ct.starts_with("audio/")
        || ct.starts_with("video/")
        || ct.contains("octet-stream")
    {
        return (String::new(), String::new());
    }
    let raw = String::from_utf8_lossy(bytes);
    if ct.contains("html") || raw.trim_start().starts_with('<') {
        (extract_title(&raw), html_to_text(&raw))
    } else {
        (String::new(), collapse_ws(&raw))
    }
}

fn extract_title(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let Some(start) = lower.find("<title") else {
        return String::new();
    };
    let Some(gt) = lower[start..].find('>') else {
        return String::new();
    };
    let content_start = start + gt + 1;
    let Some(end_rel) = lower[content_start..].find("</title>") else {
        return String::new();
    };
    collapse_ws(&decode_entities(
        html[content_start..content_start + end_rel].trim(),
    ))
}

fn html_to_text(html: &str) -> String {
    let without_script = strip_tag_blocks(html, "script");
    let without_style = strip_tag_blocks(&without_script, "style");
    let mut out = String::new();
    let mut in_tag = false;
    for ch in without_style.chars() {
        if ch == '<' {
            in_tag = true;
            continue;
        }
        if ch == '>' && in_tag {
            in_tag = false;
            out.push(' ');
            continue;
        }
        if !in_tag {
            out.push(ch);
        }
    }
    collapse_ws(&decode_entities(&out))
}

fn strip_tag_blocks(input: &str, tag: &str) -> String {
    let lower = input.to_ascii_lowercase();
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut out = String::with_capacity(input.len());
    let mut rest = 0;
    while let Some(rel) = lower[rest..].find(&open) {
        let start = rest + rel;
        out.push_str(&input[rest..start]);
        if let Some(end_rel) = lower[start..].find(&close) {
            rest = start + end_rel + close.len();
        } else {
            return out;
        }
    }
    out.push_str(&input[rest..]);
    out
}

fn decode_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let Some(end) = rest.find(';') else {
            out.push_str(rest);
            return out;
        };
        if end > 12 {
            out.push('&');
            rest = &rest[1..];
            continue;
        }
        let entity = &rest[1..end];
        if let Some(ch) = named_or_numeric(entity) {
            out.push(ch);
            rest = &rest[end + 1..];
        } else {
            out.push('&');
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    out
}

fn named_or_numeric(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" | "#39" => Some('\''),
        "nbsp" => Some(' '),
        _ => numeric_entity(entity),
    }
}

fn numeric_entity(entity: &str) -> Option<char> {
    let code = if let Some(hex) = entity
        .strip_prefix("#x")
        .or_else(|| entity.strip_prefix("#X"))
    {
        u32::from_str_radix(hex, 16).ok()?
    } else {
        entity.strip_prefix('#')?.parse().ok()?
    };
    char::from_u32(code).filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\t')
}

fn collapse_ws(input: &str) -> String {
    let mut out = String::new();
    let mut pending = false;
    for ch in input.chars() {
        if ch.is_whitespace() {
            pending = !out.is_empty();
            continue;
        }
        if pending {
            out.push(' ');
            pending = false;
        }
        out.push(ch);
    }
    out
}
