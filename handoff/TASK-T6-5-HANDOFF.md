```yaml
task_id: T6-5
goal: |
  重构 RegexTool：移除「内置模板生成」子 Tab，改为「用户给一句自然语言描述
  → 自动构造正则」的新模式。后端新增 regex_construct(statement: String) 命令，
  用规则化推断（非 LLM）从语句中提取数字位数、字符集、锚定等线索，组装正则。

in_scope:
  - crates/core/src/tools/regex_construct.rs          # 新模块：statement → 正则（规则化推断）
  - crates/core/src/tools/mod.rs                      # 注册新模块 + re-export
  - src-tauri/src/commands.rs                        # 新增 #[tauri::command] regex_construct(statement) -> ConstructedRegex
  - src-tauri/src/main.rs                             # 注册命令
  - frontend/src/tauri.js                             # 新增 regexConstruct(statement) 封装
  - frontend/src/components/RegexTool.jsx             # 移除 TemplateTab，改为 ConstructTab（语句输入 + 构造按钮 + 结果 + 测试样例高亮）
  - frontend/src/state.js                            # 移除 regexTemplateSelected/regexTemplateParams/regexGenerated，新增 regexConstructInput/regexConstructResult

out_of_scope:
  - 不改 regex_explain 模块（保留「解析正则」子 Tab 不动）
  - 不改 SqlParseTool / ToolsView 容器（T6-3 负责 Select 改造，本任务只动 RegexTool 内部）
  - 不改 search 后端
  - 不引入 LLM / 不调用网络（规则化推断，纯本地）
  - 不外发数据：statement 是用户本地输入，构造过程纯本地

acceptance_criteria:
  - 新增 core 函数 construct_regex(statement: &str) -> Result<ConstructedRegex, CoreError>
  - 支持至少 6 类线索推断：
    1. 位数：「11 位手机号」→ ^\d{11}$
    2. 字符集：「大写字母 8 位」→ ^[A-Z]{8}$
    3. 锚定/前缀：「以 1 开头 11 位」→ ^1\d{10}$
    4. 邮箱语义：「邮箱」→ ^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$
    5. URL 语义：「http(s) 链接」→ ^https?://...$
    6. 身份证语义：「18 位身份证」→ ^\d{17}[\dXx]$
  - 返回结构 { pattern: String, explanation: String, matched_clues: Vec<String> }，让前端展示推断依据
  - 新增 ≥8 单测覆盖 6 类 + 负例「无法识别的语句」返回 Err
  - 移除 regex_template.rs 的 pub 接口（generate_regex/list_regex_templates）从 mod.rs re-export；regex_template.rs 文件保留但不再被 mod.rs 引用（避免删文件影响 e2e）
  - RegexTool.jsx 移除 TemplateTab 与 TemplateTab 相关 state/action，新增 ConstructTab
  - cd frontend && npm run build 通过
  - cargo test --workspace 全绿

verification_commands:
  - cargo test --workspace
  - cargo test -p ruT0-data-kit-core construct_regex
  - cd frontend && npm run build
  - grep -rn "regexTemplateSelected\|generate_regex\|listRegexTemplates" frontend/src/   # 应 0 命中

files_likely_to_change:
  - crates/core/src/tools/regex_construct.rs         # 新增
  - crates/core/src/tools/mod.rs
  - src-tauri/src/commands.rs
  - src-tauri/src/main.rs
  - frontend/src/tauri.js
  - frontend/src/components/RegexTool.jsx
  - frontend/src/state.js

risks:
  - 规则化推断覆盖面有限，用户描述模糊时返回 Err；mitigate：返回 matched_clues 让用户看到部分识别了什么，UI 提示「请补充：位数/字符集/锚定」。
  - 既有 regex_template e2e 测试可能依赖 generate_regex；迁移时把相关 e2e 改为 construct_regex 等价断言，不破既有 search/sql_parse e2e。
  - state.js 移除旧字段时，RegexTool.jsx 必须同步移除引用，否则 npm build 报错。
  - 正则生成必须用 Rust regex crate 兼容语法（无 look-around / 无 backref），生成后用 regex::Regex::new 校验可编译。

depends_on: [T6-1]
status: planned
```

## 上下文

**用户原话**：「优化正则解析工具，不是要求内置模版，而是我给出一个语句，能自动化帮我构造」。

**既有现状**：
- `crates/core/src/tools/regex_template.rs`（325 行）：8 个内置模板（email/phone_cn/idcard_cn/ipv4/url/sql_injection_bool/sql_injection_union/mac），按 params_schema 填参生成。
- `crates/core/src/tools/regex_explain.rs`（635 行）：手写状态机解释已有正则的 token——这个能力保留（「解析」子 Tab）。
- `RegexTool.jsx` 当前两个子 Tab：`explain`（解析，保留）+ `template`（模板生成，移除）。
- state.js 有 `regexTemplateSelected/regexTemplateParams/regexGenerated` + `regexSubTab`（"explain" | "template"）。

**本任务方案**：
1. 新增 `crates/core/src/tools/regex_construct.rs`，实现规则化推断：用关键词匹配（中文「位」「以...开头」「邮箱」「身份证」「URL」「链接」「大写」「小写」「数字」「字母」）+ 位数/字符集/锚定组合，组装正则字符串，最后用 `regex::Regex::new` 校验可编译。
2. 不引入 LLM、不联网——纯本地规则推断，符合「不外发数据」约束。
3. 前端 RegexTool 把 TemplateTab 替换为 ConstructTab：TextArea 输入一句描述 → 调 `regexConstruct(statement)` → 展示 pattern + explanation + matched_clues + 测试样例高亮（复用既有 highlightMatches 辅助）。
4. 既有 regex_template.rs 文件保留（不删，避免 e2e 误引用），但 mod.rs 不再 re-export 其 pub 接口。

**安全约束**：不外发数据；全本地处理；规则与样本不上传。construct 命令纯本地正则推断。
