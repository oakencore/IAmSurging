# I Am Surging

The easiest way to get live crypto prices from [Switchboard Surge](https://switchboard.xyz).

## Quick Start

```bash
# Clone the repository
git clone https://github.com/you/IAmSurging.git
cd IAmSurging

# Run the setup script (generates feedIds.json, creates .env)
./scripts/setup.sh

# Or for non-interactive setup with defaults
./scripts/setup.sh --quick
```

The setup script will:
1. Check for required dependencies (Node.js 18+)
2. Create `.env` from the template
3. Generate `feedIds.json` with 2,000+ trading pairs from Switchboard
4. Optionally build the project

### Setup Options

```bash
# Set API key during setup (for server authentication)
./scripts/setup.sh --api-key your-secret-key

# Use a custom Switchboard API endpoint
./scripts/setup.sh --feeds-api https://custom.api.url/feeds

# Show all options
./scripts/setup.sh --help
```

## Install

```bash
cargo install --path .
```

## CLI Usage

```bash
# Get a price (shortcuts work: btc = BTC/USD)
surge btc
# BTC/USD: $89846.94

# Get multiple prices
surge btc eth sol
# BTC/USD: $89846.94
# ETH/USD: $2999.88
# SOL/USD: $129.68

# Stream live prices
surge stream btc eth

# List all 2000+ symbols
surge list
surge list --filter sol
```

### JSON Output

```bash
surge --json btc eth sol
```

## Library Usage

```rust
use i_am_surging::get_price;

#[tokio::main]
async fn main() {
    let price = get_price("btc").await.unwrap();
    println!("Bitcoin: ${:.2}", price.value);
}
```

### Multiple Prices

```rust
use i_am_surging::get_prices;

#[tokio::main]
async fn main() {
    let prices = get_prices(&["btc", "eth", "sol"]).await.unwrap();
    for p in prices {
        println!("{}: ${:.2}", p.symbol, p.value);
    }
}
```

### With Client

```rust
use i_am_surging::SurgeClient;

#[tokio::main]
async fn main() {
    let client = SurgeClient::new().unwrap();

    // Shortcuts work
    let btc = client.get_price("btc").await.unwrap();

    // Full symbols work too
    let eth = client.get_price("ETH/USDT").await.unwrap();
}
```

## Add to Your Project

```toml
[dependencies]
i-am-surging = { git = "https://github.com/you/IAmSurging" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

## Server Mode

Run as an HTTP/WebSocket API server:

```bash
# Start the server
cargo run --bin surge-server

# With authentication enabled
SURGE_API_KEY=your-secret-key cargo run --bin surge-server
```

See [API.md](API.md) for full API documentation.

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SURGE_API_KEY` | - | API key for authentication. If not set, auth is disabled |
| `SURGE_HOST` | `0.0.0.0` | Server bind address |
| `SURGE_PORT` | `9000` | Server port |
| `RUST_LOG` | `info` | Log level (trace, debug, info, warn, error) |

## Docker

```bash
# Build the image (requires feedIds.json in project root)
docker build -t surge-server .

# Run with authentication
docker run -p 9000:9000 -e SURGE_API_KEY=your-secret-key surge-server

# Run without authentication (development only)
docker run -p 9000:9000 surge-server
```

**Note:** The Docker build requires `feedIds.json` in the project root. Run `./scripts/setup.sh` to generate it automatically.

## How It Works

1. You request a price (e.g., "btc")
2. Symbol is normalized ("btc" → "BTC/USD")
3. Feed ID is looked up from `feedIds.json`
4. Crossbar API returns the oracle price
5. You get a reliable, manipulation-resistant price

## Supported Symbols

2,266 trading pairs including:
- **Major**: BTC, ETH, SOL, BNB, XRP, ADA, DOGE, AVAX, DOT, MATIC
- **Stablecoins**: USDC, USDT, DAI
- **DeFi**: UNI, AAVE, LINK, MKR, CRV, LDO
- **And 2,200+ more**

Run `surge list` to see all.

## License

MIT

---

*Unofficial project. Not affiliated with Switchboard.*
