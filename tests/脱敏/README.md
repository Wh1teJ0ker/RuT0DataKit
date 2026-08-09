# tests/脱敏/ — 数据脱敏测试数据

v1.1.3 通用模板脱敏测试数据。全部为合成数据，不含真实个人隐私。

## 脱敏规则（2 条 mask 规则 + 4 个预设）

| 规则 ID | 名称 | 说明 |
|---|---|---|
| `name-mask` | 姓名脱敏 | 旧逻辑（保留首尾各 1，中间 `*` 替换） |
| `general-mask` | 通用脱敏 | 持空模板，前端选预设填充参数；空模板=不脱敏（透传） |

`general-mask` 的 4 个预设（子规则，`TemplateParams` 常量）：

| 预设 | 名称 | 模板参数 | 示例 |
|---|---|---|---|
| 身份证号 | `idcard_preset()` | keep 6/4, mask_min_len 8, len 18 | `110101199001011234` → `110101********1234` |
| 手机号 | `phone_preset()` | keep 3/4, mask_min_len 4, len 11 | `13812345678` → `138****5678` |
| 出生日期 | `birthdate_preset()` | keep 8/0, mask_min_len 2, len 10 | `1990-01-15` → `1990-01-**` |
| 银行卡号 | `bankcard_preset()` | keep 4/4, mask_min_len 1 | `6222021234567890123` → `6222***********0123` |

> T49 子规则化：原 v1.1.3 T48 的 4 条独立规则（`idcard-mask`/`phone-mask`/`birthdate-mask`/`bankcard-mask`）收敛为 `general-mask` 的 4 个预设，不再单独 seed 到 DB。

## 测试数据文件

| 文件 | 列 | 说明 |
|---|---|---|
| `idcard.csv` | idcard, name | 18 位身份证号（含末位 X） |
| `phone.csv` | phone, owner | 11 位手机号 |
| `birthdate.csv` | birthdate, name | YYYY-MM-DD 出生日期 |
| `bankcard.csv` | bankcard, holder | 16/19 位银行卡号 |

## 验证要点

- 选 general-mask + 身份证预设 → `110101********1234`；15 位原样返回（min_len=18 guard）
- 选 general-mask + 手机预设 → `138****5678`；10 位原样返回（min_len=11 guard）
- 选 general-mask + 不脱敏（空模板）→ 原样返回（不脱敏）
- 选 general-mask + 自定义 → 6 个参数框可手动编辑
- 掩码字符输入 `#` → 临时覆盖（身份证预设 → `110101########1234`）
- 选 name-mask → `张三丰` → `张*丰`（旧逻辑，向后兼容）
