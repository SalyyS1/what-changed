//! Bounded local validation. None of these helpers contact a configured target.

use super::*;
use crate::error::{AppError, AppResult};
use serde::Serialize;
mod recipe;

pub(crate) fn recipe(input: &ScenarioRecipeDto) -> AppResult<()> {
    recipe::validate(input)
}

/// Archival definitions carry no execution or active-evidence authority.
pub(super) fn archived_recipe(input: &ScenarioRecipeDto) -> AppResult<()> {
    recipe::validate_archival(input)
}

pub(super) fn invalid(what: &'static str, reason: &str) -> AppError {
    AppError::InvalidInput {
        what,
        reason: reason.to_owned(),
    }
}

pub(crate) fn uuid(value: &str, what: &'static str) -> AppResult<()> {
    let parsed =
        uuid::Uuid::parse_str(value).map_err(|_| invalid(what, "must be a canonical UUID"))?;
    if parsed.hyphenated().to_string() != value {
        return Err(invalid(what, "must be a canonical lower-case UUID"));
    }
    Ok(())
}

pub(super) fn payload<T: Serialize>(input: &T) -> AppResult<()> {
    let value = serde_json::to_value(input)
        .map_err(|_| invalid("owner payload", "cannot encode typed payload"))?;
    fn bounded(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::String(s) => s.len() <= MAX_TEXT_BYTES,
            serde_json::Value::Array(items) => items.len() <= 1000 && items.iter().all(bounded),
            serde_json::Value::Object(fields) => fields.values().all(bounded),
            _ => true,
        }
    }
    if !bounded(&value) {
        return Err(invalid(
            "owner payload",
            "each text field must be at most 16 KiB and each collection at most 1000 entries",
        ));
    }
    let bytes = serde_json::to_vec(&value)
        .map_err(|_| invalid("owner payload", "cannot encode typed payload"))?;
    if bytes.len() > 1024 * 1024 {
        return Err(invalid("owner payload", "must be at most 1 MiB"));
    }
    Ok(())
}

pub(super) fn required(value: &str, what: &'static str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(invalid(what, "must not be empty"));
    }
    Ok(())
}

pub(super) fn page(input: &OwnerPageInput) -> AppResult<()> {
    if input.limit == 0 || input.limit > MAX_PAGE_SIZE {
        return Err(invalid("page limit", "must be between 1 and 200"));
    }
    if input.after.as_ref().is_some_and(|s| s.len() > 100) {
        return Err(invalid("page cursor", "is invalid"));
    }
    Ok(())
}

pub(super) fn origin(origin: &OriginDto, kind: EnvironmentKind) -> AppResult<String> {
    if origin.port == 0
        || !matches!(origin.scheme.as_str(), "http" | "https")
        || origin.host.is_empty()
        || origin.host.contains(['/', '@', '?', '#', '\\'])
    {
        return Err(invalid(
            "environment origin",
            "requires an exact HTTP(S) host and nonzero port",
        ));
    }
    let text = format!("{}://{}:{}", origin.scheme, origin.host, origin.port);
    let url = tauri::Url::parse(&text)
        .map_err(|_| invalid("environment origin", "must be a valid exact origin"))?;
    safe_url(&url, kind)?;
    if url.host_str() != Some(origin.host.as_str()) {
        return Err(invalid(
            "environment origin",
            "host must use its normalized spelling",
        ));
    }
    Ok(url.origin().ascii_serialization())
}

fn safe_url(url: &tauri::Url, kind: EnvironmentKind) -> AppResult<()> {
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(invalid(
            "preview URL",
            "only HTTP(S) without credentials, query or fragment is supported",
        ));
    }
    let host = url
        .host_str()
        .ok_or_else(|| invalid("preview URL", "requires a host"))?;
    let host_ip = host
        .trim_matches(['[', ']'])
        .parse::<std::net::IpAddr>()
        .map(|ip| match ip {
            std::net::IpAddr::V6(ip) => ip
                .to_ipv4_mapped()
                .map(std::net::IpAddr::V4)
                .unwrap_or(std::net::IpAddr::V6(ip)),
            ip => ip,
        });
    let loopback = host == "localhost" || host_ip.as_ref().is_ok_and(|ip| ip.is_loopback());
    let private = host_ip.as_ref().is_ok_and(|ip| match ip {
        std::net::IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_broadcast()
                || ip.is_multicast()
        }
        std::net::IpAddr::V6(ip) => {
            ip.is_unspecified()
                || ip.is_multicast()
                || (ip.segments()[0] & 0xfe00) == 0xfc00
                || (ip.segments()[0] & 0xffc0) == 0xfe80
        }
    });
    if private
        || (loopback && kind != EnvironmentKind::Local)
        || (!loopback && url.scheme() != "https")
        || (!loopback
            && (host.ends_with(".localhost")
                || host.ends_with(".local")
                || host.ends_with(".internal")
                || !host.contains('.')))
    {
        return Err(invalid("preview URL", "use explicit local loopback or a public HTTPS test origin; private remote origins are unsupported"));
    }
    Ok(())
}

pub(super) fn environment(input: &SaveOwnerEnvironmentInput) -> AppResult<()> {
    required(&input.label, "environment label")?;
    if input.origins.is_empty() || input.origins.len() > 16 {
        return Err(invalid(
            "environment origins",
            "requires between 1 and 16 exact origins",
        ));
    }
    let url = tauri::Url::parse(&input.preview_url)
        .map_err(|_| invalid("preview URL", "must be an absolute HTTP(S) URL"))?;
    safe_url(&url, input.kind)?;
    let mut origins = std::collections::BTreeSet::new();
    for configured in &input.origins {
        if !origins.insert(origin(configured, input.kind)?) {
            return Err(invalid(
                "environment origins",
                "duplicate origins are not allowed",
            ));
        }
    }
    if !origins.contains(&url.origin().ascii_serialization()) {
        return Err(invalid(
            "preview URL",
            "must belong to an exact configured origin",
        ));
    }
    Ok(())
}
