use crate::{models::Settings, Error, Result};
use std::net::{IpAddr, SocketAddr};

pub fn host(value: &str) -> Result<&str> {
    if value.is_empty()
        || value.len() > 253
        || value.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return Err(Error::Invalid("host must be 1..253 characters".into()));
    }
    if value.parse::<IpAddr>().is_ok() {
        return Ok(value);
    }
    let value = value.trim_end_matches('.');
    if value.split('.').any(|label| {
        label.is_empty()
            || label.len() > 63
            || label.starts_with('-')
            || label.ends_with('-')
            || !label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    }) {
        return Err(Error::Invalid(
            "use an IP address or ASCII / punycode hostname (no URLs or shell syntax)".into(),
        ));
    }
    Ok(value)
}
pub fn endpoint(value: &str) -> Result<SocketAddr> {
    if value.len() > 80 {
        return Err(Error::Invalid("endpoint exceeds 80 characters".into()));
    }
    let addr: SocketAddr = value
        .parse()
        .map_err(|_| Error::Invalid("expected IP:port or [IPv6]:port".into()))?;
    if addr.port() == 0
        || addr.ip().is_unspecified()
        || addr.ip().is_multicast()
        || addr.ip() == IpAddr::V4(std::net::Ipv4Addr::BROADCAST)
    {
        return Err(Error::Invalid(
            "unicast address and port 1..65535 required".into(),
        ));
    }
    Ok(addr)
}
pub fn web_url(value: &str, https_only: bool) -> Result<url::Url> {
    if value.len() > 2048 {
        return Err(Error::Invalid("URL too long".into()));
    }
    let url = url::Url::parse(value).map_err(|_| Error::Invalid("invalid URL".into()))?;
    if (!matches!(url.scheme(), "http" | "https") || (https_only && url.scheme() != "https"))
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(Error::Invalid(
            "HTTP(S) URL without credentials or fragment required".into(),
        ));
    }
    Ok(url)
}
/// Explicit IPv4 STUN endpoint, not a URI, URL, list or arbitrary shell argument.
pub fn stun_server(value: &str) -> Result<(&str, u16)> {
    if value.len() > 259 {
        return Err(Error::Invalid(
            "STUN endpoint exceeds 259 characters".into(),
        ));
    }
    let (name, port) = value
        .rsplit_once(':')
        .ok_or_else(|| Error::Invalid("expected STUN hostname:port or IPv4:port".into()))?;
    host(name)?;
    if name.contains(':') || name.parse::<IpAddr>().is_ok_and(|ip| ip.is_ipv6()) {
        return Err(Error::Invalid(
            "NAT discovery currently measures IPv4 UDP only".into(),
        ));
    }
    let port: u16 = port
        .parse()
        .map_err(|_| Error::Invalid("invalid STUN port".into()))?;
    if port == 0 {
        return Err(Error::Invalid("STUN port must be 1..65535".into()));
    }
    if let Ok(ip) = name.parse::<std::net::Ipv4Addr>() {
        endpoint(&SocketAddr::new(ip.into(), port).to_string())?;
    }
    Ok((name, port))
}
pub fn settings(s: &Settings) -> Result<()> {
    if !matches!(s.language.as_str(), "zh-CN" | "en")
        || !matches!(s.theme.as_str(), "dark" | "light")
    {
        return Err(Error::Invalid("unsupported locale or theme".into()));
    }
    if !(500..=10000).contains(&s.timeout_ms)
        || !(1000..=60000).contains(&s.interval_ms)
        || !(1..=500).contains(&s.retention)
    {
        return Err(Error::Invalid(
            "timeout 500..10000 ms, interval 1000..60000 ms, retention 1..500".into(),
        ));
    }
    if !endpoint(&s.tcp_v4)?.is_ipv4() || !endpoint(&s.tcp_v6)?.is_ipv6() {
        return Err(Error::Invalid("IP family mismatch".into()));
    }
    host(&s.domain)?;
    web_url(&s.http_url, false)?;
    web_url(&s.https_url, true)?;
    web_url(&s.doh_url, true)?;
    stun_server(&s.stun_server)?;
    Ok(())
}
