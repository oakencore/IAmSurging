use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::error::{Result, SurgeError};
use crate::types::{Feed, Symbol};

/// Loads feed IDs from feedIds.json file
pub struct FeedLoader {
    feeds: HashMap<String, String>,
}

impl FeedLoader {
    /// Load feeds from the default feedIds.json path
    pub fn load_default() -> Result<Self> {
        // Look for feedIds.json in the current directory or parent
        let paths = vec![
            "feedIds.json",
            "../feedIds.json",
            "./IAmSurging/feedIds.json",
        ];

        for path in paths {
            if Path::new(path).exists() {
                return Self::load_from_path(path);
            }
        }

        Err(SurgeError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "feedIds.json not found. Please ensure feedIds.json is in the current directory.",
        )))
    }

    /// Load feeds from a specific path
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let contents = fs::read_to_string(path)?;
        let feeds: HashMap<String, String> = serde_json::from_str(&contents)?;

        Ok(Self { feeds })
    }

    /// Get feed ID for a symbol
    pub fn get_feed_id(&self, symbol: &str) -> Result<&str> {
        self.feeds
            .get(symbol)
            .map(|s| s.as_str())
            .ok_or_else(|| SurgeError::FeedNotFound(symbol.to_string()))
    }

    /// Get feed for a symbol
    pub fn get_feed(&self, symbol: &str) -> Result<Feed> {
        let feed_id = self.get_feed_id(symbol)?.to_string();
        let symbol = Symbol::from_str(symbol)?;
        Ok(Feed::new(symbol, feed_id))
    }

    /// Get all available symbols
    pub fn get_all_symbols(&self) -> Vec<String> {
        let mut symbols: Vec<String> = self.feeds.keys().cloned().collect();
        symbols.sort();
        symbols
    }

    /// Get total number of feeds
    pub fn count(&self) -> usize {
        self.feeds.len()
    }

    /// Check if a symbol exists
    pub fn has_symbol(&self, symbol: &str) -> bool {
        self.feeds.contains_key(symbol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_feeds() {
        // This test will only work if feedIds.json exists
        if let Ok(loader) = FeedLoader::load_default() {
            assert!(loader.count() > 0);

            // Test common symbols
            assert!(loader.has_symbol("BTC/USD"));
            assert!(loader.has_symbol("ETH/USD"));
            assert!(loader.has_symbol("SOL/USD"));
        }
    }
}
