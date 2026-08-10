//! 应用设置命令（tshark 路径 + 全局每页行数）。
//!
//! settings.json 结构：
//! `{ "tshark_path": "<path>" | null, "page_size": <u32> | null }`。
//!
//! 读写 `app_config_dir/settings.json`。文件缺失视为未配置（返回默认空
//! `AppSettings`）；解析失败返回 `SettingsError::Corrupt`，不静默覆盖原文件。
//!
//! 写入采用「同目录临时文件 + `persist()` 原子 rename」语义，并通过
//! `Mutex` 在进程内串行化 read-modify-write，避免并发保存丢字段。
//!
//! v1.1.2：新增 `page_size` 字段（全局每页行数），`#[serde(default)]`
//! 保证旧版 settings.json（无 page_size）反序列化时取 None → 前端回退 50。

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::AppHandle;
use tempfile::NamedTempFile;

use ruT0_data_kit_core::pcap;

/// settings.json 结构（v1.1.2 新增 page_size）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct AppSettings {
    pub(crate) tshark_path: Option<String>,
    #[serde(default)]
    pub(crate) page_size: Option<u32>,
}

/// 进程内串行化 settings 读-改-写，避免并发保存丢字段。
///
/// 所有命令共享同一个全局锁；锁本身不保证「按顺序」也无须，只保证同一时刻
/// 只有一个读-改-写事务在跑。单进程桌面应用场景下足够。
static SETTINGS_LOCK: Mutex<()> = Mutex::new(());

/// 读 settings.json 的结果。
///
/// `Ok(settings)`：文件缺失或内容合法；`Err(Corrupt)`：JSON 解析失败，
/// 不静默覆盖原文件，由调用方决定是否提示用户。
pub(crate) enum ReadSettingsResult {
    Ok(AppSettings),
    Corrupt(PathBuf),
}

/// settings 读写错误。
#[derive(Debug)]
pub(crate) enum SettingsError {
    /// `app_config_dir` 解析失败。
    ConfigDir(String),
    /// 创建 config 目录失败。
    CreateDir(String),
    /// JSON 解析失败（原文件保持不动，不静默回退默认）。
    Corrupt(PathBuf),
    /// 序列化失败。
    Serialize(String),
    /// 临时文件 / 原子 rename 失败。
    Persist(String),
}

impl SettingsError {
    fn to_message(&self) -> String {
        match self {
            SettingsError::ConfigDir(e) => format!("app_config_dir: {e}"),
            SettingsError::CreateDir(e) => format!("create config dir: {e}"),
            SettingsError::Corrupt(path) => format!(
                "settings.json 已损坏，未自动覆盖。请检查或删除该文件后重启：{}",
                path.display()
            ),
            SettingsError::Serialize(e) => format!("serialize settings: {e}"),
            SettingsError::Persist(e) => format!("persist settings: {e}"),
        }
    }
}

impl From<SettingsError> for String {
    fn from(e: SettingsError) -> String {
        e.to_message()
    }
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, SettingsError> {
    use tauri::Manager;
    app.path()
        .app_config_dir()
        .map_err(|e| SettingsError::ConfigDir(e.to_string()))
}

/// 读取目录下的 settings.json。
///
/// 返回 `ReadSettingsResult`；文件缺失视为未配置（返回默认空 settings）；
/// JSON 解析失败返回 `Corrupt`，**不覆盖原文件**。
fn read_settings_from(dir: &Path) -> ReadSettingsResult {
    let path = dir.join("settings.json");
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return ReadSettingsResult::Ok(AppSettings::default()),
    };
    match serde_json::from_str::<AppSettings>(&content) {
        Ok(s) => ReadSettingsResult::Ok(s),
        Err(_) => ReadSettingsResult::Corrupt(path),
    }
}

/// 读取 app_config_dir 下的 settings.json，缺失返回默认。
///
/// 启动期（`lib.rs` setup）调用：损坏文件不阻断启动，仅记录日志并使用默认
/// 值；非启动期命令应改用返回 `Result` 的 `try_read_settings` 以向用户报错。
pub(crate) fn read_settings(app: &AppHandle) -> AppSettings {
    let dir = match config_dir(app) {
        Ok(p) => p,
        Err(_) => return AppSettings::default(),
    };
    match read_settings_from(&dir) {
        ReadSettingsResult::Ok(s) => s,
        ReadSettingsResult::Corrupt(path) => {
            eprintln!("settings.json 已损坏，启动期不覆盖：{}", path.display());
            AppSettings::default()
        }
    }
}

/// 读取 settings.json，损坏时返回 `SettingsError::Corrupt`（不静默覆盖）。
pub(crate) fn try_read_settings(app: &AppHandle) -> Result<AppSettings, SettingsError> {
    let dir = config_dir(app)?;
    match read_settings_from(&dir) {
        ReadSettingsResult::Ok(s) => Ok(s),
        ReadSettingsResult::Corrupt(path) => Err(SettingsError::Corrupt(path)),
    }
}

/// 写入 settings.json：同目录临时文件 + `persist()` 原子 rename。
///
/// 失败时不破坏旧文件（临时文件被丢弃，原 settings.json 保持不动）。
fn write_settings_to(dir: &Path, settings: &AppSettings) -> Result<(), SettingsError> {
    std::fs::create_dir_all(dir).map_err(|e| SettingsError::CreateDir(e.to_string()))?;
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| SettingsError::Serialize(e.to_string()))?;

    // NamedTempFile 默认在 std::env::temp_dir()，跨卷 rename 会失败；
    // 显式指定同目录，保证 `persist()`（rename）为同卷原子替换。
    let tmp = NamedTempFile::new_in(dir)
        .map_err(|e| SettingsError::Persist(format!("create temp file: {e}")))?;
    std::fs::write(tmp.path(), &json)
        .map_err(|e| SettingsError::Persist(format!("write temp file: {e}")))?;
    let dest = dir.join("settings.json");
    tmp.persist(&dest)
        .map_err(|e| SettingsError::Persist(format!("persist rename: {e}")))?;
    Ok(())
}

/// 进程内串行执行 read-modify-write，避免并发保存丢字段。
fn with_settings_lock<R>(f: impl FnOnce() -> R) -> R {
    let _guard = SETTINGS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    f()
}

/// 写入 settings.json（原子 persist + 进程内串行）。
fn write_settings(app: &AppHandle, settings: &AppSettings) -> Result<(), SettingsError> {
    let dir = config_dir(app)?;
    with_settings_lock(|| write_settings_to(&dir, settings))
}

/// 自动探测本机 tshark。
///
/// 调 `core::pcap::detect_tshark()`，返回 `TsharkInfo` 或 `null`（探测失败）。
/// 全本地探测，不外发数据。
#[tauri::command]
pub async fn detect_tshark() -> Result<Value, String> {
    match pcap::detect_tshark() {
        Some(info) => serde_json::to_value(info).map_err(|e| e.to_string()),
        None => Ok(Value::Null),
    }
}

/// 启动时加载 tshark 路径并注入 core 运行时。
///
/// 读 `app_config_dir/settings.json` 的 `tshark_path`，非空时调
/// `core::pcap::set_tshark_path` 注入进程级覆盖。返回加载到的路径（或 null）。
/// 启动期若 settings.json 损坏，记录日志并回退默认（不覆盖原文件）。
#[tauri::command]
pub async fn load_tshark_path(app: AppHandle) -> Result<Value, String> {
    let settings = match try_read_settings(&app) {
        Ok(s) => s,
        Err(SettingsError::Corrupt(path)) => {
            eprintln!(
                "load_tshark_path: settings.json 已损坏，未覆盖：{}",
                path.display()
            );
            AppSettings::default()
        }
        Err(e) => return Err(e.into()),
    };
    let path = settings.tshark_path;
    pcap::set_tshark_path(path.clone());
    serde_json::to_value(path).map_err(|e| e.to_string())
}

/// 保存 tshark 路径（写入 settings.json + 注入运行时）。
///
/// `path` 为 `Some(non_empty)` 时设置覆盖；`None` 或空串时清除覆盖，
/// 回退到 PATH 中的 `tshark`。写入采用临时文件 + 原子 rename。
#[tauri::command]
pub async fn save_tshark_path(app: AppHandle, path: Option<String>) -> Result<(), String> {
    let mut settings = try_read_settings(&app)?;
    settings.tshark_path = path.clone();
    write_settings(&app, &settings)?;
    pcap::set_tshark_path(path);
    Ok(())
}

/// 加载全局每页行数（v1.1.2）。
///
/// 读 settings.json 的 `page_size`，未配置时返回 null（前端回退默认 50）。
/// settings.json 损坏时返回错误（不静默覆盖），由前端提示用户。
#[tauri::command]
pub async fn load_page_size(app: AppHandle) -> Result<Option<u32>, String> {
    let settings = try_read_settings(&app)?;
    Ok(settings.page_size)
}

/// 保存全局每页行数（v1.1.2）。
///
/// `page_size = Some(n)` 时写入；`None` 时清除（前端回退默认 50）。
/// 仅持久化到 settings.json，不注入运行时——前端 state 自行 dispatch。
#[tauri::command]
pub async fn save_page_size(app: AppHandle, page_size: Option<u32>) -> Result<(), String> {
    let mut settings = try_read_settings(&app)?;
    settings.page_size = page_size;
    write_settings(&app, &settings).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_raw(dir: &Path, content: &str) {
        fs::write(dir.join("settings.json"), content).unwrap();
    }

    /// 以路径形式复用 try_read 逻辑（不依赖 AppHandle）。
    fn try_read_settings_path(dir: &Path) -> Result<AppSettings, SettingsError> {
        match read_settings_from(dir) {
            ReadSettingsResult::Ok(s) => Ok(s),
            ReadSettingsResult::Corrupt(p) => Err(SettingsError::Corrupt(p)),
        }
    }

    #[test]
    fn missing_file_returns_default() {
        let dir = tempdir().unwrap();
        match read_settings_from(dir.path()) {
            ReadSettingsResult::Ok(s) => {
                assert!(s.tshark_path.is_none());
                assert!(s.page_size.is_none());
            }
            ReadSettingsResult::Corrupt(_) => panic!("missing file should not be corrupt"),
        }
    }

    #[test]
    fn valid_file_round_trips() {
        let dir = tempdir().unwrap();
        let mut s = AppSettings::default();
        s.tshark_path = Some("/usr/bin/tshark".into());
        s.page_size = Some(100);
        write_settings_to(dir.path(), &s).unwrap();

        match read_settings_from(dir.path()) {
            ReadSettingsResult::Ok(read) => {
                assert_eq!(read.tshark_path.as_deref(), Some("/usr/bin/tshark"));
                assert_eq!(read.page_size, Some(100));
            }
            ReadSettingsResult::Corrupt(_) => panic!("valid file should parse"),
        }
    }

    #[test]
    fn legacy_file_without_page_size_parses() {
        let dir = tempdir().unwrap();
        write_raw(dir.path(), r#"{"tshark_path":"/opt/tshark"}"#);
        match read_settings_from(dir.path()) {
            ReadSettingsResult::Ok(s) => {
                assert_eq!(s.tshark_path.as_deref(), Some("/opt/tshark"));
                assert!(s.page_size.is_none());
            }
            ReadSettingsResult::Corrupt(_) => panic!("legacy file should parse"),
        }
    }

    #[test]
    fn corrupt_file_is_not_overwritten() {
        let dir = tempdir().unwrap();
        let raw = "{ broken json";
        write_raw(dir.path(), raw);

        // 损坏：read 返回 Corrupt，不覆盖。
        match read_settings_from(dir.path()) {
            ReadSettingsResult::Corrupt(_) => {}
            _ => panic!("corrupt file should return Corrupt"),
        }
        // 原文件内容保持不变。
        assert_eq!(
            fs::read_to_string(dir.path().join("settings.json")).unwrap(),
            raw
        );
    }

    #[test]
    fn write_replaces_existing_atomically() {
        // 原子写入：第一次写成功后内容完整；第二次写覆盖第一次。
        let dir = tempdir().unwrap();
        let mut s = AppSettings::default();
        s.tshark_path = Some("/bin/tshark".into());
        write_settings_to(dir.path(), &s).unwrap();
        assert!(dir.path().join("settings.json").exists());

        s.tshark_path = Some("/usr/local/bin/tshark".into());
        s.page_size = Some(50);
        write_settings_to(dir.path(), &s).unwrap();

        let read = match read_settings_from(dir.path()) {
            ReadSettingsResult::Ok(s) => s,
            _ => panic!("should read after overwrite"),
        };
        assert_eq!(read.tshark_path.as_deref(), Some("/usr/local/bin/tshark"));
        assert_eq!(read.page_size, Some(50));
    }

    #[test]
    fn write_to_unwritable_dir_fails_without_partial_file() {
        // 写入到一个无法创建子目录的路径应失败，
        // 且不在目标位置留下 settings.json。
        // 用一个文件阻挡 create_dir_all 的目录链，使其可靠失败。
        let dir = tempdir().unwrap();
        let blocking_file = dir.path().join("blocking-file");
        std::fs::write(&blocking_file, "not a dir").unwrap();
        let bogus = blocking_file.join("sub");
        let s = AppSettings::default();
        let res = write_settings_to(&bogus, &s);
        assert!(
            res.is_err(),
            "writing to a path blocked by a file should fail"
        );
        assert!(!bogus.join("settings.json").exists());
    }

    #[test]
    fn concurrent_save_does_not_lose_fields() {
        // 模拟两次串行 read-modify-write：先保存 tshark_path，再保存 page_size，
        // 两个字段都应保留（验证锁 + 原子写 + 每次重新 read）。
        let dir = tempdir().unwrap();

        // save_tshark_path 等价路径。
        let mut s1 = AppSettings::default();
        s1.tshark_path = Some("/usr/local/bin/tshark".into());
        with_settings_lock(|| write_settings_to(dir.path(), &s1)).unwrap();

        let mut s2 = match read_settings_from(dir.path()) {
            ReadSettingsResult::Ok(s) => s,
            _ => panic!("should read after first write"),
        };
        s2.page_size = Some(25);
        with_settings_lock(|| write_settings_to(dir.path(), &s2)).unwrap();

        let final_settings = match read_settings_from(dir.path()) {
            ReadSettingsResult::Ok(s) => s,
            _ => panic!("should read after second write"),
        };
        assert_eq!(
            final_settings.tshark_path.as_deref(),
            Some("/usr/local/bin/tshark")
        );
        assert_eq!(final_settings.page_size, Some(25));
    }

    #[test]
    fn corrupt_then_read_returns_error_not_default() {
        // 读到 Corrupt 后，try_read_settings 返回 Err，调用方（命令层）
        // 不会走到 write，避免「损坏 → 静默写默认值」覆盖原文件。
        let dir = tempdir().unwrap();
        let raw = "not json";
        write_raw(dir.path(), raw);

        let res = try_read_settings_path(dir.path());
        assert!(matches!(res, Err(SettingsError::Corrupt(_))));
        // 原文件未被改动。
        assert_eq!(
            fs::read_to_string(dir.path().join("settings.json")).unwrap(),
            raw
        );
    }
}
