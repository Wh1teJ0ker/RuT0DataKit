```yaml
task_id: T41
goal: |
  加固 tshark 解析的测试覆盖：
  1. 验证 Windows 路径候选（candidate_paths 含 Program Files / Program Files (x86)）已有断言，补强边界
  2. 为 PcapReader::read 新增集成测试：本机有 tshark + 有 fixture 时跑，验证 HTTP 字段解析正确
  3. 新增 tshark stdout 解析逻辑的单元测试（不依赖 tshark，用构造的 stdout 字符串）
in_scope:
  - crates/core/src/pcap/detect.rs（补强 candidate_paths 测试断言）
  - crates/core/src/pcap/reader.rs（新增 stdout 解析单元测试 + 集成测试补强）
out_of_scope:
  - 不改 detect.rs / reader.rs 的生产代码逻辑（仅补测试）
  - 不改非 pcap 模块
  - 不在 CI 强制依赖 tshark（集成测试用 #[ignore] 或条件跳过）
acceptance_criteria:
  - candidate_paths 测试覆盖三平台路径（macOS homebrew + Linux /usr/bin + Windows Program Files / Program Files (x86)）
  - 新增 parse_tshark_stdout 单元测试：构造含 8 字段的 tab 分隔 stdout → 解析出正确的 HttpRequest
  - 新增 parse_tshark_stdout_empty_lines 测试：空行跳过
  - 新增 parse_tshark_stdout_hex_body 测试：http.file_data 十六进制 → 解码为字符串
  - 本机集成测试 read_fixture_pcap 已存在（#[ignore]），补强断言字段非空
  - cargo test --workspace 全绿
verification_commands:
  - cargo fmt --check
  - cargo clippy --workspace -- -D warnings
  - cargo test --workspace
files_likely_to_change:
  - crates/core/src/pcap/detect.rs（测试模块）
  - crates/core/src/pcap/reader.rs（测试模块）
risks:
  - 现有 reader.rs 的解析逻辑内联在 read() 方法里，不易单独测试 stdout 解析。方案：把解析逻辑抽成 parse_tshark_output(stdout: &str) -> Vec<HttpRequest> 私有函数（纯函数，不调 Command），再在 read() 里调用。这属于「测试驱动的最小重构」，不算改生产行为。
depends_on: []
status: planned
```

## 实现指引

### 重构 reader.rs：抽取 parse_tshark_output

把 `read()` 方法中的 stdout 解析逻辑抽成私有函数：

```rust
/// 解析 tshark -T fields 的 stdout 为 HttpRequest 列表。
/// 每行一条记录，tab 分隔 8 字段；空行跳过。
fn parse_tshark_output(stdout: &str) -> Vec<HttpRequest> {
    let mut result = Vec::new();
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        let get = |idx: usize| -> String { parts.get(idx).unwrap_or(&"").to_string() };
        let body_raw = get(6);
        let body = if body_raw.is_empty() {
            String::new()
        } else {
            let bytes = hex_to_bytes(&body_raw);
            String::from_utf8_lossy(&bytes).to_string()
        };
        result.push(HttpRequest {
            frame_no: get(0),
            src_ip: get(1),
            dst_ip: get(2),
            method: get(3),
            host: get(4),
            uri: get(5),
            body,
            user_agent: get(7),
        });
    }
    result
}
```

`read()` 方法改为：拿到 stdout 后调 `parse_tshark_output(&stdout)`。

**这是行为保持的重构**：解析逻辑完全不变，只是抽成可测函数。

### 新增单元测试

```rust
#[test]
fn parse_tshark_output_basic() {
    let stdout = "1\t192.168.1.1\t93.184.216.34\tGET\texample.com\t/\t\tRuT0DataKit-test/1.0";
    let reqs = parse_tshark_output(stdout);
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0].frame_no, "1");
    assert_eq!(reqs[0].src_ip, "192.168.1.1");
    assert_eq!(reqs[0].method, "GET");
    assert_eq!(reqs[0].host, "example.com");
    assert_eq!(reqs[0].user_agent, "RuT0DataKit-test/1.0");
}

#[test]
fn parse_tshark_output_skips_empty_lines() {
    let stdout = "\n\n1\t\t\tGET\thost\t/\t\t\n\n\n2\t\t\tPOST\thost2\t/path\t\t\n";
    let reqs = parse_tshark_output(stdout);
    assert_eq!(reqs.len(), 2);
    assert_eq!(reqs[0].frame_no, "1");
    assert_eq!(reqs[1].frame_no, "2");
}

#[test]
fn parse_tshark_output_hex_body_decoded() {
    // "Hello" = 48656c6c6f
    let stdout = "1\t\t\tPOST\thost\t/api\t48656c6c6f\tua";
    let reqs = parse_tshark_output(stdout);
    assert_eq!(reqs[0].body, "Hello");
}

#[test]
fn parse_tshark_output_missing_fields_padded() {
    // 只有 3 个字段 → 其余补空串
    let stdout = "1\t\tGET";
    let reqs = parse_tshark_output(stdout);
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0].frame_no, "1");
    assert_eq!(reqs[0].method, "GET");
    assert_eq!(reqs[0].host, "");
    assert_eq!(reqs[0].body, "");
}

#[test]
fn parse_tshark_output_empty_input() {
    assert_eq!(parse_tshark_output("").len(), 0);
    assert_eq!(parse_tshark_output("\n\n\n").len(), 0);
}
```

### detect.rs 测试补强

现有 `candidate_paths_covers_three_platforms` 已覆盖三平台。补一个测试验证 Windows 路径格式：

```rust
#[test]
fn candidate_paths_windows_paths_are_absolute() {
    let paths = candidate_paths();
    let win_paths: Vec<_> = paths.iter().filter(|p| p.contains(r"Program Files")).collect();
    assert!(!win_paths.is_empty());
    for p in &win_paths {
        assert!(p.starts_with(r"C:\"));
        assert!(p.ends_with("tshark.exe"));
    }
}
```

### 集成测试

现有 `read_fixture_pcap`（#[ignore]）已存在。补强断言：

```rust
#[test]
#[ignore = "本机 tshark 读取，CI 无 tshark/无样本时跳过"]
fn read_fixture_pcap() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("base")
        .join("base.pcap");
    if !path.exists() {
        return;
    }
    let reqs = PcapReader::new().read(&path).expect("read pcap");
    assert!(!reqs.is_empty(), "fixture pcap 应有 HTTP 请求");
    let first = &reqs[0];
    assert!(!first.frame_no.is_empty());
    assert!(!first.method.is_empty());
    assert!(first.method.contains("GET") || first.method.contains("POST"));
    assert!(!first.host.is_empty());
}
```
