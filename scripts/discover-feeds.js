#!/usr/bin/env node

/**
 * Discover and save Switchboard Surge feed IDs
 *
 * Usage:
 *   node scripts/discover-feeds.js              # Interactive mode
 *   node scripts/discover-feeds.js --all        # Save all feeds
 *   node scripts/discover-feeds.js --list       # List all feeds
 *   node scripts/discover-feeds.js --search BTC # Search feeds
 *
 * Environment Variables:
 *   FEEDS_API_URL - Custom Switchboard feeds API URL
 *                   Default: https://explorer.switchboardlabs.xyz/api/feeds
 */

const fs = require('fs');
const path = require('path');
const readline = require('readline');

// Configuration
const API_URL = process.env.FEEDS_API_URL || 'https://explorer.switchboardlabs.xyz/api/feeds';

// Output file is always in project root
const SCRIPT_DIR = __dirname;
const PROJECT_ROOT = path.dirname(SCRIPT_DIR);
const OUTPUT_FILE = path.join(PROJECT_ROOT, 'feedIds.json');

async function fetchAllFeeds() {
    let allFeeds = [];
    let page = 1;
    let totalPages = 1;

    console.log(`Fetching feeds from: ${API_URL}`);

    while (page <= totalPages) {
        const response = await fetch(`${API_URL}?page=${page}&limit=100`);

        if (!response.ok) {
            throw new Error(`API request failed: ${response.status} ${response.statusText}`);
        }

        const data = await response.json();

        allFeeds = allFeeds.concat(data.feeds);
        totalPages = data.pagination.totalPages;

        process.stdout.write(`\rFetched page ${page}/${totalPages} (${allFeeds.length} feeds)`);
        page++;
    }

    console.log('\n');
    return allFeeds;
}

function formatFeedId(feed) {
    const symbol = `${feed.base}/${feed.quote}`;
    return {
        symbol,
        feedId: feed.mainnetFeedHash,
        name: feed.name,
        rank: feed.rank
    };
}

function saveFeeds(feeds, filename) {
    const feedMap = {};
    feeds.forEach(feed => {
        const formatted = formatFeedId(feed);
        if (formatted.feedId) {  // Only save feeds with valid IDs
            feedMap[formatted.symbol] = formatted.feedId;
        }
    });

    fs.writeFileSync(filename, JSON.stringify(feedMap, null, 2));
    console.log(`Saved ${Object.keys(feedMap).length} feeds to ${filename}`);
}

function loadExistingFeeds() {
    try {
        if (fs.existsSync(OUTPUT_FILE)) {
            return JSON.parse(fs.readFileSync(OUTPUT_FILE, 'utf8'));
        }
    } catch (e) {
        // File doesn't exist or invalid
    }
    return {};
}

async function interactiveMode(feeds) {
    const rl = readline.createInterface({
        input: process.stdin,
        output: process.stdout
    });

    const question = (prompt) => new Promise(resolve => rl.question(prompt, resolve));

    console.log(`Found ${feeds.length} available feeds.\n`);
    console.log('Options:');
    console.log('  1. Save ALL feeds to feedIds.json');
    console.log('  2. Search and select specific feeds');
    console.log('  3. List top 50 feeds by rank');
    console.log('  4. Exit\n');

    const choice = await question('Enter choice (1-4): ');

    if (choice === '1') {
        saveFeeds(feeds, OUTPUT_FILE);
    } else if (choice === '2') {
        const existing = loadExistingFeeds();
        let selectedFeeds = { ...existing };

        console.log('\nEnter search terms to find feeds (or "done" to finish):');

        while (true) {
            const search = await question('\nSearch: ');

            if (search.toLowerCase() === 'done') {
                break;
            }

            const matches = feeds.filter(f =>
                f.base.toLowerCase().includes(search.toLowerCase()) ||
                f.quote.toLowerCase().includes(search.toLowerCase()) ||
                f.name.toLowerCase().includes(search.toLowerCase())
            );

            if (matches.length === 0) {
                console.log('No feeds found matching that search.');
                continue;
            }

            console.log(`\nFound ${matches.length} matches:`);
            matches.slice(0, 20).forEach((feed, i) => {
                const symbol = `${feed.base}/${feed.quote}`;
                const exists = selectedFeeds[symbol] ? ' [already added]' : '';
                console.log(`  ${i + 1}. ${symbol}${exists}`);
            });

            if (matches.length > 20) {
                console.log(`  ... and ${matches.length - 20} more`);
            }

            const selection = await question('\nEnter numbers to add (e.g., "1,2,3" or "all" or "none"): ');

            if (selection.toLowerCase() === 'all') {
                matches.forEach(feed => {
                    const formatted = formatFeedId(feed);
                    if (formatted.feedId) {
                        selectedFeeds[formatted.symbol] = formatted.feedId;
                    }
                });
                console.log(`Added ${matches.length} feeds.`);
            } else if (selection.toLowerCase() !== 'none') {
                const nums = selection.split(',').map(n => parseInt(n.trim()) - 1);
                nums.forEach(i => {
                    if (i >= 0 && i < matches.length) {
                        const formatted = formatFeedId(matches[i]);
                        if (formatted.feedId) {
                            selectedFeeds[formatted.symbol] = formatted.feedId;
                            console.log(`Added ${formatted.symbol}`);
                        }
                    }
                });
            }
        }

        if (Object.keys(selectedFeeds).length > 0) {
            fs.writeFileSync(OUTPUT_FILE, JSON.stringify(selectedFeeds, null, 2));
            console.log(`\nSaved ${Object.keys(selectedFeeds).length} feeds to ${OUTPUT_FILE}`);
        }
    } else if (choice === '3') {
        console.log('\nTop 50 feeds by rank:\n');
        const sorted = feeds.sort((a, b) => (a.rank || 999) - (b.rank || 999));
        sorted.slice(0, 50).forEach((feed, i) => {
            console.log(`  ${i + 1}. ${feed.base}/${feed.quote}`);
        });
    }

    rl.close();
}

async function main() {
    const args = process.argv.slice(2);

    try {
        const feeds = await fetchAllFeeds();

        if (args.includes('--all')) {
            saveFeeds(feeds, OUTPUT_FILE);
        } else if (args.includes('--list')) {
            console.log('Available feeds:\n');
            feeds.forEach(feed => {
                console.log(`  ${feed.base}/${feed.quote}`);
            });
            console.log(`\nTotal: ${feeds.length} feeds`);
        } else if (args.includes('--search')) {
            const searchIndex = args.indexOf('--search');
            const searchTerm = args[searchIndex + 1] || '';

            const matches = feeds.filter(f =>
                f.base.toLowerCase().includes(searchTerm.toLowerCase()) ||
                f.quote.toLowerCase().includes(searchTerm.toLowerCase())
            );

            console.log(`Feeds matching "${searchTerm}":\n`);
            matches.forEach(feed => {
                console.log(`  ${feed.base}/${feed.quote}: ${feed.mainnetFeedHash}`);
            });
            console.log(`\nTotal: ${matches.length} feeds`);
        } else {
            await interactiveMode(feeds);
        }
    } catch (error) {
        console.error('Error:', error.message);
        process.exit(1);
    }
}

main();
