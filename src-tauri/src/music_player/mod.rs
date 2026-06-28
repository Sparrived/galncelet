use std::sync::Mutex;
use std::time::Instant;
use serde::Serialize;

pub mod lyrics;

/// SMTC 音乐播放器状态
pub struct MusicPlayerState {
    last_track_key: Mutex<Option<String>>,
    cached_thumbnail: Mutex<Option<String>>,
    selected_session_id: Mutex<Option<String>>,
    /// 客户端位置追踪（SMTC 不报告时间线时使用）
    estimated_position_ms: Mutex<u64>,
    estimated_duration_ms: Mutex<u64>,
    last_poll_instant: Mutex<Option<Instant>>,
    last_playing: Mutex<bool>,
    pub lyrics_cache: lyrics::LyricsCache,
}

impl MusicPlayerState {
    pub fn new() -> Self {
        Self {
            last_track_key: Mutex::new(None),
            cached_thumbnail: Mutex::new(None),
            selected_session_id: Mutex::new(None),
            estimated_position_ms: Mutex::new(0),
            estimated_duration_ms: Mutex::new(0),
            last_poll_instant: Mutex::new(None),
            last_playing: Mutex::new(false),
            lyrics_cache: lyrics::LyricsCache::new(),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct MediaInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub thumbnail: Option<String>,
    pub duration_ms: u64,
    pub position_ms: u64,
    pub is_playing: bool,
    pub shuffle: bool,
    pub repeat_mode: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct MediaSessionInfo {
    pub id: String,
    pub app_name: String,
}

/// 读取缩略图流为字节
fn read_thumbnail_stream(
    stream: &windows::Storage::Streams::IRandomAccessStreamWithContentType,
) -> windows::core::Result<Vec<u8>> {
    let size = stream.Size()?;
    if size == 0 || size > 10 * 1024 * 1024 {
        return Ok(Vec::new());
    }
    let reader = windows::Storage::Streams::DataReader::CreateDataReader(stream)?;
    let loaded = reader.LoadAsync(size as u32)?;
    loaded.get()?;
    let mut buf = vec![0u8; size as usize];
    reader.ReadBytes(&mut buf)?;
    Ok(buf)
}

/// 获取指定或当前活跃的会话
fn get_session(
    manager: &windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager,
    session_id: Option<&str>,
) -> Option<windows::Media::Control::GlobalSystemMediaTransportControlsSession> {
    if let Some(id) = session_id {
        let sessions = manager.GetSessions().ok()?;
        for s in sessions {
            if let Ok(sid) = s.SourceAppUserModelId() {
                if sid.to_string() == id {
                    return Some(s);
                }
            }
        }
        None
    } else {
        manager.GetCurrentSession().ok()
    }
}

/// 查询当前活跃的媒体播放信息
fn request_media_info(
    state: &MusicPlayerState,
    session_id: Option<&str>,
) -> windows::core::Result<Option<MediaInfo>> {
    let manager =
        windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?;
    let manager = manager.get()?;

    let session = match get_session(&manager, session_id) {
        Some(s) => s,
        None => return Ok(None),
    };

    let timeline = session.GetTimelineProperties()?;
    let playback = session.GetPlaybackInfo()?;

    let status = playback.PlaybackStatus().unwrap_or(
        windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus::Closed,
    );
    let is_playing =
        status == windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;

    let props = match session.TryGetMediaPropertiesAsync().and_then(|p| p.get()) {
        Ok(p) => p,
        Err(e) => {
            // Err(S_OK) 的情况：会话存在但媒体属性不可用
            eprintln!("[music-player] TryGetMediaProperties failed: {}", e);
            return Ok(None);
        }
    };

    let title = props.Title().unwrap_or_default().to_string();
    let artist = props.Artist().unwrap_or_default().to_string();
    let album = props.AlbumTitle().unwrap_or_default().to_string();

    let smtc_duration = timeline.EndTime().map(|t| t.Duration.max(0) as u64 / 10_000).unwrap_or(0);
    let smtc_position = timeline.Position().map(|t| t.Duration.max(0) as u64 / 10_000).unwrap_or(0);

    // 客户端位置追踪
    let now = Instant::now();
    let track_key = format!("{}-{}", title, artist);

    // 短暂持锁：检查切歌 + 更新 key（忽略空 metadata）
    let track_changed = {
        let mut guard = state.last_track_key.lock().unwrap();
        if title.is_empty() {
            // SMTC 没返回有效 metadata，不触发切歌
            false
        } else {
            let changed = guard.as_ref() != Some(&track_key);
            if changed {
                eprintln!("[music-player] track changed: {:?}", track_key);
                *guard = Some(track_key.clone());
            }
            changed
        }
    };

    // 位置估算：累加 + SMTC 修正
    let (duration_ms, position_ms) = {
        let mut est_pos = state.estimated_position_ms.lock().unwrap();
        let mut est_dur = state.estimated_duration_ms.lock().unwrap();
        let mut last_instant = state.last_poll_instant.lock().unwrap();

        if track_changed {
            *est_pos = smtc_position;
            if smtc_duration > 0 { *est_dur = smtc_duration; }
            *last_instant = Some(now);
        } else {
            if smtc_duration > 0 { *est_dur = smtc_duration; }
            // 先累加
            if is_playing {
                if let Some(prev) = *last_instant {
                    *est_pos += now.duration_since(prev).as_millis() as u64;
                }
            }
            // SMTC 非零时修正（差值 > 2s 才跳变，避免抖动）
            if smtc_position > 0 {
                let diff = (*est_pos as i64 - smtc_position as i64).unsigned_abs();
                if diff > 2000 {
                    *est_pos = smtc_position;
                }
            }
            *last_instant = Some(now);
        }
        (*est_dur, *est_pos)
    };

    let shuffle = playback.IsShuffleActive()
        .and_then(|r| r.Value())
        .unwrap_or(false);
    let repeat = playback.AutoRepeatMode()
        .and_then(|r| r.Value())
        .unwrap_or(windows::Media::MediaPlaybackAutoRepeatMode::None);
    let repeat_mode = match repeat {
        windows::Media::MediaPlaybackAutoRepeatMode::None => "none",
        windows::Media::MediaPlaybackAutoRepeatMode::Track => "one",
        windows::Media::MediaPlaybackAutoRepeatMode::List => "all",
        _ => "none",
    }
    .to_string();

    let mut last_key = state.last_track_key.lock().unwrap();

    let thumbnail = if track_changed {
        *last_key = Some(track_key);
        let thumb = match props.Thumbnail() {
            Ok(thumb_ref) => match thumb_ref.OpenReadAsync() {
                Ok(async_op) => match async_op.get() {
                    Ok(stream) => match read_thumbnail_stream(&stream) {
                        Ok(bytes) if !bytes.is_empty() => {
                            use base64::Engine;
                            Some(base64::engine::general_purpose::STANDARD.encode(&bytes))
                        }
                        _ => None,
                    },
                    Err(_) => None,
                },
                Err(_) => None,
            },
            Err(_) => None,
        };
        *state.cached_thumbnail.lock().unwrap() = thumb.clone();
        thumb
    } else {
        state.cached_thumbnail.lock().unwrap().clone()
    };

    Ok(Some(MediaInfo {
        title,
        artist,
        album,
        thumbnail,
        duration_ms,
        position_ms,
        is_playing,
        shuffle,
        repeat_mode,
    }))
}

/// Tauri 命令：获取当前媒体播放信息
#[tauri::command]
pub async fn get_media_info(
    state: tauri::State<'_, std::sync::Arc<MusicPlayerState>>,
) -> Result<Option<MediaInfo>, String> {
    let sid = state.selected_session_id.lock().unwrap().clone();
    // 如果没选中任何会话，尝试自动选中唯一可用的
    let effective_sid = if sid.is_none() {
        let manager =
            windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
                .and_then(|m| m.get());
        if let Ok(mgr) = manager {
            if let Ok(sessions) = mgr.GetSessions() {
                let size = sessions.Size().unwrap_or(0);
                if size == 1 {
                    if let Ok(s) = sessions.GetAt(0) {
                        s.SourceAppUserModelId().ok().map(|h| h.to_string()).filter(|id| !id.is_empty())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };
    let sid_ref = effective_sid.as_deref().or(sid.as_deref());
    match request_media_info(&state, sid_ref) {
        Ok(info) => Ok(info),
        Err(e) => {
            eprintln!("[music-player] SMTC error: {}", e);
            Ok(None)
        }
    }
}

/// Tauri 命令：列出所有 SMTC 会话
#[tauri::command]
pub async fn get_media_sessions() -> Result<Vec<MediaSessionInfo>, String> {
    let manager =
        windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            .map_err(|e| e.to_string())?;
    let manager = manager.get().map_err(|e| e.to_string())?;

    let sessions = manager.GetSessions().map_err(|e| e.to_string())?;
    let count = sessions.Size().unwrap_or(0);
    let mut result = Vec::new();
    for i in 0..count {
        let s = sessions.GetAt(i).map_err(|e| e.to_string())?;
        let id = s
            .SourceAppUserModelId()
            .map(|h| h.to_string())
            .unwrap_or_default();
        if id.is_empty() {
            continue;
        }
        let app_name = id.clone();
        result.push(MediaSessionInfo {
            id,
            app_name,
        });
    }
    Ok(result)
}

/// Tauri 命令：选择要控制的会话
#[tauri::command]
pub async fn select_media_session(
    state: tauri::State<'_, std::sync::Arc<MusicPlayerState>>,
    session_id: Option<String>,
) -> Result<(), String> {
    *state.selected_session_id.lock().unwrap() = session_id;
    Ok(())
}

/// 媒体控制命令
#[derive(serde::Deserialize, Clone, Debug)]
pub enum MediaAction {
    Play,
    Pause,
    Toggle,
    Next,
    Previous,
    SetPosition(u64),
    SetShuffle(bool),
    CycleRepeat,
}

/// Tauri 命令：控制媒体播放
#[tauri::command]
pub async fn media_control(
    state: tauri::State<'_, std::sync::Arc<MusicPlayerState>>,
    action: MediaAction,
) -> Result<bool, String> {
    let manager =
        windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            .map_err(|e| format!("SMTC manager error: {}", e))?;
    let manager = manager
        .get()
        .map_err(|e| format!("SMTC manager get error: {}", e))?;

    let sid = state.selected_session_id.lock().unwrap().clone();
    // 自动选中唯一可用会话
    let effective_sid = if sid.is_none() {
        if let Ok(sessions) = manager.GetSessions() {
            let size = sessions.Size().unwrap_or(0);
            if size == 1 {
                sessions.GetAt(0).ok()
                    .and_then(|s| s.SourceAppUserModelId().ok())
                    .map(|h| h.to_string())
                    .filter(|id| !id.is_empty())
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };
    let sid_ref = effective_sid.as_deref().or(sid.as_deref());
    let session = get_session(&manager, sid_ref)
        .ok_or("No active media session".to_string())?;

    let result = match action {
        MediaAction::Play => {
            session
                .TryPlayAsync()
                .map_err(|e| e.to_string())?
                .get()
                .map_err(|e| e.to_string())?
        }
        MediaAction::Pause => {
            session
                .TryPauseAsync()
                .map_err(|e| e.to_string())?
                .get()
                .map_err(|e| e.to_string())?
        }
        MediaAction::Toggle => {
            let is_playing = session
                .GetPlaybackInfo()
                .map_err(|e| e.to_string())?
                .PlaybackStatus()
                .map_err(|e| e.to_string())?
                == windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;
            if is_playing {
                session
                    .TryPauseAsync()
                    .map_err(|e| e.to_string())?
                    .get()
                    .map_err(|e| e.to_string())?
            } else {
                session
                    .TryPlayAsync()
                    .map_err(|e| e.to_string())?
                    .get()
                    .map_err(|e| e.to_string())?
            }
        }
        MediaAction::Next => {
            session
                .TrySkipNextAsync()
                .map_err(|e| e.to_string())?
                .get()
                .map_err(|e| e.to_string())?
        }
        MediaAction::Previous => {
            session
                .TrySkipPreviousAsync()
                .map_err(|e| e.to_string())?
                .get()
                .map_err(|e| e.to_string())?
        }
        MediaAction::SetPosition(ms) => {
            let position_100ns = (ms as i64) * 10_000;
            session
                .TryChangePlaybackPositionAsync(position_100ns)
                .map_err(|e| e.to_string())?
                .get()
                .map_err(|e| e.to_string())?
        }
        MediaAction::SetShuffle(enabled) => {
            session
                .TryChangeShuffleActiveAsync(enabled)
                .map_err(|e| e.to_string())?
                .get()
                .map_err(|e| e.to_string())?
        }
        MediaAction::CycleRepeat => {
            let current = session
                .GetPlaybackInfo()
                .map_err(|e| e.to_string())?
                .AutoRepeatMode()
                .map_err(|e| e.to_string())?
                .Value();
            let next = match current {
                Ok(windows::Media::MediaPlaybackAutoRepeatMode::None) => {
                    windows::Media::MediaPlaybackAutoRepeatMode::List
                }
                Ok(windows::Media::MediaPlaybackAutoRepeatMode::List) => {
                    windows::Media::MediaPlaybackAutoRepeatMode::Track
                }
                _ => windows::Media::MediaPlaybackAutoRepeatMode::None,
            };
            session
                .TryChangeAutoRepeatModeAsync(next)
                .map_err(|e| e.to_string())?
                .get()
                .map_err(|e| e.to_string())?
        }
    };

    Ok(result)
}

// ─── Lyrics ──────────────────────────────────────────────────────

/// Tauri 命令：获取歌词
#[tauri::command]
pub async fn get_lyrics(
    state: tauri::State<'_, std::sync::Arc<MusicPlayerState>>,
    title: String,
    artist: String,
    album: String,
) -> Result<Option<lyrics::Lyrics>, String> {
    let result = lyrics::fetch_lyrics(&state.lyrics_cache, &title, &artist, &album).await;
    // 如果歌词 API 返回了 duration，用它补充 est_dur
    if let Some(ref lyrics) = result {
        if let Some(dur) = lyrics.duration_ms {
            if dur > 0 {
                let mut est_dur = state.estimated_duration_ms.lock().unwrap();
                if *est_dur == 0 {
                    *est_dur = dur;
                }
            }
        }
    }
    Ok(result)
}

// ─── Plugin Setup ──────────────────────────────────────────────────

/// Initialize music player plugin.
pub fn setup(app: &tauri::AppHandle) {
    use tauri::Manager;
    use std::sync::Arc;
    let state = Arc::new(MusicPlayerState::new());
    app.manage(state);
}
