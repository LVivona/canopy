use std::{collections::{HashMap, HashSet}, error, fmt, iter, net::IpAddr};
use multiaddr::{FromUrlErr, Multiaddr, Protocol};

// Define a transformation function type
type ProtocolTransformer<'a> = fn(&str, bool) -> Result<Multiaddr, FromUrlErr>;

struct ProtocolType<'a> {
    name: &'a str,
    support: Vec<Protocol<'a>>,
    transformer: ProtocolTransformer<'a>,
}



impl<'a> ProtocolType<'a> {
    fn supports(&self, protocol: Protocol<'a>) -> bool {
        self.support.iter().any(|p| protocol == *p)
    }
    
    fn transform(&self, url: &str, lossy: bool) -> Result<Multiaddr, FromUrlErr> {
        (self.transformer)(url, lossy)
    }
}

// Protocol registry to look up transformers
#[derive(Default)]
struct ProtocolRegistry<'a> {
    protocols: HashMap<&'a str, ProtocolType<'a>>,
}


impl<'a> ProtocolRegistry<'a> {
    pub fn new() -> Self {
        ProtocolRegistry {
            protocols: HashMap::new(),
        }
    }
    
    pub fn get(&self, scheme: &str) -> Option<&ProtocolType<'a>> {
        // Handle custom protocol variants like libp2p+http
        if scheme.contains('+') {
            let parts: Vec<&str> = scheme.split('+').collect();
            if let Some(base) = self.protocols.get(parts[0]) {
                return Some(base);
            }
        }
        
        self.protocols.get(scheme)
    }
    
    pub fn register(&mut self, protocol_type: ProtocolType<'a>) {
        self.protocols.insert(protocol_type.name, protocol_type);
    }

}

pub fn from_custom_url_path<'a>(url: &str, lossy: bool, registry: &ProtocolRegistry<'a>) -> Result<Multiaddr, FromUrlErr> {
    let url_parsed = url::Url::parse(url).map_err(|_| FromUrlErr::BadUrl)?;
    let scheme = url_parsed.scheme();
    
    if let Some(protocol_type) = registry.get(scheme) {
        protocol_type.transform(url, lossy)
    } else {
        Err(FromUrlErr::UnsupportedScheme)
    }
}

/// Called when `url.scheme()` is an Internet-like URL.
fn from_url_inner_http_ws(
    url: &str, 
    lossy: bool,
) -> Result<Multiaddr, FromUrlErr> {
    let url = url::Url::parse(url).map_err(|_| FromUrlErr::BadUrl)?;
    
    let (protocol, lost_path, default_port) = match url.scheme() {
        "ws" => (Protocol::Ws(url.path().to_owned().into()), false, 80),
        "wss" => (Protocol::Wss(url.path().to_owned().into()), false, 443),
        "http" => (Protocol::Http, true, 80),
        "https" => (Protocol::Https, true, 443),
        _ => unreachable!("We only call this function for one of the given schemes; qed"),
    };
    
    let port = Protocol::Tcp(url.port().unwrap_or(default_port));
    let ip = if let Some(hostname) = url.host_str() {
        if let Ok(ip) = hostname.parse::<IpAddr>() {
            Protocol::from(ip)
        } else {
            Protocol::Dns(hostname.into())
        }
    } else {
        return Err(FromUrlErr::BadUrl);
    };
    
    if !lossy
        && (!url.username().is_empty()
            || url.password().is_some()
            || (lost_path && url.path() != "/" && !url.path().is_empty())
            || url.query().is_some()
            || url.fragment().is_some())
    {
        return Err(FromUrlErr::InformationLoss);
    }
    
    Ok(iter::once(ip)
        .chain(iter::once(port))
        .chain(iter::once(protocol))
        .collect())
}

/// Called when `url.scheme()` is a path-like URL.
fn from_url_inner_path(url: &str, lossy: bool) -> Result<Multiaddr, FromUrlErr> {
    let url = url::Url::parse(url).map_err(|_| FromUrlErr::BadUrl)?;
    
    let protocol = match url.scheme() {
        "unix" => Protocol::Unix(url.path().to_owned().into()),
        _ => unreachable!("We only call this function for one of the given schemes; qed"),
    };
    
    if !lossy
        && (!url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some())
    {
        return Err(FromUrlErr::InformationLoss);
    }
    
    Ok(Multiaddr::from(protocol))
}

#[cfg(test)]
mod test {
    use super::*;
    

    // Custom transformer for libp2p protocol
    fn transform_libp2p(url: &str, lossy: bool) -> Result<Multiaddr, FromUrlErr> {
    let url_parsed = url::Url::parse(url).map_err(|_| FromUrlErr::BadUrl)?;
    let scheme = url_parsed.scheme();
    
    // Check if this is a variant
    let is_http_variant = scheme.contains("+http");
    let is_https_variant = scheme.contains("+https");
    
    // Default port for standard libp2p websocket
    let default_port = 432;
    
    let port = url_parsed.port().unwrap_or(default_port);
    let hostname = url_parsed.host_str().ok_or(FromUrlErr::BadUrl)?;
    
    let ip = if let Ok(ip) = hostname.parse::<IpAddr>() {
        Protocol::from(ip)
    } else {
        Protocol::Dns(hostname.into())
    };
    
    // Build the multiaddr
    // For libp2p:// -> ip4/0.0.0.0/tcp/432/ws
    // For libp2p+http:// -> ip4/0.0.0.0/tcp/443/http
    let protocol = if is_http_variant {
        Protocol::Http
    } else if is_https_variant {
        Protocol::Https
    } else {
        Protocol::Ws(url_parsed.path().to_owned().into())
    };
    
    Ok(iter::once(ip)
        .chain(iter::once(Protocol::Tcp(port)))
        .chain(iter::once(protocol))
        .collect())
}

    #[test]
    fn custom_protocol_test() -> Result<(), FromUrlErr> {
        let mut registry = ProtocolRegistry::new();
        
        registry.register(ProtocolType {
           name : "p2p",
           support : vec![Protocol::Ws("".into()), Protocol::Tcp(0)],
           transformer : transform_libp2p 
        });

        // Test libp2p protocol
        let addr = from_custom_url_path("p2p://example.com/", false, &registry)?;
        println!("p2p: {:?}", addr);
        // Should produce: /dns/example.com/tcp/432/ws/
        
        // Test libp2p+http variant
        let addr = from_custom_url_path("p2p+http://example.com/", false, &registry)?;
        println!("p2p+http: {:?}", addr);
        // Should produce: /dns/example.com/tcp/443/http
        
        Ok(())
    }

}