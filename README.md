# I Am Surging

A simple Rust client for getting live cryptocurrency prices from Switchboard Surge.

**What is this?** This tool is a simple wrapper, letting you fetch real time prices for Bitcoin, Ethereum, Solana, and 2,000+ other cryptocurrencies directly from your terminal or Rust application.

**This is an unofficial, personal project and is not affiliated with or endorsed by Switchboard.** For official documentation, visit https://docs.switchboard.xyz

---

## What You Can Do

- **Get live crypto prices** – Fetch current prices for BTC, ETH, SOL, and 2,266 other trading pairs
- **Use from terminal** – Simple CLI interface
- **Integrate with Rust** – Use as a library in your Rust projects
- **JSON output** – Get data in JSON format for processing

---

## Quick Start

### 1. Prerequisites

You'll need:
- **Rust** (version 1.70 or later) 
- **Node.js** (version 18 or later) 
- **Switchboard Surge API key / Use your Solana Wallet to get one** – Use your Solana wallet to get one at [switchboard.xyz](https://explorer.switchboardlabs.xyz/subscriptions) Once subscribed, your wallet's public key (address) becomes your 'API Key'.

### 2. Installation

```bash
# Clone or download this project, then:
npm install                        # Install dependencies
cargo build --release              # Build the project
```

### 3. Set Your API Key

```bash
export SURGE_API_KEY="your-api-key-here"
```

---

## Usage

### Get a Single Price

```bash
./target/release/surge-cli get BTC/USD
```

Output:
```
BTC/USD Price
--------------------------------------------------
Price:   $91974.891580
Feed ID: 4cd1cad962425681af07b9254b7d804de3ca3446fbfd1371bb258d2c75059812
```

### Get Multiple Prices

```bash
./target/release/surge-cli get-multiple BTC/USD ETH/USD SOL/USD
```

Output:
```
Multiple Prices
==================================================

Symbol:  BTC/USD
Price:   $91974.891580
Feed ID: 4cd1cad...

Symbol:  ETH/USD
Price:   $3032.460000
Feed ID: a0950ee...

Symbol:  SOL/USD
Price:   $142.150000
Feed ID: 822512e...

Total: 3 prices fetched
```

### Get JSON Output

```bash
./target/release/surge-cli --format json get BTC/USD
```

Output:
```json
{
  "symbol": "BTC/USD",
  "feed_id": "4cd1cad962425681af07b9254b7d804de3ca3446fbfd1371bb258d2c75059812",
  "value": 91974.89158
}
```

### List Available Symbols

```bash
# List all symbols
./target/release/surge-cli list

# Filter by name
./target/release/surge-cli list --filter BTC

# Limit results
./target/release/surge-cli list --limit 10
```

---

## Command Reference

| Command | Description | Example |
|---------|-------------|---------|
| `get <SYMBOL>` | Get price for one symbol | `surge-cli get BTC/USD` |
| `get-multiple <SYMBOLS>` | Get prices for multiple symbols | `surge-cli get-multiple BTC/USD ETH/USD` |
| `list` | List available symbols | `surge-cli list --filter SOL` |

### Options

| Option | Description |
|--------|-------------|
| `--api-key <KEY>` | Your API key (or set `SURGE_API_KEY` environment variable) |
| `--format json` | Output in JSON format |

---

## Use as a Rust Library

Add to your `Cargo.toml`:

```toml
[dependencies]
i-am-surging = { path = "../IAmSurging" }
tokio = { version = "1", features = ["full"] }
```

### Example Code

```rust
use i_am_surging::SurgeClient;

#[tokio::main]
async fn main() -> i_am_surging::Result<()> {
    // Create client with your API key
    let client = SurgeClient::new("your-api-key")?;

    // Get a single price
    let btc = client.get_price("BTC/USD").await?;
    println!("Bitcoin: ${:.2}", btc.value);

    // Get multiple prices
    let prices = client.get_multiple_prices(&["BTC/USD", "ETH/USD", "SOL/USD"]).await?;
    for price in prices {
        println!("{}: ${:.2}", price.symbol, price.value);
    }

    Ok(())
}
```
---

## Troubleshooting

### "API key must be provided"

Set your API key:
```bash
export SURGE_API_KEY="your-api-key"
```

Or pass it directly:
```bash
./target/release/surge-cli --api-key "your-key" get BTC/USD
```

### "Feed not found for symbol"

Check the exact symbol format:
```bash
./target/release/surge-cli list | grep BTC
```

Symbols use the format `BASE/QUOTE` (e.g., `BTC/USD`, not `BTCUSD`).

### "Helper script failed"

Make sure Node.js dependencies are installed:
```bash
npm install
node --version  # Should be 18.0.0 or higher
```

---

## How It Works

1. You request a price (e.g., BTC/USD)
2. The client looks up the feed ID in `feedIds.json`
3. It queries Switchboard's oracle network via the Crossbar API
4. The oracle returns a **weighted average price** from multiple exchanges
5. You receive the aggregated price

The prices are aggregated from multiple sources for reliability and manipulation resistance.

---

## Setup: Creating feedIds.json

You need a `feedIds.json` file that maps trading pairs to their Switchboard feed IDs.

### Option 1: Use the Discovery Tool (Recommended)

Automatically discover and save feed IDs from Switchboard:

```bash
# Interactive mode - choose which feeds to save
node discover-feeds.js

# Save ALL available feeds (952+)
node discover-feeds.js --all

# Search for specific feeds
node discover-feeds.js --search BTC

# List all available feeds
node discover-feeds.js --list
```

The interactive mode lets you:
1. Save all feeds at once
2. Search and select specific feeds
3. View top feeds by popularity

### Option 2: Manual Setup

Browse feeds at the [Switchboard Explorer](https://explorer.switchboardlabs.xyz/) and create `feedIds.json` manually:

```json
{
  "BTC/USD": "4cd1cad962425681af07b9254b7d804de3ca3446fbfd1371bb258d2c75059812",
  "ETH/USD": "a0950ee5ee117b2e2c30f154a69e17bfb489a7610c508dc5f67eb2a14616d8ea",
  "SOL/USD": "822512ee9add93518eca1c105a38422841a76c590db079eebb283deb2c14caa9"
}
```

The key is the symbol (e.g., `BTC/USD`) and the value is the feed ID.

---

## Requirements

The following files must be in your working directory:
- `feedIds.json` – Your symbol to feed ID mappings (create using `discover-feeds.js`)
- `fetch-price.js` – Helper script for fetching prices (included)
- `discover-feeds.js` – Feed discovery tool (included)

---

## Licence

MIT
