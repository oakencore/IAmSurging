use clap::{Parser, Subcommand};
use i_am_surging::{Result, Surge, SurgeClient, SurgeEvent};

#[derive(Parser)]
#[command(name = "surge-cli")]
#[command(about = "Fetch cryptocurrency prices from Switchboard Surge", long_about = None)]
struct Cli {
    /// API key (or set SURGE_API_KEY environment variable)
    #[arg(long)]
    api_key: Option<String>,

    /// Output format
    #[arg(short, long, default_value = "pretty")]
    format: OutputFormat,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Copy)]
enum OutputFormat {
    Pretty,
    Json,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pretty" => Ok(OutputFormat::Pretty),
            "json" => Ok(OutputFormat::Json),
            _ => Err(format!("Invalid format: {}. Use 'pretty' or 'json'", s)),
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Get the latest price for a single symbol
    Get {
        /// Symbol to fetch (e.g., BTC/USD)
        symbol: String,
    },
    /// Get prices for multiple symbols
    GetMultiple {
        /// Symbols to fetch (e.g., BTC/USD ETH/USD SOL/USD)
        symbols: Vec<String>,
    },
    /// Stream real-time prices via WebSocket
    Stream {
        /// Symbols to stream (e.g., BTC/USD ETH/USD SOL/USD)
        symbols: Vec<String>,
    },
    /// List all available symbols
    List {
        /// Filter symbols (case-insensitive substring match)
        #[arg(short, long)]
        filter: Option<String>,

        /// Limit number of results
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Fetch available Surge feeds from API
    Feeds,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let format = cli.format;

    // Check if this is a list command (doesn't need API key)
    if matches!(cli.command, Commands::List { .. }) {
        if let Commands::List { filter, limit } = cli.command {
            // List command doesn't need API key, just load the feed data
            let feed_loader = i_am_surging::FeedLoader::load_default()?;
            let mut symbols = feed_loader.get_all_symbols();

            // Apply filter if provided
            if let Some(ref filter_str) = filter {
                let filter_lower = filter_str.to_lowercase();
                symbols.retain(|s| s.to_lowercase().contains(&filter_lower));
            }

            // Apply limit if provided
            if let Some(limit_count) = limit {
                symbols.truncate(limit_count);
            }

            match format {
                OutputFormat::Pretty => {
                    println!("Available Symbols");
                    if let Some(ref f) = filter {
                        println!("   Filter: {}", f);
                    }
                    if let Some(l) = limit {
                        println!("   Limit: {}", l);
                    }
                    println!("{}", "-".repeat(50));
                    println!();

                    for (i, symbol) in symbols.iter().enumerate() {
                        println!("{:4}. {}", i + 1, symbol);
                    }

                    println!();
                    println!("Total: {} symbols", symbols.len());
                }
                OutputFormat::Json => {
                    let json = serde_json::to_string_pretty(&symbols)?;
                    println!("{}", json);
                }
            }

            return Ok(());
        }
    }

    // For other commands, we need an API key
    let api_key = cli.api_key
        .or_else(|| std::env::var("SURGE_API_KEY").ok())
        .expect("API key must be provided via --api-key or SURGE_API_KEY environment variable");

    let client = SurgeClient::new(&api_key)?;

    match cli.command {
        Commands::Get { symbol } => {
            let price = client.get_price(&symbol).await?;

            match format {
                OutputFormat::Pretty => {
                    println!("{} Price", symbol);
                    println!("{}", "-".repeat(50));
                    println!("Price:   ${:.6}", price.value);
                    println!("Feed ID: {}", price.feed_id);
                }
                OutputFormat::Json => {
                    let json = serde_json::to_string_pretty(&price)?;
                    println!("{}", json);
                }
            }
        }

        Commands::GetMultiple { symbols } => {
            let symbol_refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();
            let prices = client.get_multiple_prices(&symbol_refs).await?;

            match format {
                OutputFormat::Pretty => {
                    println!("Multiple Prices");
                    println!("{}", "=".repeat(50));
                    for price in &prices {
                        println!();
                        println!("Symbol:  {}", price.symbol);
                        println!("Price:   ${:.6}", price.value);
                        println!("Feed ID: {}", price.feed_id);
                    }
                    println!();
                    println!("Total: {} prices fetched", prices.len());
                }
                OutputFormat::Json => {
                    let json = serde_json::to_string_pretty(&prices)?;
                    println!("{}", json);
                }
            }
        }

        Commands::Stream { symbols } => {
            let symbol_refs: Vec<&str> = symbols.iter().map(|s| s.as_str()).collect();

            match format {
                OutputFormat::Pretty => {
                    println!("Streaming prices for: {}", symbols.join(", "));
                    println!("{}", "=".repeat(50));
                    println!("Press Ctrl+C to stop\n");
                }
                OutputFormat::Json => {}
            }

            let mut surge = Surge::new(&api_key);
            let mut rx = surge.subscribe_events();

            surge.connect_and_subscribe(symbol_refs).await?;

            // Handle events
            loop {
                match rx.recv().await {
                    Ok(event) => match event {
                        SurgeEvent::Connected => {
                            if matches!(format, OutputFormat::Pretty) {
                                println!("Connected to Surge\n");
                            }
                        }
                        SurgeEvent::PriceUpdate(update) => {
                            match format {
                                OutputFormat::Pretty => {
                                    println!(
                                        "{}: ${:.6} ({}ms)",
                                        update.data.symbol,
                                        update.data.price,
                                        update.data.source_timestamp_ms
                                    );
                                }
                                OutputFormat::Json => {
                                    if let Ok(json) = serde_json::to_string(&update) {
                                        println!("{}", json);
                                    }
                                }
                            }
                        }
                        SurgeEvent::Error(e) => {
                            eprintln!("Error: {}", e);
                        }
                        SurgeEvent::Disconnected => {
                            if matches!(format, OutputFormat::Pretty) {
                                println!("\nDisconnected");
                            }
                        }
                        SurgeEvent::Reconnecting { attempt, delay_ms } => {
                            if matches!(format, OutputFormat::Pretty) {
                                println!("Reconnecting (attempt {}, delay {}ms)", attempt, delay_ms);
                            }
                        }
                    },
                    Err(_) => break,
                }
            }
        }

        Commands::Feeds => {
            let surge = Surge::new(&api_key);
            let feeds = surge.get_surge_feeds().await?;

            match format {
                OutputFormat::Pretty => {
                    println!("Available Surge Feeds");
                    println!("{}", "=".repeat(50));
                    println!();

                    for (i, feed) in feeds.iter().enumerate() {
                        println!("{:4}. {}", i + 1, feed.symbol);
                        if let Some(id) = &feed.feed_id {
                            println!("      Feed ID: {}", id);
                        }
                    }

                    println!();
                    println!("Total: {} feeds", feeds.len());
                }
                OutputFormat::Json => {
                    let json = serde_json::to_string_pretty(&feeds)?;
                    println!("{}", json);
                }
            }
        }

        Commands::List { .. } => {
            // Already handled above
            unreachable!()
        }
    }

    Ok(())
}
