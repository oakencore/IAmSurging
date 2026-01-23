use clap::{Parser, Subcommand};
use i_am_surging::{FeedLoader, Result, Surge, SurgeClient, SurgeEvent};
use std::process;

#[derive(Parser)]
#[command(
    name = "surge",
    about = "Get real-time crypto prices from Switchboard Surge",
    version,
    after_help = "EXAMPLES:
    surge btc              Get BTC/USD price
    surge btc eth sol      Get multiple prices
    surge stream btc eth   Stream live prices
    surge list             List all 2000+ supported symbols"
)]
struct Cli {
    /// Output as JSON
    #[arg(short, long)]
    json: bool,

    #[command(subcommand)]
    command: Option<Commands>,

    /// Symbols to fetch (e.g., btc, eth, sol/usdt)
    #[arg(trailing_var_arg = true)]
    symbols: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Stream real-time prices via WebSocket
    Stream {
        /// Symbols to stream
        symbols: Vec<String>,
    },
    /// List available symbols
    List {
        /// Filter by substring
        #[arg(short, long)]
        filter: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    let json = cli.json;

    match cli.command {
        Some(Commands::List { filter }) => {
            let loader = FeedLoader::load_default()?;
            let mut symbols = loader.get_all_symbols();

            if let Some(ref filter_term) = filter {
                let filter_lower = filter_term.to_lowercase();
                symbols.retain(|s| s.to_lowercase().contains(&filter_lower));
            }

            if json {
                println!("{}", serde_json::to_string_pretty(&symbols)?);
            } else {
                for s in &symbols {
                    println!("{}", s);
                }
                eprintln!("\n{} symbols", symbols.len());
            }
        }

        Some(Commands::Stream { symbols }) => {
            if symbols.is_empty() {
                eprintln!("Usage: surge stream <SYMBOLS>...");
                eprintln!("Example: surge stream btc eth sol");
                process::exit(1);
            }

            let symbol_refs: Vec<&str> = symbols.iter().map(String::as_str).collect();
            let mut surge = Surge::new(""); // API key not required
            let mut event_rx = surge.subscribe_events();
            surge.connect_and_subscribe(symbol_refs).await?;

            if !json {
                eprintln!("Streaming {} (Ctrl+C to stop)\n", symbols.join(", "));
            }

            while let Ok(event) = event_rx.recv().await {
                match event {
                    SurgeEvent::PriceUpdate(u) => {
                        if json {
                            println!("{}", serde_json::to_string(&u)?);
                        } else {
                            println!("{}: ${:.2}", u.data.symbol, u.data.price);
                        }
                    }
                    SurgeEvent::Error(e) => eprintln!("Error: {}", e),
                    _ => {}
                }
            }
        }

        None => {
            if cli.symbols.is_empty() {
                eprintln!("Usage: surge <SYMBOLS>...");
                eprintln!("Example: surge btc eth sol");
                eprintln!("\nRun 'surge --help' for more options");
                process::exit(1);
            }

            let client = SurgeClient::new()?;
            let symbol_refs: Vec<&str> = cli.symbols.iter().map(String::as_str).collect();
            let prices = client.get_multiple_prices(&symbol_refs).await?;

            if json {
                println!("{}", serde_json::to_string_pretty(&prices)?);
            } else {
                for p in &prices {
                    println!("{}: ${:.2}", p.symbol, p.value);
                }
            }
        }
    }

    Ok(())
}
