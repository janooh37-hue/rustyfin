//! Torrent search service - Search across YTS, TPB, and 1337x

use regex::Regex;

use crate::config::AppConfig;
use crate::models::SearchResult;

/// Torrent search service
pub struct TorrentSearchService {
    config: AppConfig,
    client: reqwest::Client,
}

impl TorrentSearchService {
    /// Create a new search service
    pub fn new(config: AppConfig) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self { config, client }
    }

    /// Search for torrents using the given indexer list
    pub async fn search(&self, query: &str, media_type: &str, active_indexers: &[String]) -> Vec<SearchResult> {
        let mut results = Vec::new();
        let indexers = active_indexers;

        // Search YTS for movies (only if enabled)
        if media_type == "movie" && indexers.iter().any(|i| i == "yts") {
            if let Ok(r) = self.search_yts(query).await {
                results.extend(r);
            }
        }

        // Search TPB (only if enabled)
        if indexers.iter().any(|i| i == "tpb") {
            let tpb_cat = if media_type == "movie" { "207" } else { "208" };
            if let Ok(r) = self.search_tpb(query, tpb_cat).await {
                results.extend(r);
            }
        }

        // Search 1337x (only if enabled)
        if indexers.iter().any(|i| i == "1337x") {
            if let Ok(r) = self.search_1337x(query).await {
                results.extend(r);
            }
        }

        // Filter CAM if configured
        if self.config.settings.avoid_cam {
            results.retain(|r| !is_cam(&r.name));
        }

        // Sort by seeds (highest first)
        results.sort_by(|a, b| b.seeds.cmp(&a.seeds));
        results
    }

    /// Search YTS API
    async fn search_yts(&self, query: &str) -> Result<Vec<SearchResult>, reqwest::Error> {
        let url = "https://yts.torrentbay.st/api/v2/list_movies.json";
        let response = self.client
            .get(url)
            .query(&[("query_term", query), ("limit", "20")])
            .send()
            .await?;

        let data: serde_json::Value = response.json().await?;
        let movies_array = data["data"]["movies"].as_array();
        let movies = match movies_array {
            Some(arr) => arr,
            None => return Ok(Vec::new()),
        };

        let mut results = Vec::new();
        for movie in movies {
            let title = movie["title"].as_str().unwrap_or("");
            let year = movie["year"].as_i64().map(|y| y as i32);
            
            for torrent in movie["torrents"].as_array().unwrap_or(&Vec::new()) {
                let name = format!(
                    "{} ({}) [{}]",
                    title,
                    year.map(|y| y.to_string()).unwrap_or_default(),
                    torrent["quality"].as_str().unwrap_or("unknown")
                );

                let size_bytes = torrent["size_bytes"].as_u64().unwrap_or(0);
                let size_gb = size_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

                results.push(SearchResult {
                    name: name.clone(),
                    title: title.to_string(),
                    year,
                    quality: torrent["quality"].as_str().unwrap_or("unknown").to_string(),
                    seeds: torrent["seeds"].as_u64().unwrap_or(0) as u32,
                    size: torrent["size"].as_str().unwrap_or("").to_string(),
                    size_gb: (size_gb * 100.0).round() / 100.0,
                    magnet: build_magnet(
                        torrent["hash"].as_str().unwrap_or(""),
                        &name,
                    ),
                    source: "yts".to_string(),
                });
            }
        }

        Ok(results)
    }

    /// Search The Pirate Bay API
    async fn search_tpb(&self, query: &str, category: &str) -> Result<Vec<SearchResult>, reqwest::Error> {
        let url = "https://apibay.org/q.php";
        let response = self.client
            .get(url)
            .query(&[("q", query), ("cat", category)])
            .send()
            .await?;

        let entries: Vec<serde_json::Value> = response.json().await?;

        let mut results = Vec::new();
        for entry in entries {
            let name = entry["name"].as_str().unwrap_or("");
            if name == "No results returned" {
                continue;
            }

            let info_hash = entry["info_hash"].as_str().unwrap_or("");
            if info_hash.is_empty() || info_hash == "0" {
                continue;
            }

            let seeds = entry["seeders"].as_str().unwrap_or("0").parse().unwrap_or(0);
            let size_bytes: u64 = entry["size"].as_str().unwrap_or("0").parse().unwrap_or(0);
            let size_gb = size_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

            let (title, year) = extract_title_year(name);

            results.push(SearchResult {
                name: name.to_string(),
                title,
                year,
                quality: detect_quality(name),
                seeds,
                size: format!("{:.1} GB", size_gb),
                size_gb: (size_gb * 100.0).round() / 100.0,
                magnet: build_magnet(info_hash, name),
                source: "tpb".to_string(),
            });
        }

        Ok(results)
    }

    /// Search 1337x (web scraping)
    async fn search_1337x(&self, query: &str) -> Result<Vec<SearchResult>, reqwest::Error> {
        let url = format!("https://1337x.to/search/{}/1/", urlencoding::encode(query));
        let response = self.client.get(&url).send().await?;

        let html = response.text().await?;
        let document = scraper::Html::parse_document(&html);

        let row_selector = scraper::Selector::parse("table.table-list tbody tr").unwrap();
        let rows = document.select(&row_selector);

        let mut results = Vec::new();
        for row in rows.take(15) {
            let name = row
                .select(&scraper::Selector::parse("td.name a:nth-of-type(2)").unwrap())
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            if name.is_empty() {
                continue;
            }

            let seeds: u32 = row
                .select(&scraper::Selector::parse("td.seeds").unwrap())
                .next()
                .map(|el| el.text().collect::<String>().parse().unwrap_or(0))
                .unwrap_or(0);

            let size = row
                .select(&scraper::Selector::parse("td.size").unwrap())
                .next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();

            let href = row
                .select(&scraper::Selector::parse("td.name a:nth-of-type(2)").unwrap())
                .next()
                .and_then(|el| el.value().attr("href"))
                .unwrap_or("")
                .to_string();

            // Get magnet from detail page
            let magnet = self.get_1337x_magnet(&href).await.unwrap_or_default();
            if magnet.is_empty() {
                continue;
            }

            let (title, year) = extract_title_year(&name);
            let size_gb = parse_size_gb(&size);

            results.push(SearchResult {
                name: name.clone(),
                title,
                year,
                quality: detect_quality(&name),
                seeds,
                size,
                size_gb,
                magnet,
                source: "1337x".to_string(),
            });
        }

        Ok(results)
    }

    /// Get magnet link from 1337x detail page
    async fn get_1337x_magnet(&self, href: &str) -> Result<String, reqwest::Error> {
        let base = "https://1337x.to";
        let url = if href.starts_with('/') {
            format!("{}{}", base, href)
        } else {
            href.to_string()
        };

        let response = self.client.get(&url).send().await?;
        let html = response.text().await?;
        let document = scraper::Html::parse_document(&html);

        let magnet_selector = scraper::Selector::parse("a[href^='magnet:']").unwrap();
        Ok(document
            .select(&magnet_selector)
            .next()
            .and_then(|el| el.value().attr("href"))
            .map(|s| s.to_string())
            .unwrap_or_default())
    }
}

/// Build magnet link with trackers
fn build_magnet(info_hash: &str, name: &str) -> String {
    let dn = urlencoding::encode(name);
    let trackers = [
        "udp://tracker.opentrackr.org:1337/announce",
        "udp://open.stealth.si:80/announce",
        "udp://tracker.torrent.eu.org:451/announce",
    ];
    let tr_params: String = trackers
        .iter()
        .map(|t| format!("&tr={}", urlencoding::encode(t)))
        .collect();

    format!(
        "magnet:?xt=urn:btih:{}&dn={}{}",
        info_hash, dn, tr_params
    )
}

/// Detect quality from name
fn detect_quality(name: &str) -> String {
    let name_lower = name.to_lowercase();
    if name_lower.contains("2160p") || name_lower.contains("4k") {
        "2160p".to_string()
    } else if name_lower.contains("1080p") {
        "1080p".to_string()
    } else if name_lower.contains("720p") {
        "720p".to_string()
    } else if name_lower.contains("480p") {
        "480p".to_string()
    } else {
        "unknown".to_string()
    }
}

/// Check if release is CAM
fn is_cam(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    name_lower.contains("cam")
        || name_lower.contains("hdcam")
        || name_lower.contains("ts")
        || name_lower.contains("telesync")
}

/// Extract title and year from torrent name
fn extract_title_year(name: &str) -> (String, Option<i32>) {
    let year_re = Regex::new(r"[\(\[\s]?((?:19|20)\d{2})[\)\]\s]?").unwrap();
    
    if let Some(m) = year_re.find(name) {
        let year: i32 = m.as_str().trim_matches(|c| c == '(' || c == ')' || c == '[' || c == ']').parse().unwrap_or(0);
        let title = name[..m.start()].trim().to_string();
        (title, Some(year))
    } else {
        (name.to_string(), None)
    }
}

/// Parse size string to GB
fn parse_size_gb(size: &str) -> f64 {
    let size_lower = size.to_lowercase();
    let re = Regex::new(r"([\d,.]+)\s*(gb|mb|kb|tb)").unwrap();
    
    if let Some(caps) = re.captures(&size_lower) {
        let value: f64 = caps.get(1).map(|m| m.as_str().replace(',', "").parse().unwrap_or(0.0)).unwrap_or(0.0);
        let unit = caps.get(2).map(|m| m.as_str()).unwrap_or("gb");
        
        match unit {
            "tb" => value * 1024.0,
            "gb" | "gib" => value,
            "mb" | "mib" => value / 1024.0,
            "kb" | "kib" => value / (1024.0 * 1024.0),
            _ => value,
        }
    } else {
        0.0
    }
}