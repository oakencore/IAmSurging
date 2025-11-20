#!/usr/bin/env node

/**
 * Helper script to fetch a single price using the Switchboard SDK
 * This bridges the gap until full protobuf support is added to Rust
 *
 * Usage: node fetch-price.js <feedId>
 * Example: node fetch-price.js 4cd1cad962425681af07b9254b7d804de3ca3446fbfd1371bb258d2c75059812
 *
 * Environment variables:
 * - ANCHOR_PROVIDER_URL: Solana RPC URL (optional, defaults to mainnet)
 * - ANCHOR_WALLET: Path to wallet JSON (optional, not needed for read-only)
 */

const { CrossbarClient } = require("@switchboard-xyz/common");

async function main() {
  const feedId = process.argv[2];

  if (!feedId) {
    console.error("Error: Feed ID required");
    console.error("Usage: node fetch-price.js <feedId>");
    process.exit(1);
  }

  try {
    // Create crossbar client with default Switchboard crossbar URL
    // The CrossbarClient doesn't need Solana RPC or wallet for read-only operations
    const crossbar = new CrossbarClient("http://crossbar.switchboard.xyz");

    const response = await crossbar.simulateFeed(feedId);

    if (response && response.results && response.results.length > 0) {
      const price = parseFloat(String(response.results[0]));
      console.log(price);
    } else {
      console.error("Error: No data returned");
      process.exit(1);
    }
  } catch (error) {
    console.error(`Error: ${error.message}`);
    process.exit(1);
  }
}

main();
