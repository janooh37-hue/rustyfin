//! Torrent search service - Search across multiple indexers

use regex::Regex;

use crate::config::AppConfig;
use crate::models::SearchResult;

/// All available indexers with metadata
pub struct IndexerInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    /// "movie", "tv", "anime", or "all"
    pub category: &'static str,
}

/// Registry of all available indexers
pub const AVAILABLE_INDEXERS: &[IndexerInfo] = &[
    IndexerInfo { id: "yts",       name: "YTS",            description: "Movies (API)",          category: "movie" },
    IndexerInfo { id: "tpb",       name: "The Pirate Bay", description: "General (API)",          category: "all" },
    IndexerInfo { id: "1337x",     name: "1337x",          description: "General (scrape)",       category: "all" },
    IndexerInfo { id: "eztv",      name: "EZTV",           description: "TV Shows (API)",         category: "tv" },
    IndexerInfo { id: "nyaa",      name: "Nyaa",           description: "Anime (scrape)",         category: "anime" },
    IndexerInfo { id: "torrentgalaxy", name: "TorrentGalaxy", description: "General (scrape)",    category: "all" },
    IndexerInfo { id: "limetorrents", name: "LimeTorrents", description: "General (scrape)",      category: "all" },
];

/// Get all available indexer IDs
pub fn all_indexer_ids() -> Vec<&'static str> {
    AVAILABLE_INDEXERS.iter().map(|i| i.id).collect()
}

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

        for indexer_id in active_indexers {
            let r = match indexer_id.as_str() {
                "yts" if media_type == "movie" => self.search_yts(query).await,
                "yts" => continue, // YTS is movie-only
                "tpb" => {
                    let cat = if media_type == "movie" { "207" } else { "208" };
                    self.search_tpb(query, cat).await
                }
                "1337x" => self.search_1337x(query).await,
                "eztv" if media_type != "movie" => self.search_eztv(query).await,
                "eztv" => continue, // EZTV is TV-only
                "nyaa" => self.search_nyaa(query).await,
                "torrentgalaxy" => self.search_torrentgalaxy(query).await,
                "limetorrents" => self.search_limetorrents(query).await,
                _ => continue,
            };
            if let Ok(r) = r {
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

    /// Search YTS API (movies only)
    async fn search_yts(&self, query: &str) -> Result<Vec<SearchResult>, Box<dyn std::error::Error + Send + Sync>> {
        let url = "https://yts.torrentbay.st/api/v2/list_movies.json";
        let response = self.client
            .get(url)
            .query(&[("query_term", query), ("limit", "20")])
            .send()
            .await?;

        let data: serde_json::Value = response.json().await?;
        let movies = match data["data"]["movies"].as_array() {
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
                    magnet: build_magnet(torrent["hash"].as_str().unwrap_or(""), &name),
                    source: "yts".to_string(),
                });
            }
        }

        Ok(results)
    }

    /// Search The Pirate Bay API
    async fn search_tpb(&self, query: &str, category: &str) -> Result<Vec<SearchResult>, Box<dyn std::error::Error + Send + Sync>> {
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
            if name == "No results returned" { continue; }

            let info_hash = entry["info_hash"].as_str().unwrap_or("");
            if info_hash.is_empty() || info_hash == "0" { continue; }

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
    async fn search_1337x(&self, query: &str) -> Result<Vec<SearchResult>, Box<dyn std::error::Error + Send + Sync>> {
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

            if name.is_empty() { continue; }

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

            let magnet = self.get_1337x_magnet(&href).await.unwrap_or_default();
            if magnet.is_empty() { continue; }

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
    async fn get_1337x_magnet(&self, href: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let url = if href.starts_with('/') {
            format!("https://1337x.to{}", href)
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

    /// Search EZTV API (TV shows)
    async fn search_eztv(&self, query: &str) -> Result<Vec<SearchResult>, Box<dyn std::error::Error + Send + Sync>> {
        let url = "https://eztvx.to/api/get-torrents";
        let response = self.client
            .get(url)
            .query(&[("limit", "30")])
            .send()
            .await?;

        let data: serde_json::Value = response.json().await?;
        let torrents = match data["torrents"].as_array() {
            Some(arr) => arr,
            None => return Ok(Vec::new()),
        };

        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for t in torrents {
            let name = t["title"].as_str().unwrap_or("");
            if !name.to_lowercase().contains(&query_lower) { continue; }

            let hash = t["hash"].as_str().unwrap_or("");
            if hash.is_empty() { continue; }

            let seeds = t["seeds"].as_u64().unwrap_or(0) as u32;
            let size_bytes = t["size_bytes"].as_u64()
                .or_else(|| t["size_bytes"].as_str().and_then(|s| s.parse().ok()))
                .unwrap_or(0);
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
                magnet: build_magnet(hash, name),
                source: "eztv".to_string(),
            });
        }

        Ok(results)
    }

    /// Search Nyaa.si (anime torrents via RSS/scrape)
    async fn search_nyaa(&self, query: &str) -> Result<Vec<SearchResult>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "https://nyaa.si/?f=0&c=1_2&q={}&s=seeders&o=desc",
            urlencoding::encode(query)
        );
        let response = self.client.get(&url).send().await?;
        let html = response.text().await?;
        let document = scraper::Html::parse_document(&html);

        let row_selector = scraper::Selector::parse("table.torrent-list tbody tr").unwrap();
        let td_selector = scraper::Selector::parse("td").unwrap();
        let link_selector = scraper::Selector::parse("td:nth-of-type(2) a:not(.comments)").unwrap();
        let magnet_selector = scraper::Selector::parse("td:nth-of-type(3) a[href^='magnet:']").unwrap();

        let mut results = Vec::new();
        for row in document.select(&row_selector).take(20) {
            let name = row
                .select(&link_selector)
                .last()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            if name.is_empty() { continue; }

            let magnet = row
                .select(&magnet_selector)
                .next()
                .and_then(|el| el.value().attr("href"))
                .unwrap_or("")
                .to_string();

            if magnet.is_empty() { continue; }

            let tds: Vec<_> = row.select(&td_selector).collect();
            let size = tds.get(3).map(|td| td.text().collect::<String>().trim().to_string()).unwrap_or_default();
            let seeds: u32 = tds.get(5).map(|td| td.text().collect::<String>().trim().parse().unwrap_or(0)).unwrap_or(0);

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
                source: "nyaa".to_string(),
            });
        }

        Ok(results)
    }

    /// Search TorrentGalaxy (web scraping)
    async fn search_torrentgalaxy(&self, query: &str) -> Result<Vec<SearchResult>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "https://torrentgalaxy.to/torrents.php?search={}&sort=seeders&order=desc",
            urlencoding::encode(query)
        );
        let response = self.client.get(&url).send().await?;
        let html = response.text().await?;
        let document = scraper::Html::parse_document(&html);

        let row_selector = scraper::Selector::parse("div.tgxtablerow").unwrap();

        let mut results = Vec::new();
        for row in document.select(&row_selector).take(15) {
            let name_el = row.select(&scraper::Selector::parse("a.txlight").unwrap()).next();
            let name = name_el
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            if name.is_empty() { continue; }

            let magnet = row
                .select(&scraper::Selector::parse("a[href^='magnet:']").unwrap())
                .next()
                .and_then(|el| el.value().attr("href"))
                .unwrap_or("")
                .to_string();

            if magnet.is_empty() { continue; }

            // Extract seeds from the span with font color green
            let seeds: u32 = row
                .select(&scraper::Selector::parse("span[title='Seeders/Leechers'] b:first-child, font[color='green'] b").unwrap())
                .next()
                .map(|el| el.text().collect::<String>().trim().parse().unwrap_or(0))
                .unwrap_or(0);

            let size = row
                .select(&scraper::Selector::parse("span.badge-secondary").unwrap())
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

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
                source: "torrentgalaxy".to_string(),
            });
        }

        Ok(results)
    }

    /// Search LimeTorrents (web scraping)
    async fn search_limetorrents(&self, query: &str) -> Result<Vec<SearchResult>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "https://www.limetorrents.lol/search/all/{}/seeds/1/",
            urlencoding::encode(query)
        );
        let response = self.client.get(&url).send().await?;
        let html = response.text().await?;
        let document = scraper::Html::parse_document(&html);

        let row_selector = scraper::Selector::parse("table.table2 tbody tr").unwrap();

        let mut results = Vec::new();
        for row in document.select(&row_selector).take(15) {
            let name_el = row.select(&scraper::Selector::parse("td.tdleft div.tt-name a").unwrap()).next();
            let name = name_el
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            if name.is_empty() { continue; }

            let href = name_el
                .and_then(|el| el.value().attr("href"))
                .unwrap_or("")
                .to_string();

            if href.is_empty() { continue; }

            let tds: Vec<_> = row.select(&scraper::Selector::parse("td").unwrap()).collect();
            let size = tds.get(2).map(|td| td.text().collect::<String>().trim().to_string()).unwrap_or_default();
            let seeds: u32 = tds.get(3)
                .map(|td| td.text().collect::<String>().trim().replace(',', "").parse().unwrap_or(0))
                .unwrap_or(0);

            // Get magnet from detail page
            let magnet = self.get_limetorrents_magnet(&href).await.unwrap_or_default();
            if magnet.is_empty() { continue; }

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
                source: "limetorrents".to_string(),
            });
        }

        Ok(results)
    }

    /// Get magnet from LimeTorrents detail page
    async fn get_limetorrents_magnet(&self, href: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let url = if href.starts_with('/') {
            format!("https://www.limetorrents.lol{}", href)
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
        "udp://tracker.coppersurfer.tk:6969/announce",
        "udp://exodus.desync.com:6969/announce",
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
    let re = Regex::new(r"([\d,.]+)\s*(gb|mb|kb|tb|gib|mib)").unwrap();

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
