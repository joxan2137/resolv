mod benchmark;
mod dns_provider;

use anyhow::Result;
use futures::future::join_all;
use indicatif::{ProgressBar, ProgressStyle};

use num_cpus;

#[tokio::main]
async fn main() -> Result<()> {
    let mut providers = dns_provider::get_providers().await?;
    println!("Loaded {} DNS providers", providers.len());

    let pb = ProgressBar::new(providers.len() as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} ({eta})")?);

    // Calculate optimal chunk size based on CPU cores
    let num_cpus = num_cpus::get();
    let chunk_size = (providers.len() / num_cpus).max(50);
    let mut results = Vec::new();

    for chunk in providers.chunks_mut(chunk_size) {
        let chunk_futures: Vec<_> = chunk
            .iter_mut()
            .map(|provider| {
                let pb = pb.clone();
                async move {
                    let result = benchmark::benchmark_provider(provider).await;
                    pb.inc(1);
                    result
                }
            })
            .collect();

        results.extend(join_all(chunk_futures).await);
    }

    pb.finish_with_message("Benchmarking complete");

    let mut valid_providers: Vec<_> = providers
        .into_iter()
        .filter(|p| p.avg_response_time.is_some())
        .collect();

    valid_providers.sort_by(|a, b| {
        a.avg_response_time.unwrap().partial_cmp(&b.avg_response_time.unwrap()).unwrap()
    });

    println!("\nTop 10 Fastest DNS Providers:");
    println!("-----------------------------");
    for provider in valid_providers.iter().take(10) {
        println!(
            "{} by {} ({}):",
            provider.name,
            provider.organization,
            provider.location
        );
        println!(
            "  IPv4: {} - {:.3}ms",
            provider.ipv4,
            provider.avg_response_time.unwrap() * 1000.0
        );
        if let Some(ipv6) = &provider.ipv6 {
            println!("  IPv6: {}", ipv6);
        }
        println!();
    }

    println!("\nTotal working DNS servers: {}", valid_providers.len());
    Ok(())
}
