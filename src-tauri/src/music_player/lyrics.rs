use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use serde::Serialize;

/// 单行歌词
#[derive(Serialize, Clone, Debug)]
pub struct LyricLine {
    pub time_ms: u64,
    pub text: String,
}

/// 歌词数据
#[derive(Serialize, Clone, Debug)]
pub struct Lyrics {
    pub lines: Vec<LyricLine>,
    pub source: String,
    /// 歌曲时长（毫秒），从 API 获取
    pub duration_ms: Option<u64>,
}

/// 歌词缓存：key = "title-artist"
pub struct LyricsCache {
    cache: Mutex<HashMap<String, Option<Lyrics>>>,
}

impl LyricsCache {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn get(&self, key: &str) -> Option<Option<Lyrics>> {
        self.cache.lock().unwrap().get(key).cloned()
    }

    pub fn insert(&self, key: String, lyrics: Option<Lyrics>) {
        self.cache.lock().unwrap().insert(key, lyrics);
    }
}

/// 解析 LRC 格式歌词
fn parse_lrc(lrc: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();
    for line in lrc.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // 匹配 [mm:ss.xx] 或 [mm:ss.xxx] 或 [mm:ss]
        let bytes = line.as_bytes();
        let mut i = 0;
        let mut timestamps = Vec::new();
        while i < bytes.len() && bytes[i] == b'[' {
            // 找到 ]
            if let Some(end) = line[i..].find(']') {
                let tag = &line[i + 1..i + end];
                if let Some(ms) = parse_timestamp(tag) {
                    timestamps.push(ms);
                }
                i += end + 1;
            } else {
                break;
            }
        }
        let text = line[i..].trim().to_string();
        if text.is_empty() {
            continue;
        }
        for ts in &timestamps {
            lines.push(LyricLine {
                time_ms: *ts,
                text: text.clone(),
            });
        }
    }
    lines.sort_by_key(|l| l.time_ms);
    lines
}

/// 解析 mm:ss.xx 时间戳为毫秒
fn parse_timestamp(tag: &str) -> Option<u64> {
    let parts: Vec<&str> = tag.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let mins: u64 = parts[0].parse().ok()?;
    let sec_parts: Vec<&str> = parts[1].split('.').collect();
    let secs: u64 = sec_parts[0].parse().ok()?;
    let ms: u64 = if sec_parts.len() > 1 {
        let frac = sec_parts[1];
        match frac.len() {
            1 => frac.parse::<u64>().ok()? * 100,
            2 => frac.parse::<u64>().ok()? * 10,
            3 => frac.parse::<u64>().ok()?,
            _ => 0,
        }
    } else {
        0
    };
    Some(mins * 60_000 + secs * 1000 + ms)
}

/// 从网易云搜索歌词
async fn fetch_netease_lyrics(title: &str, artist: &str) -> Option<Lyrics> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;

    let query = format!("{} {}", title, artist);
    let search_url = format!(
        "https://music.163.com/api/search/get?s={}&type=1&limit=1",
        urlencoding::encode(&query)
    );

    let resp = client.get(&search_url)
        .header("Referer", "https://music.163.com")
        .header("User-Agent", "Mozilla/5.0")
        .send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;
    let songs = json.get("result")?.get("songs")?.as_array()?;
    let song = songs.first()?;
    let song_id = song.get("id")?.as_i64()?;
    let duration_ms = song.get("duration").and_then(|d| d.as_u64());

    let lyric_url = format!("https://music.163.com/api/song/lyric?id={}&lv=1&kv=1&tv=-1", song_id);
    let resp = client.get(&lyric_url)
        .header("Referer", "https://music.163.com")
        .header("User-Agent", "Mozilla/5.0")
        .send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;

    let lrc_text = json.get("lrc")?.get("lyric")?.as_str()?;
    let lines = parse_lrc(lrc_text);
    if lines.is_empty() {
        return None;
    }
    Some(Lyrics {
        lines,
        source: "netease".to_string(),
        duration_ms,
    })
}

/// 从 LRCLIB 搜索歌词
async fn fetch_lrclib_lyrics(title: &str, artist: &str, album: &str) -> Option<Lyrics> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;

    let url = format!(
        "https://lrclib.net/api/search?track_name={}&artist_name={}&album_name={}",
        urlencoding::encode(title),
        urlencoding::encode(artist),
        urlencoding::encode(album),
    );

    let resp = client.get(&url)
        .header("User-Agent", "galncelet/0.1")
        .send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;
    let results = json.as_array()?;

    // 优先 synced lyrics，其次 plain
    for item in results {
        let duration_ms = item.get("duration").and_then(|d| d.as_f64()).map(|d| (d * 1000.0) as u64);
        if let Some(synced) = item.get("syncedLyrics").and_then(|v| v.as_str()) {
            let lines = parse_lrc(synced);
            if !lines.is_empty() {
                return Some(Lyrics {
                    lines,
                    source: "lrclib".to_string(),
                    duration_ms,
                });
            }
        }
        if let Some(plain) = item.get("plainLyrics").and_then(|v| v.as_str()) {
            let lines: Vec<LyricLine> = plain
                .lines()
                .enumerate()
                .map(|(i, l)| LyricLine {
                    time_ms: i as u64 * 5000, // 估算间隔
                    text: l.to_string(),
                })
                .filter(|l| !l.text.is_empty())
                .collect();
            if !lines.is_empty() {
                return Some(Lyrics {
                    lines,
                    source: "lrclib".to_string(),
                    duration_ms,
                });
            }
        }
    }
    None
}

/// 多源回退获取歌词
pub async fn fetch_lyrics(
    cache: &LyricsCache,
    title: &str,
    artist: &str,
    album: &str,
) -> Option<Lyrics> {
    if title.is_empty() {
        return None;
    }

    let key = format!("{}-{}", title, artist);
    if let Some(cached) = cache.get(&key) {
        return cached;
    }

    // 回退链：网易云 → LRCLIB
    let result = fetch_netease_lyrics(title, artist)
        .await
        .or_else(|| {
            // tokio runtime 里不能直接 .or_else(fetch_lrclib) 因为 async
            // 用 block_on 不好，这里改用 futures::future::try_join 不合适
            // 直接在同一个 async 函数里做
            None // placeholder, 下面实际实现
        });

    // 如果网易云没拿到，试 LRCLIB
    let result = if result.is_none() {
        fetch_lrclib_lyrics(title, artist, album).await
    } else {
        result
    };

    cache.insert(key, result.clone());
    result
}
