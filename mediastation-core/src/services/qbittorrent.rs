use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;

use crate::config::QBittorrentConfig;
use crate::models::{torrent_state, Torrent, TransferInfo};

pub struct QBittorrentService {
    client: reqwest::Client,
    config: QBittorrentConfig,
    base_url: String,
    logged_in: bool,
    sid: Option<String>,
}

impl QBittorrentService {
    pub fn new(config: &QBittorrentConfig) -> Self {
        let client = reqwest::Client::builder()
            .build()
            .expect("Failed to create HTTP client");

        let base_url = format!("{}/api/v2/", config.host.trim_end_matches('/'));

        Self {
            client,
            config: config.clone(),
            base_url,
            logged_in: false,
            sid: None,
        }
    }

    pub async fn login(&mut self) -> bool {
        let url = format!("{}auth/login", self.base_url);

        let params = [
            ("username", self.config.username.as_str()),
            ("password", self.config.password.as_str()),
        ];

        match self.client.post(&url).form(&params).send().await {
            Ok(response) => {
                let status = response.status();
                if status.is_success() || status.as_u16() == 200 {
                    if let Some(set_cookie) = response.headers().get("set-cookie") {
                        if let Ok(cookie_str) = set_cookie.to_str() {
                            for part in cookie_str.split(';') {
                                let part = part.trim();
                                if part.starts_with("SID=") {
                                    self.sid = Some(part.to_string());
                                    break;
                                }
                            }
                        }
                    }
                    self.logged_in = true;
                    log::info!("Successfully logged into qBittorrent");
                    true
                } else {
                    log::warn!("Failed to login to qBittorrent: {}", status);
                    false
                }
            }
            Err(e) => {
                log::warn!("Failed to connect to qBittorrent: {}", e);
                false
            }
        }
    }

    async fn ensure_logged_in(&mut self) -> bool {
        if !self.logged_in {
            self.login().await
        } else {
            true
        }
    }

    fn auth_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(ref sid) = self.sid {
            if let Ok(value) = HeaderValue::from_str(sid) {
                headers.insert("Cookie", value);
            }
        }
        headers
    }

    pub async fn get_torrents(&mut self, category: Option<&str>) -> Vec<Torrent> {
        if !self.ensure_logged_in().await {
            return Vec::new();
        }

        let mut url = format!("{}torrents/info", self.base_url);
        if let Some(cat) = category {
            url.push_str(&format!("?category={}", cat));
        }

        match self.client.get(&url).headers(self.auth_headers()).send().await {
            Ok(response) => {
                match response.json::<Vec<ApiTorrent>>().await {
                    Ok(torrents) => torrents.into_iter().map(|t| t.into()).collect(),
                    Err(e) => {
                        log::warn!("Failed to parse torrent list: {}", e);
                        Vec::new()
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to get torrents: {}", e);
                Vec::new()
            }
        }
    }

    pub async fn get_active_downloads(&mut self) -> Vec<Torrent> {
        let torrents = self.get_torrents(None).await;
        torrents
            .into_iter()
            .filter(|t| torrent_state::is_downloading(&t.state))
            .collect()
    }

    pub async fn get_completed(&mut self, category: Option<&str>) -> Vec<Torrent> {
        let torrents = self.get_torrents(category).await;
        torrents
            .into_iter()
            .filter(|t| torrent_state::is_seeding(&t.state))
            .collect()
    }

    pub async fn get_stalled(&mut self, threshold_hours: u32) -> Vec<Torrent> {
        let torrents = self.get_torrents(None).await;
        let now = chrono::Utc::now().timestamp();
        let threshold_secs = threshold_hours as i64 * 3600;

        torrents
            .into_iter()
            .filter(|t| {
                let active_time = t.added_on;
                if active_time <= 0 {
                    return false;
                }
                let inactive_secs = now - active_time;
                inactive_secs > threshold_secs
                    && (torrent_state::is_downloading(&t.state) || torrent_state::is_seeding(&t.state))
                    && t.download_speed == 0
                    && t.upload_speed == 0
            })
            .collect()
    }

    pub async fn add_torrent(&mut self, magnet: &str, category: &str) -> bool {
        if !self.ensure_logged_in().await {
            return false;
        }

        let url = format!("{}torrents/add", self.base_url);

        let params = [
            ("urls", magnet),
            ("category", category),
        ];

        match self.client.post(&url).headers(self.auth_headers()).form(&params).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    log::info!("Successfully added torrent: {}", magnet);
                    true
                } else {
                    let status = response.status();
                    log::warn!("Failed to add torrent: {}", status);
                    false
                }
            }
            Err(e) => {
                log::warn!("Failed to add torrent: {}", e);
                false
            }
        }
    }

    pub async fn delete_torrent(&mut self, hash: &str, delete_files: bool) -> bool {
        if !self.ensure_logged_in().await {
            return false;
        }

        let url = format!("{}torrents/delete", self.base_url);

        let params = [
            ("hashes", hash),
            ("deleteFiles", if delete_files { "true" } else { "false" }),
        ];

        match self.client.post(&url).headers(self.auth_headers()).form(&params).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    log::info!("Successfully deleted torrent: {}", hash);
                    true
                } else {
                    log::warn!("Failed to delete torrent: {}", response.status());
                    false
                }
            }
            Err(e) => {
                log::warn!("Failed to delete torrent: {}", e);
                false
            }
        }
    }

    pub async fn pause_torrent(&mut self, hash: &str) -> bool {
        if !self.ensure_logged_in().await {
            return false;
        }

        let url = format!("{}torrents/pause", self.base_url);
        let params = [("hashes", hash)];

        match self.client.post(&url).headers(self.auth_headers()).form(&params).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    log::info!("Successfully paused torrent: {}", hash);
                    true
                } else {
                    log::warn!("Failed to pause torrent: {}", response.status());
                    false
                }
            }
            Err(e) => {
                log::warn!("Failed to pause torrent: {}", e);
                false
            }
        }
    }

    pub async fn resume_torrent(&mut self, hash: &str) -> bool {
        if !self.ensure_logged_in().await {
            return false;
        }

        let url = format!("{}torrents/resume", self.base_url);
        let params = [("hashes", hash)];

        match self.client.post(&url).headers(self.auth_headers()).form(&params).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    log::info!("Successfully resumed torrent: {}", hash);
                    true
                } else {
                    log::warn!("Failed to resume torrent: {}", response.status());
                    false
                }
            }
            Err(e) => {
                log::warn!("Failed to resume torrent: {}", e);
                false
            }
        }
    }

    pub async fn get_transfer_info(&mut self) -> TransferInfo {
        if !self.ensure_logged_in().await {
            return TransferInfo::default();
        }

        let url = format!("{}transfer/info", self.base_url);

        match self.client.get(&url).headers(self.auth_headers()).send().await {
            Ok(response) => {
                match response.json::<ApiTransferInfo>().await {
                    Ok(info) => TransferInfo {
                        download_speed: info.dl_info_speed,
                        upload_speed: info.up_info_speed,
                        dht_nodes: info.dht_nodes.unwrap_or(0),
                    },
                    Err(e) => {
                        log::warn!("Failed to parse transfer info: {}", e);
                        TransferInfo::default()
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to get transfer info: {}", e);
                TransferInfo::default()
            }
        }
    }
}

impl Default for TransferInfo {
    fn default() -> Self {
        Self {
            download_speed: 0,
            upload_speed: 0,
            dht_nodes: 0,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ApiTorrent {
    hash: String,
    name: String,
    state: String,
    progress: f64,
    size: u64,
    dlspeed: u64,
    upspeed: u64,
    eta: i64,
    num_seeds: u32,
    num_leechs: u32,
    category: String,
    added_on: i64,
    #[serde(default)]
    content_path: String,
    #[serde(default)]
    save_path: String,
}

impl From<ApiTorrent> for Torrent {
    fn from(api: ApiTorrent) -> Self {
        Torrent {
            hash: api.hash,
            name: api.name,
            state: api.state,
            progress: api.progress,
            size: api.size,
            download_speed: api.dlspeed,
            upload_speed: api.upspeed,
            eta: api.eta,
            seeds: api.num_seeds,
            leechs: api.num_leechs,
            category: api.category,
            added_on: api.added_on,
            content_path: api.content_path,
            save_path: api.save_path,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ApiTransferInfo {
    #[serde(rename = "dl_info_speed")]
    dl_info_speed: u64,
    #[serde(rename = "up_info_speed")]
    up_info_speed: u64,
    #[serde(rename = "dht_nodes")]
    dht_nodes: Option<u32>,
}