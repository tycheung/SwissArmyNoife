//! Quickstart sketch (`sak324-a`) — run against a live `http-admin`.
//!
//! ```bash
//! cargo run -p http-admin
//! cargo run -p sdk --example quickstart
//! ```

use sdk::SakClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = std::env::var("SAK_HTTP").unwrap_or_else(|_| "http://127.0.0.1:8787".into());
    let client = SakClient::new(base);
    println!("health: {}", client.health().await?);
    println!("modules: {}", client.list_modules().await?);
    println!("capacity: {}", client.capacity().await?);
    println!("work: {}", client.list_work().await?);
    Ok(())
}
