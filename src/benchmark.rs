use crate::dns_provider::DnsProvider;
use anyhow::Result;
use std::time::Instant;
use std::net::SocketAddr;
use std::sync::Arc;
use hickory_resolver::config::{ResolverConfig, ResolverOpts, NameServerConfig, Protocol};
use hickory_resolver::TokioAsyncResolver;
use hickory_resolver::Name;
use tokio::time::timeout;
use std::time::Duration;

pub async fn benchmark_provider(provider: &mut DnsProvider) -> Result<()> {
    let socket_addr: SocketAddr = format!("{}:53", provider.ipv4).parse()?;
    let ns_config = NameServerConfig::new(socket_addr, Protocol::Udp);
    
    let mut opts = ResolverOpts::default();
    opts.timeout = Duration::from_secs(3);
    opts.attempts = 2;
    
    let config = ResolverConfig::from_parts(None, vec![], vec![ns_config]);
    let resolver = TokioAsyncResolver::tokio(config, opts);
    let resolver = Arc::new(resolver);

    let domains = vec![
        "google.com",
        "cloudflare.com",
        "microsoft.com",
        "github.com",
        "netflix.com"
    ];
    let mut total_time = 0.0;
    let mut successful_queries = 0;

    for domain in domains.iter() {
        let domain_name = match Name::from_ascii(domain) {
            Ok(name) => name,
            Err(_) => continue,
        };

        let start = Instant::now();
        match timeout(Duration::from_secs(5), resolver.lookup_ip(domain_name)).await {
            Ok(Ok(_)) => {
                let elapsed = start.elapsed().as_secs_f64();
                total_time += elapsed;
                successful_queries += 1;
            }
            _ => continue,
        }
    }

    if successful_queries > 0 {
        provider.avg_response_time = Some(total_time / successful_queries as f64);
    }
    
    Ok(())
}
