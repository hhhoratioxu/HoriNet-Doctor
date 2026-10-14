//! Companion CLI for reproducible diagnostics. Exports are always redacted.
use horinet_core::{
    diagnostics,
    models::Settings,
    network::{connectivity, dns, icmp, traceroute},
    reports, Result,
};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
async fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let token = CancellationToken::new();
    let cancel = token.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        cancel.cancel();
    });
    let timeout = Duration::from_secs(3);
    match args.first().map(String::as_str) {
        Some("diagnose") => {
            let report = diagnostics::run(Settings::default(),token,|n,c| eprintln!("{n}/16 {}: {:?}",c.id,c.status)).await?;
            println!("{}",reports::json(&report,false)?);
        },
        Some("snapshot") => {
            let snapshot = diagnostics::inspect(&token).await?;
            // Counts only; no addresses, MACs, SSIDs or proxy endpoints in console.
            println!("adapters={} up={} routes={} system_proxy={} warnings={}",snapshot.adapters.len(),snapshot.adapters.iter().filter(|a|a.up).count(),snapshot.routes.len(),snapshot.proxy.system_enabled,snapshot.warnings.len());
        },
        Some("tcp") if args.len() == 2 => println!("TCP {:.2} ms",connectivity::tcp(horinet_core::validation::endpoint(&args[1])?,timeout,&token).await?),
        Some("ping") if args.len() == 2 => { let addr = connectivity::resolve(&args[1],443,timeout,&token).await?[0]; let r = icmp::ping(addr.ip(),64,timeout,&token).await?; println!("ICMP status={} reached={} rtt_ms={:?}",r.status_code,r.reached,r.rtt_ms); },
        Some("dns") if args.len() == 4 => { let r = dns::query(&args[1],&args[2],&args[3],false,timeout,&token).await?; println!("{} records={} time_ms={:.2} error={:?}",r.transport,r.records.len(),r.elapsed_ms,r.error); },
        Some("trace") if args.len() == 3 => { let hops = traceroute::trace(&args[1],args[2] == "6",30,timeout,&token,|h| println!("hop={} replied={} rtt_ms={:?}",h.ttl,h.reply.address.is_some(),h.reply.rtt_ms)).await?; println!("hops={}",hops.len()); },
        _ => println!("HoriNet Doctor 0.1.0-beta.1\nCommands: snapshot | diagnose | tcp IP:PORT | ping HOST | dns DOMAIN IP:53 udp|tcp | trace HOST 4|6\nAll diagnose exports are redacted. Ctrl+C cancels. Network tests are direct and user initiated."),
    }
    Ok(())
}
