import { invoke } from "@tauri-apps/api/core";

/**
 * 调用 `detect_tshark` IPC：自动探测本机 tshark。
 * @returns {Promise<{path: string, version: string} | null>} TsharkInfo 或 null
 */
export function detectTshark() {
  return invoke("detect_tshark");
}

/**
 * 调用 `load_tshark_path` IPC：启动时加载 settings.json 中的 tshark 路径。
 * @returns {Promise<string | null>} 已保存的 tshark 路径或 null
 */
export function loadTsharkPath() {
  return invoke("load_tshark_path");
}

/**
 * 调用 `save_tshark_path` IPC：保存 tshark 路径到 settings.json + 注入运行时。
 * @param {string | null} path - tshark 绝对路径；null 清除覆盖
 * @returns {Promise<void>}
 */
export function saveTsharkPath(path) {
  return invoke("save_tshark_path", { path });
}

/**
 * 调用 `load_page_size` IPC：启动时加载 settings.json 中的全局每页行数。
 * v1.1.2 新增。
 * @returns {Promise<number | null>} 已保存的每页行数或 null（未配置时回退默认 50）
 */
export function loadPageSize() {
  return invoke("load_page_size");
}

/**
 * 调用 `save_page_size` IPC：保存全局每页行数到 settings.json。
 * v1.1.2 新增。
 * @param {number | null} pageSize - 每页行数；null 清除（回退默认 50）
 * @returns {Promise<void>}
 */
export function savePageSize(pageSize) {
  return invoke("save_page_size", { pageSize });
}

/**
 * 调用 `check_update` IPC：检查应用更新（无网络/无新版本统一降级 available=false）。
 * @returns {Promise<{available: boolean, version: string|null, notes: string|null}>} UpdateStatus（camelCase）
 */
export function checkUpdate() {
  return invoke("check_update");
}

/**
 * 调用 `install_update` IPC：安装已下载的更新（重启后生效）。
 * @returns {Promise<void>}
 */
export function installUpdate() {
  return invoke("install_update");
}
