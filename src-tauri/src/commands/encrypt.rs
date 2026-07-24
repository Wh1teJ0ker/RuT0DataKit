//! 加密 / 编码命令：单值 + 批量列（4 命令，T13-2）。
//!
//! 委托 `ruT0_data_kit_core::tools::encrypt` 的纯本地实现，无任何网络依赖。
//! 单值命令 `encrypt_text` / `decrypt_text` 镜像 `mask::trial_mask` 的单值模式；
//! 批量列命令 `encrypt_columns` / `decrypt_columns` 镜像 `mask::apply_rules_cols_records`
//! 的 headers+rows+selected_columns 批量模式，未选列原样保留。

use ruT0_data_kit_core::tools::encrypt::{decrypt_text as core_decrypt, encrypt_text as core_encrypt, EncryptAlgo};
use serde_json::{json, Value};

/// 把前端传入的算法字符串映射为 core 的 [`EncryptAlgo`]。
///
/// 支持的别名：
/// - `aes` / `aes-cbc` → `EncryptAlgo::AesCbc`
/// - `base64` → `EncryptAlgo::Base64`
/// - `hex` → `EncryptAlgo::Hex`
///
/// 大小写不敏感。无效算法返回友好错误，不 panic。
fn parse_algo(algo: &str) -> Result<EncryptAlgo, String> {
    match algo.to_ascii_lowercase().as_str() {
        "aes" | "aes-cbc" => Ok(EncryptAlgo::AesCbc),
        "base64" => Ok(EncryptAlgo::Base64),
        "hex" => Ok(EncryptAlgo::Hex),
        other => Err(format!(
            "未识别的加密算法: {other}（支持 aes / aes-cbc / base64 / hex）"
        )),
    }
}

/// 单值加密 / 编码：对 `plaintext` 应用 `algo` + `key`，返回结果字符串。
///
/// `key` 仅 AES 需要（任意长度字符串，内部 SHA-256 派生 AES-256 密钥）；
/// Base64 / Hex 忽略 `key`。AES 使用随机 IV（内嵌于密文前缀）。
///
/// 返回 `{ ok: bool, result: Option<String>, error: Option<String> }`：
/// - `ok=true`：`result` 为密文/编码串，`error` 为 None。
/// - `ok=false`：`result` 为 None，`error` 为错误原因。
#[tauri::command]
pub fn encrypt_text(algo: String, plaintext: String, key: String) -> Result<Value, String> {
    let parsed = match parse_algo(&algo) {
        Ok(a) => a,
        Err(e) => {
            return Ok(json!({
                "ok": false,
                "result": null,
                "error": e,
            }));
        }
    };
    match core_encrypt(parsed, &plaintext, &key, "") {
        Ok(ct) => Ok(json!({
            "ok": true,
            "result": ct,
            "error": null,
        })),
        Err(e) => Ok(json!({
            "ok": false,
            "result": null,
            "error": e,
        })),
    }
}

/// 单值解密 / 解码：对 `ciphertext` 应用 `algo` + `key`，返回原文。
///
/// 返回结构与 [`encrypt_text`] 同构（`ok` / `result` / `error`）。
#[tauri::command]
pub fn decrypt_text(algo: String, ciphertext: String, key: String) -> Result<Value, String> {
    let parsed = match parse_algo(&algo) {
        Ok(a) => a,
        Err(e) => {
            return Ok(json!({
                "ok": false,
                "result": null,
                "error": e,
            }));
        }
    };
    match core_decrypt(parsed, &ciphertext, &key, "") {
        Ok(pt) => Ok(json!({
            "ok": true,
            "result": pt,
            "error": null,
        })),
        Err(e) => Ok(json!({
            "ok": false,
            "result": null,
            "error": e,
        })),
    }
}

/// 批量列加密 / 编码：对 `selected_columns` 中的列 cell 调 `encrypt_text`，
/// 未选列原样保留。输入 `headers` / `rows` 即 PreprocessView 归一化产物。
///
/// `skipped_fields` = `selected_columns` 中不在 `headers` 的列名（前端可据此
/// 提示用户哪些选择被跳过，不 panic）。
///
/// 返回 `{ headers, processed_rows, summary, skipped_fields }`：
/// - `headers`：原样回传。
/// - `processed_rows`：与 `rows` 同维度，仅选中列被处理。
/// - `summary`：`{ total_rows, processed_cells, skipped_fields_count, errors }`。
#[tauri::command]
pub fn encrypt_columns(
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    algo: String,
    key: String,
    selected_columns: Vec<String>,
) -> Result<Value, String> {
    process_columns(headers, rows, algo, key, selected_columns, true)
}

/// 批量列解密 / 解码：[`encrypt_columns`] 的逆操作，签名与返回结构同构。
#[tauri::command]
pub fn decrypt_columns(
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    algo: String,
    key: String,
    selected_columns: Vec<String>,
) -> Result<Value, String> {
    process_columns(headers, rows, algo, key, selected_columns, false)
}

/// 批量列处理内部实现，`encrypt` 为 true 走加密路径，false 走解密路径。
fn process_columns(
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    algo: String,
    key: String,
    selected_columns: Vec<String>,
    encrypt: bool,
) -> Result<Value, String> {
    let parsed = match parse_algo(&algo) {
        Ok(a) => a,
        Err(e) => return Err(e),
    };

    // header -> 列索引；同时记录选中但找不到的列名。
    let header_index: std::collections::HashMap<&str, usize> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| (h.as_str(), i))
        .collect();

    let mut skipped_fields: Vec<String> = Vec::new();
    let mut target_indices: Vec<usize> = Vec::new();
    for col in &selected_columns {
        match header_index.get(col.as_str()) {
            Some(&i) => target_indices.push(i),
            None => skipped_fields.push(col.clone()),
        }
    }

    let total_rows = rows.len();
    let mut processed_rows: Vec<Vec<String>> = Vec::with_capacity(total_rows);
    let mut processed_cells: usize = 0;
    let mut errors: usize = 0;

    for row in rows {
        let mut out_row = row.clone();
        for &i in &target_indices {
            // 列索引可能超过某行长度（数据不规范），原样保留并记错。
            let cell = match out_row.get_mut(i) {
                Some(c) => c,
                None => {
                    errors += 1;
                    continue;
                }
            };
            let original = std::mem::take(cell);
            let result = if encrypt {
                core_encrypt(parsed, &original, &key, "")
            } else {
                core_decrypt(parsed, &original, &key, "")
            };
            match result {
                Ok(s) => {
                    *cell = s;
                    processed_cells += 1;
                }
                Err(_) => {
                    // 失败时还原原值，记一次错误，不中断后续 cell / 行。
                    *cell = original;
                    errors += 1;
                }
            }
        }
        processed_rows.push(out_row);
    }

    Ok(json!({
        "headers": headers,
        "processed_rows": processed_rows,
        "summary": {
            "total_rows": total_rows,
            "processed_cells": processed_cells,
            "skipped_fields_count": skipped_fields.len(),
            "errors": errors,
        },
        "skipped_fields": skipped_fields,
    }))
}
