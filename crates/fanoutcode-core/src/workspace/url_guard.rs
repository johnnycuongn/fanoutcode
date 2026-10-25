//! Where a context doc may be fetched from. Loopback, link-local (including the 169.254.169.254 metadata address),
//! unspecified and multicast addresses are always refused; private ranges (intranets) only for a doc the developer
//! added or confirmed. Every hop of a fetch is resolved, checked and then pinned to the checked address, so a DNS
//! answer cannot change between the check and the connection.

use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};

use crate::{Error, Result};

/// The most redirects a fetch follows.
pub const MAX_HOPS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddrClass {
    /// Never fetched.
    Forbidden,
    /// 10/8, 172.16/12, 192.168/16, 100.64/10, fc00::/7: only for trusted docs.
    Private,
    Public,
}

fn classify_v4(ip: Ipv4Addr) -> AddrClass {
    let o = ip.octets();
    if ip.is_loopback()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_multicast()
        || ip.is_broadcast()
        || o[0] == 0
    {
        AddrClass::Forbidden
    } else if ip.is_private() || (o[0] == 100 && (o[1] & 0xc0) == 64) {
        AddrClass::Private
    } else {
        AddrClass::Public
    }
}

pub fn classify(ip: IpAddr) -> AddrClass {
    match ip {
        IpAddr::V4(v4) => classify_v4(v4),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return classify_v4(v4);
            }
            let seg = v6.segments();
            if v6.is_loopback() || v6.is_unspecified() || v6.is_multicast() || (seg[0] & 0xffc0) == 0xfe80 {
                AddrClass::Forbidden
            } else if (seg[0] & 0xfe00) == 0xfc00 {
                AddrClass::Private
            } else {
                AddrClass::Public
            }
        }
    }
}

/// Resolves a host for a fetch; injectable for tests.
pub trait Resolve: Send + Sync {
    fn resolve(&self, host: &str, port: u16) -> std::io::Result<Vec<SocketAddr>>;
}

pub struct SystemResolver;

impl Resolve for SystemResolver {
    fn resolve(&self, host: &str, port: u16) -> std::io::Result<Vec<SocketAddr>> {
        Ok((host, port).to_socket_addrs()?.collect())
    }
}

/// The resolver and checks a fetch runs with.
pub struct NetGuard<'a> {
    pub resolver: &'a dyn Resolve,
    /// Tests only: one host whose loopback answer is accepted, so a mock server can stand in for a public site.
    pub(crate) exempt_host: Option<String>,
}

impl NetGuard<'static> {
    pub fn system() -> Self {
        NetGuard {
            resolver: &SystemResolver,
            exempt_host: None,
        }
    }
}

/// A host as compared: lower case, no trailing dot, IPv6 brackets removed.
pub fn normalise_host(host: &str) -> String {
    host.trim_end_matches('.')
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_ascii_lowercase()
}

fn refuse(host: &str, why: &str) -> Error {
    Error::Precondition(format!("Fanout Code won't fetch {host}: {why}."))
}

/// Checks without resolving (adding or importing a doc): an http(s) URL whose host is not `localhost` and, when it
/// is an address literal, not a forbidden one (nor a private one for an untrusted doc).
pub fn check_lexical(url: &str, trusted: bool) -> Result<()> {
    let u = url::Url::parse(url).map_err(|_| Error::Invalid("A link must start with http:// or https://".into()))?;
    if !matches!(u.scheme(), "http" | "https") {
        return Err(Error::Invalid("A link must start with http:// or https://".into()));
    }
    let host = normalise_host(u.host_str().unwrap_or_default());
    if host.is_empty() || host == "localhost" || host.ends_with(".localhost") {
        return Err(refuse(&host, "it is this computer"));
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        check_addrs(&host, &[ip], trusted)?;
    }
    Ok(())
}

/// Refuses when any address is forbidden, or private for an untrusted doc.
pub fn check_addrs(host: &str, addrs: &[IpAddr], trusted: bool) -> Result<()> {
    if addrs.is_empty() {
        return Err(Error::Provider(format!("Couldn't resolve {host}")));
    }
    for ip in addrs {
        match classify(*ip) {
            AddrClass::Forbidden => return Err(refuse(host, "it is a local or link-local address")),
            AddrClass::Private if !trusted => {
                return Err(refuse(
                    host,
                    "it is on a private network and this reference came from a workspace file; confirm it in the \
                     workspace's references or add it yourself",
                ))
            }
            _ => {}
        }
    }
    Ok(())
}

/// Validates one hop and returns the host and the address to pin it to (`None` for an address literal).
pub fn validate_hop(url: &url::Url, trusted: bool, guard: &NetGuard) -> Result<(String, Option<SocketAddr>)> {
    check_lexical(url.as_str(), trusted)?;
    let host = normalise_host(url.host_str().unwrap_or_default());
    if host.parse::<IpAddr>().is_ok() {
        return Ok((host, None));
    }
    let port = url.port_or_known_default().unwrap_or(443);
    let addrs = guard
        .resolver
        .resolve(&host, port)
        .map_err(|e| Error::Provider(format!("Couldn't resolve {host}: {e}")))?;
    if guard.exempt_host.as_deref() != Some(host.as_str()) {
        check_addrs(&host, &addrs.iter().map(SocketAddr::ip).collect::<Vec<_>>(), trusted)?;
    }
    let first = *addrs
        .first()
        .ok_or_else(|| Error::Provider(format!("Couldn't resolve {host}")))?;
    Ok((host, Some(first)))
}

/// How one hop's client is built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientPlan {
    /// The system proxy is used only for a trusted hop to the vetted host (corporate networks need it); the proxy
    /// resolves the name itself, so the address pin does not hold on that path.
    pub use_proxy: bool,
    /// The checked address the connection is pinned to (not for address literals). Through a proxy the proxy
    /// resolves the name, so the pin holds only when no proxy is configured.
    pub pin: Option<SocketAddr>,
}

/// Trust covers only the doc's own host (normalised): a hop anywhere else is checked as untrusted.
pub fn hop_trusted(trusted: bool, origin_host: &str, hop_host: &str) -> bool {
    trusted && normalise_host(origin_host) == normalise_host(hop_host)
}

/// An untrusted hop never goes through a proxy, so the pin and the address checks hold.
pub fn client_plan(trusted_hop: bool, addr: Option<SocketAddr>) -> ClientPlan {
    ClientPlan {
        use_proxy: trusted_hop,
        pin: addr,
    }
}

/// A client for one hop: no automatic redirects; per `client_plan`, no proxy for an untrusted hop, and the
/// connection pinned to the checked address.
pub fn client_for(host: &str, plan: ClientPlan) -> Result<reqwest::Client> {
    let mut b = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(15));
    if !plan.use_proxy {
        b = b.no_proxy();
    }
    if let Some(a) = plan.pin {
        b = b.resolve(host, a);
    }
    b.build().map_err(|e| Error::Provider(format!("Fetching {host}: {e}")))
}

/// GET `url`, following up to `MAX_HOPS` redirects, each hop checked (trusted only on the doc's own host) and, when
/// untrusted, pinned with no proxy. Returns the final response.
pub async fn guarded_get(url: &str, trusted: bool, guard: &NetGuard<'_>) -> Result<reqwest::Response> {
    let mut current =
        url::Url::parse(url).map_err(|_| Error::Invalid("A link must start with http:// or https://".into()))?;
    let origin = normalise_host(current.host_str().unwrap_or_default());
    for _ in 0..=MAX_HOPS {
        let hop_host = normalise_host(current.host_str().unwrap_or_default());
        let trusted_hop = hop_trusted(trusted, &origin, &hop_host);
        let (host, addr) = validate_hop(&current, trusted_hop, guard)?;
        let resp = client_for(&host, client_plan(trusted_hop, addr))?
            .get(current.clone())
            .send()
            .await
            .map_err(|e| Error::Provider(format!("Fetching {host}: {}", e.without_url())))?;
        if !resp.status().is_redirection() {
            return Ok(resp);
        }
        let location = resp
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| Error::Provider(format!("Fetching {host}: a redirect without a location")))?;
        current = current
            .join(location)
            .map_err(|_| Error::Provider(format!("Fetching {host}: a bad redirect")))?;
    }
    Err(Error::Provider(format!(
        "Fetching {url}: more than {MAX_HOPS} redirects"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification() {
        for (ip, class) in [
            ("127.0.0.1", AddrClass::Forbidden),
            ("169.254.169.254", AddrClass::Forbidden),
            ("0.0.0.0", AddrClass::Forbidden),
            ("224.0.0.1", AddrClass::Forbidden),
            ("::1", AddrClass::Forbidden),
            ("fe80::1", AddrClass::Forbidden),
            ("::ffff:127.0.0.1", AddrClass::Forbidden),
            ("::ffff:169.254.169.254", AddrClass::Forbidden),
            ("10.1.2.3", AddrClass::Private),
            ("172.16.0.1", AddrClass::Private),
            ("192.168.1.1", AddrClass::Private),
            ("100.64.0.1", AddrClass::Private),
            ("fd00::1", AddrClass::Private),
            ("93.184.216.34", AddrClass::Public),
            ("2606:4700::1", AddrClass::Public),
        ] {
            assert_eq!(classify(ip.parse().unwrap()), class, "{ip}");
        }
    }

    #[test]
    fn lexical_checks() {
        assert!(check_lexical("http://localhost:8080/x", true).is_err());
        assert!(check_lexical("http://LOCALHOST./x", true).is_err());
        assert!(check_lexical("http://169.254.169.254/latest", true).is_err());
        assert!(check_lexical("http://[::1]/", true).is_err());
        assert!(check_lexical("http://10.0.0.5/wiki", false).is_err());
        assert!(check_lexical("http://10.0.0.5/wiki", true).is_ok());
        assert!(check_lexical("https://example.com/spec", false).is_ok());
        assert!(check_lexical("file:///etc/passwd", true).is_err());
    }

    #[test]
    fn trust_covers_only_the_original_host_and_untrusted_hops_skip_the_proxy() {
        assert!(hop_trusted(true, "Wiki.Corp.", "wiki.corp"));
        assert!(!hop_trusted(true, "wiki.corp", "other.corp"));
        assert!(!hop_trusted(false, "wiki.corp", "wiki.corp"));
        let addr: SocketAddr = "10.0.0.5:443".parse().unwrap();
        assert_eq!(
            client_plan(false, Some(addr)),
            ClientPlan {
                use_proxy: false,
                pin: Some(addr)
            }
        );
        assert_eq!(
            client_plan(true, Some(addr)),
            ClientPlan {
                use_proxy: true,
                pin: Some(addr)
            }
        );
    }
}
