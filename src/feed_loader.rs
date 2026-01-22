use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::error::{Result, SurgeError};

/// Loads feed IDs from feedIds.json file
pub struct FeedLoader {
    feeds: HashMap<String, String>,
}

impl FeedLoader {
    /// Load feeds from the default feedIds.json path
    pub fn load_default() -> Result<Self> {
        let paths = ["feedIds.json", "../feedIds.json"];
        for path in paths {
            if Path::new(path).exists() {
                return Self::load_from_path(path);
            }
        }
        Err(SurgeError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "feedIds.json not found",
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

    /// Get all available symbols
    pub fn get_all_symbols(&self) -> Vec<String> {
        let mut symbols: Vec<String> = self.feeds.keys().cloned().collect();
        symbols.sort();
        symbols
    }

    /// Check if a symbol exists
    pub fn has_symbol(&self, symbol: &str) -> bool {
        self.feeds.contains_key(symbol)
    }

    /// Get the total number of feeds
    pub fn len(&self) -> usize {
        self.feeds.len()
    }

    /// Check if the loader is empty
    pub fn is_empty(&self) -> bool {
        self.feeds.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Loading tests ===

    #[test]
    fn test_load_default() {
        let loader = FeedLoader::load_default().expect("should load feedIds.json");
        assert!(!loader.is_empty(), "should have feeds");
        assert!(loader.len() > 2000, "should have 2000+ feeds");
    }

    #[test]
    fn test_load_from_invalid_path() {
        let result = FeedLoader::load_from_path("/nonexistent/path.json");
        assert!(result.is_err(), "should fail for invalid path");
    }

    // === Symbol lookup tests ===

    #[test]
    fn test_has_common_symbols() {
        let loader = FeedLoader::load_default().unwrap();
        assert!(loader.has_symbol("BTC/USD"), "should have BTC/USD");
        assert!(loader.has_symbol("ETH/USD"), "should have ETH/USD");
        assert!(loader.has_symbol("SOL/USD"), "should have SOL/USD");
    }

    #[test]
    fn test_has_symbol_false_for_invalid() {
        let loader = FeedLoader::load_default().unwrap();
        assert!(!loader.has_symbol("INVALID/SYMBOL"));
        assert!(!loader.has_symbol(""));
        assert!(!loader.has_symbol("btc")); // lowercase without /USD
    }

    #[test]
    fn test_get_feed_id_valid() {
        let loader = FeedLoader::load_default().unwrap();
        let feed_id = loader.get_feed_id("BTC/USD").unwrap();
        assert!(!feed_id.is_empty(), "feed_id should not be empty");
        assert_eq!(feed_id.len(), 64, "feed_id should be 64 hex chars");
    }

    #[test]
    fn test_get_feed_id_is_hex() {
        let loader = FeedLoader::load_default().unwrap();
        let feed_id = loader.get_feed_id("BTC/USD").unwrap();
        assert!(
            feed_id.chars().all(|c| c.is_ascii_hexdigit()),
            "feed_id should be hex string"
        );
    }

    #[test]
    fn test_get_feed_id_invalid_returns_error() {
        let loader = FeedLoader::load_default().unwrap();
        let result = loader.get_feed_id("INVALID/SYMBOL");
        assert!(result.is_err(), "should return error for unknown symbol");

        if let Err(SurgeError::FeedNotFound(symbol)) = result {
            assert_eq!(symbol, "INVALID/SYMBOL");
        } else {
            panic!("should be FeedNotFound error");
        }
    }

    // === get_all_symbols tests ===

    #[test]
    fn test_get_all_symbols_not_empty() {
        let loader = FeedLoader::load_default().unwrap();
        let symbols = loader.get_all_symbols();
        assert!(!symbols.is_empty());
        assert!(symbols.len() > 2000);
    }

    #[test]
    fn test_get_all_symbols_sorted() {
        let loader = FeedLoader::load_default().unwrap();
        let symbols = loader.get_all_symbols();
        let mut sorted = symbols.clone();
        sorted.sort();
        assert_eq!(symbols, sorted, "symbols should be sorted");
    }

    #[test]
    fn test_get_all_symbols_contains_major() {
        let loader = FeedLoader::load_default().unwrap();
        let symbols = loader.get_all_symbols();
        assert!(symbols.contains(&"BTC/USD".to_string()));
        assert!(symbols.contains(&"ETH/USD".to_string()));
        assert!(symbols.contains(&"SOL/USD".to_string()));
    }

    // === len/is_empty tests ===

    #[test]
    fn test_len() {
        let loader = FeedLoader::load_default().unwrap();
        assert!(loader.len() > 2000);
        assert_eq!(loader.len(), loader.get_all_symbols().len());
    }

    #[test]
    fn test_is_empty() {
        let loader = FeedLoader::load_default().unwrap();
        assert!(!loader.is_empty());
    }
}
