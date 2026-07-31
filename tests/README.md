# tests/ — 导入/导出验证数据

每个格式子目录一个 `base.*`，共享 12 列 PII schema，用于验证导入流
（datasource Reader）与导出。两行数据（张三/李四）覆盖批量导入场景。

## 共享列 schema

| 列名 | 含义 | 预设覆盖 |
|---|---|---|
| username | 用户名 | username |
| name | 中文姓名 | name |
| sex | 性别 | — |
| birth | 生日 YYYYMMDD | birth |
| idcard | 身份证号 18 位 | idcard |
| phone | 手机号 11 位 | phone |
| email | 邮箱 | email |
| bankcard | 银行卡号 | bankcard |
| ip | IPv4 地址 | ip |
| mac | MAC 地址 | mac |
| address | 中文地址 | address |
| password | 密码 | password |

## 各格式说明

| 路径 | 格式 | reader 状态 | 说明 |
|---|---|---|---|
| `csv/base.csv` | CSV | ✅ v1.0.0 已实现（CsvReader） | 首行表头，UTF-8 无 BOM |
| `xlsx/base.xlsx` | XLSX | ✅ v1.0.0 已实现（XlsxReader） | 首个工作表 `users`，首行表头 |
| `json/base.json` | JSON | ✅ v1.0.0 已实现（JsonReader） | JSON 数组，每元素一个对象 |
| `jsonl/base.jsonl` | NDJSON | ✅ v1.0.0 已实现（与 JsonReader 共用） | 每行一个 JSON 对象 |
| `sql/base.sql` | SQL | ✅ v1.0.0 已实现（SqlReader，rusqlite in-memory 执行） | CREATE/INSERT/SELECT/UPDATE 多语句，收集所有 SELECT 结果 |
| `txt/base.txt` | 自由文本 | ✅ v1.0.0 已实现（TxtReader） | 整文件读为单 `content` cell，含 PII token 供 extract 提取 |
| `pcap/base.pcap` | PCAP | ✅ v1.0.0 已实现（PcapReader，依赖 tshark） | 二进制，1 条 HTTP GET 请求包（含 phone/idcard 明文头） |

> v1.0.0 `detect_format` 接受 `.csv`/`.xlsx`/`.json`/`.jsonl`/`.txt`/`.sql`/
> `.pcap`/`.pcapng` 全部格式。SQL 走 rusqlite in-memory 执行得到结构化结果
> （而非文本解析）；pcap 走 tshark 子进程提取 HTTP 字段，缺失时返回
> `DependencyMissing("tshark")`，前端据此弹提示并禁用 pcap 导入按钮，
> Settings → tshark 路径可配置（多平台自动探测 + 手动指定）。

## pcap 验证

`pcap/base.pcap` 已通过 tshark 解析验证：
```
$ tshark -r tests/pcap/base.pcap -Y http -T fields -e http.request.method -e http.host -e http.user_agent
GET	example.com	RuT0DataKit-test/1.0
```
