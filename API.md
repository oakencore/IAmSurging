# Surge API Documentation

REST and WebSocket API for real-time cryptocurrency price data from Switchboard Surge.

## Base URL

```
http://localhost:9000
```

## Authentication

All `/v1/*` endpoints require API key authentication via the `Authorization` header:

```
Authorization: Bearer <SURGE_API_KEY>
```

Public endpoints (`/health`, `/ready`, `/metrics`) do not require authentication.

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `SURGE_API_KEY` | No | - | API key for authentication. If not set, auth is disabled |
| `SURGE_PORT` | No | 9000 | Server port |
| `SURGE_HOST` | No | 0.0.0.0 | Server host |
| `RUST_LOG` | No | info | Log level filter |

---

## REST Endpoints

### Health Check

Liveness probe - always returns 200 if the server is running.

```
GET /health
```

**Response:**
```json
{
  "status": "healthy"
}
```

---

### Readiness Check

Readiness probe - returns 200 if feed data is loaded and the server can serve requests.

```
GET /ready
```

**Response (200):**
```json
{
  "status": "ready"
}
```

**Response (503):**
```json
{
  "status": "not ready"
}
```

---

### Prometheus Metrics

Prometheus-formatted metrics for monitoring.

```
GET /metrics
```

**Response:**
```
# HELP http_requests_total Total number of HTTP requests
# TYPE http_requests_total counter
http_requests_total{method="GET",path="/v1/prices/btc",status="200"} 42

# HELP http_request_duration_seconds HTTP request duration in seconds
# TYPE http_request_duration_seconds histogram
http_request_duration_seconds_bucket{method="GET",path="/v1/prices/btc",status="200",le="0.005"} 40

# HELP active_websocket_connections Number of active WebSocket connections
# TYPE active_websocket_connections gauge
active_websocket_connections 5
```

---

### Get Single Price

Get the current price for a single symbol.

```
GET /v1/prices/:symbol
```

**Path Parameters:**
- `symbol` - Price pair symbol (e.g., `btc`, `BTC/USD`, `eth/usdt`)

**Example:**
```bash
curl -H "Authorization: Bearer $API_KEY" http://localhost:9000/v1/prices/btc
```

**Response (200):**
```json
{
  "success": true,
  "data": {
    "symbol": "BTC/USD",
    "feed_id": "abc123...",
    "price": 89846.94
  }
}
```

**Response (404):**
```json
{
  "success": false,
  "error": "Feed not found: INVALID/SYMBOL"
}
```

---

### Get Multiple Prices

Get prices for multiple symbols in a single request.

```
GET /v1/prices?symbols=<comma-separated-symbols>
```

**Query Parameters:**
- `symbols` - Comma-separated list of symbols

**Example:**
```bash
curl -H "Authorization: Bearer $API_KEY" "http://localhost:9000/v1/prices?symbols=btc,eth,sol"
```

**Response (200):**
```json
{
  "success": true,
  "data": [
    {
      "symbol": "BTC/USD",
      "feed_id": "abc123...",
      "price": 89846.94
    },
    {
      "symbol": "ETH/USD",
      "feed_id": "def456...",
      "price": 3245.50
    },
    {
      "symbol": "SOL/USD",
      "feed_id": "ghi789...",
      "price": 148.25
    }
  ]
}
```

---

### List Available Symbols

Get a list of all available price feed symbols.

```
GET /v1/symbols
GET /v1/symbols?filter=<substring>
```

**Query Parameters:**
- `filter` (optional) - Filter symbols by substring match

**Example:**
```bash
# List all symbols
curl -H "Authorization: Bearer $API_KEY" http://localhost:9000/v1/symbols

# Filter by "sol"
curl -H "Authorization: Bearer $API_KEY" "http://localhost:9000/v1/symbols?filter=sol"
```

**Response (200):**
```json
{
  "success": true,
  "data": {
    "symbols": ["SOL/USD", "SOL/USDT", "WSOL/USD"],
    "count": 3
  }
}
```

---

## WebSocket API

### Connect

```
WS /v1/stream
```

**Headers:**
```
Authorization: Bearer <SURGE_API_KEY>
```

**Example (using websocat):**
```bash
websocat -H "Authorization: Bearer $API_KEY" ws://localhost:9000/v1/stream
```

---

### Subscribe to Symbols

**Client → Server:**
```json
{
  "action": "subscribe",
  "symbols": ["BTC/USD", "ETH/USD"]
}
```

**Server → Client (confirmation):**
```json
{
  "type": "subscribed",
  "symbols": ["BTC/USD", "ETH/USD"]
}
```

---

### Unsubscribe from Symbols

**Client → Server:**
```json
{
  "action": "unsubscribe",
  "symbols": ["ETH/USD"]
}
```

**Server → Client (confirmation):**
```json
{
  "type": "unsubscribed",
  "symbols": ["ETH/USD"]
}
```

---

### Price Updates

Real-time price updates are pushed to the client:

**Server → Client:**
```json
{
  "type": "price",
  "symbol": "BTC/USD",
  "price": 89846.94,
  "timestamp": 1705936800000,
  "feed_id": "abc123..."
}
```

---

### Error Messages

**Server → Client:**
```json
{
  "type": "error",
  "message": "Invalid message format"
}
```

---

## Error Responses

All error responses follow this format:

```json
{
  "success": false,
  "error": "Error message here"
}
```

**HTTP Status Codes:**

| Code | Description |
|------|-------------|
| 200 | Success |
| 400 | Bad Request - Invalid parameters |
| 401 | Unauthorized - Missing or invalid API key |
| 404 | Not Found - Symbol not found |
| 502 | Bad Gateway - Upstream API error |
| 503 | Service Unavailable - Server not ready |

---

## Examples

### cURL

```bash
# Set API key
export API_KEY="your-api-key"

# Get BTC price
curl -H "Authorization: Bearer $API_KEY" http://localhost:9000/v1/prices/btc

# Get multiple prices
curl -H "Authorization: Bearer $API_KEY" "http://localhost:9000/v1/prices?symbols=btc,eth,sol"

# List all SOL-related symbols
curl -H "Authorization: Bearer $API_KEY" "http://localhost:9000/v1/symbols?filter=sol"
```

### Python

```python
import requests
import websocket
import json

API_KEY = "your-api-key"
BASE_URL = "http://localhost:9000"

# REST API
headers = {"Authorization": f"Bearer {API_KEY}"}

# Get single price
response = requests.get(f"{BASE_URL}/v1/prices/btc", headers=headers)
print(response.json())

# WebSocket streaming
ws = websocket.WebSocket()
ws.connect(
    "ws://localhost:9000/v1/stream",
    header=[f"Authorization: Bearer {API_KEY}"]
)

# Subscribe
ws.send(json.dumps({"action": "subscribe", "symbols": ["BTC/USD", "ETH/USD"]}))

# Receive updates
while True:
    message = json.loads(ws.recv())
    print(f"{message['symbol']}: ${message['price']}")
```

### JavaScript

```javascript
const API_KEY = 'your-api-key';
const BASE_URL = 'http://localhost:9000';

// REST API
const response = await fetch(`${BASE_URL}/v1/prices/btc`, {
  headers: { 'Authorization': `Bearer ${API_KEY}` }
});
const data = await response.json();
console.log(data);

// WebSocket
const ws = new WebSocket('ws://localhost:9000/v1/stream', [], {
  headers: { 'Authorization': `Bearer ${API_KEY}` }
});

ws.onopen = () => {
  ws.send(JSON.stringify({
    action: 'subscribe',
    symbols: ['BTC/USD', 'ETH/USD']
  }));
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  if (data.type === 'price') {
    console.log(`${data.symbol}: $${data.price}`);
  }
};
```

---

## Running the Server

### Local Development

```bash
# Set API key and start server
SURGE_API_KEY=your-secret-key cargo run --bin surge-server
```

### Docker

```bash
# Build image
docker build -t surge-server .

# Run container
docker run -d \
  -e SURGE_API_KEY=your-secret-key \
  -p 9000:9000 \
  --name surge-server \
  surge-server
```

### Docker Compose

```yaml
version: '3.8'
services:
  surge-server:
    build: .
    environment:
      - SURGE_API_KEY=${SURGE_API_KEY}
      - RUST_LOG=info
    ports:
      - "9000:9000"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/health"]
      interval: 30s
      timeout: 3s
      retries: 3
```
