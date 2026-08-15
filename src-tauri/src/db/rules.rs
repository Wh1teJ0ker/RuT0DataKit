//! 规则 CRUD：`upsert_rule` / `list_rules` / `get_rule` /
//! `set_rule_enabled` / `count_rules` / `update_rule_params` /
//! `update_rule_template` / `update_rule_extract_config` /
//! `seed_builtin_rules`（含废弃 id 清理）。
//!
//! T91 从 `db/mod.rs` 拆出；方法签名 / SQL / 测试逻辑保持不变。

use super::{row_to_rule, DbManager, DbError};
#[allow(unused_imports)]
use ruT0_data_kit_core::processor::rules::{
    ExtractParams, Rule, RuleKind, RuleRegistry, TemplateParams,
};
use rusqlite::params;

// v1.1+ IPC 将调用；单测已覆盖。
#[allow(
    dead_code,
    reason = "v1.1+ IPC 将接入（list_rules/get_rule 等）；单测已覆盖"
)]
impl DbManager {
    // ---- Rule CRUD（v1.1.0）----

    /// upsert 一条规则（按 id 冲突覆盖）。
    /// `template` 列存 `TemplateParams` JSON（None → NULL）。
    pub fn upsert_rule(&self, rule: &Rule) -> Result<(), DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let template_json: Option<String> = rule
            .template
            .as_ref()
            .map(|t| serde_json::to_string(t).unwrap_or_default());
        let params_json: Option<String> = rule
            .params
            .as_ref()
            .map(|p| serde_json::to_string(p).unwrap_or_default());
        conn.execute(
            "INSERT INTO rules (id, name, kind, field, pattern, replacement, template, params, enabled, description)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                name=excluded.name,
                kind=excluded.kind,
                field=excluded.field,
                pattern=excluded.pattern,
                replacement=excluded.replacement,
                template=excluded.template,
                params=excluded.params,
                enabled=excluded.enabled,
                description=excluded.description",
            params![
                rule.id,
                rule.name,
                rule.kind.to_string(),
                rule.field,
                rule.pattern,
                rule.replacement,
                template_json,
                params_json,
                rule.enabled as i64,
                rule.description,
            ],
        )?;
        Ok(())
    }

    /// 列出全部规则（按 name 升序，Unicode 码点排序）。
    pub fn list_rules(&self) -> Result<Vec<Rule>, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, name, kind, field, pattern, replacement, template, enabled, description, params
             FROM rules
             ORDER BY name ASC",
        )?;
        let rows = stmt.query_map([], row_to_rule)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 按 id 查找规则。
    pub fn get_rule(&self, id: &str) -> Result<Option<Rule>, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let rule = conn
            .query_row(
                "SELECT id, name, kind, field, pattern, replacement, template, enabled, description, params
                 FROM rules
                 WHERE id = ?1",
                params![id],
                row_to_rule,
            )
            .ok();
        Ok(rule)
    }

    /// 切换规则启用状态。不存在返回 `Ok(false)`。
    pub fn set_rule_enabled(&self, id: &str, enabled: bool) -> Result<bool, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let affected = conn.execute(
            "UPDATE rules SET enabled = ?1 WHERE id = ?2",
            params![enabled as i64, id],
        )?;
        Ok(affected > 0)
    }

    /// 统计规则数。
    pub fn count_rules(&self) -> Result<i64, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM rules", [], |r| r.get(0))?;
        Ok(count)
    }

    /// 更新规则可填参数（pattern / replacement）。`None` 表示该字段保持不变。
    /// 返回是否命中（id 存在）。
    pub fn update_rule_params(
        &self,
        id: &str,
        pattern: Option<&str>,
        replacement: Option<&str>,
    ) -> Result<bool, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut touched = false;
        if let Some(p) = pattern {
            let n = conn.execute(
                "UPDATE rules SET pattern = ?1 WHERE id = ?2",
                params![p, id],
            )?;
            touched |= n > 0;
        }
        if let Some(r) = replacement {
            let n = conn.execute(
                "UPDATE rules SET replacement = ?1 WHERE id = ?2",
                params![r, id],
            )?;
            touched |= n > 0;
        }
        if !touched {
            // 无字段要更新 → 仅返回存在性。
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM rules WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )?;
            return Ok(count > 0);
        }
        Ok(touched)
    }

    /// 更新规则的通用模板脱敏参数（`rules.template` 列）。v1.1.3 T49 新增。
    ///
    /// `template` 为 `Some(tpl)` → 序列化为 JSON 写入；`None` → 写 NULL（清空模板）。
    /// 返回是否命中（id 存在）。SQL 全部用 `?N` + `params![]` 绑定，禁止字符串拼接。
    pub fn update_rule_template(
        &self,
        id: &str,
        template: Option<&TemplateParams>,
    ) -> Result<bool, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let template_json: Option<String> =
            template.map(|t| serde_json::to_string(t).unwrap_or_default());
        let n = conn.execute(
            "UPDATE rules SET template = ?1 WHERE id = ?2",
            params![template_json, id],
        )?;
        if n > 0 {
            Ok(true)
        } else {
            // 未命中 → 仅返回存在性。
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM rules WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )?;
            Ok(count > 0)
        }
    }

    /// 更新提取规则的函数式校验参数（`rules.params` 列）+ 提取正则
    /// （`rules.pattern` 列）。v1.1.3 T55 新增。
    ///
    /// - `pattern` 为 `Some(p)` → 更新 pattern；`None` → 保持不变。
    /// - `params` 为 `Some(p)` → 序列化为 JSON 写入；`None` → 写 NULL（清空）。
    ///   传 `None` 仅清空 params；如需同时清空 pattern，请传 `Some("")`。
    ///
    /// 返回是否命中（id 存在）。SQL 全部用 `?N` + `params![]` 绑定，禁止字符串拼接。
    pub fn update_rule_extract_config(
        &self,
        id: &str,
        pattern: Option<&str>,
        params: Option<&ExtractParams>,
    ) -> Result<bool, DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        let mut touched = false;
        if let Some(p) = pattern {
            let n = conn.execute(
                "UPDATE rules SET pattern = ?1 WHERE id = ?2",
                params![p, id],
            )?;
            touched |= n > 0;
        }
        if params.is_some() {
            let params_json: Option<String> =
                params.map(|p| serde_json::to_string(p).unwrap_or_default());
            let n = conn.execute(
                "UPDATE rules SET params = ?1 WHERE id = ?2",
                params![params_json, id],
            )?;
            touched |= n > 0;
        }
        if !touched {
            // 未命中 → 仅返回存在性。
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM rules WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )?;
            return Ok(count > 0);
        }
        Ok(touched)
    }

    /// 启动时按 id upsert 缺失的内置规则。
    /// v1.1.3 T49 起改为"遍历内置规则集，对每条 id 不存在的规则 upsert"：
    /// 老用户升级时自动补 seed `simple-mask` / `segment-mask` 规则（4 条原独立
    /// 脱敏规则已收敛为预设，不再单独 seed）。已存在规则（含用户修改过参数的）不动。
    /// T50：upsert 之后清理 v1.1.3 T48 遗留的 4 条独立脱敏规则 id（`idcard-mask`/
    /// `phone-mask`/`birthdate-mask`/`bankcard-mask`），从用户 DB 中删除。
    /// T54：清理列表追加 `general-mask`（原单条通用脱敏规则拆分为 `simple-mask` +
    /// `segment-mask` 两条独立规则，旧 id 由本方法删除）。
    /// T55：补 seed 3 条提取规则（`phone-extract` / `bankcard-extract` / `ip-extract`），
    /// `with_defaults()` 共 8 条。
    /// T55b：拆分 `ip-extract` 为 `ip4-extract` + `ip6-extract` 两条独立规则，
    /// `with_defaults()` 共 9 条；旧 `ip-extract` 由 `cleanup_deprecated_rules` 删除。
    /// T55c：新增 `idcard-extract`（18 位身份证号 + 校验码 + 性别推断），
    /// `with_defaults()` 共 10 条。
    /// v1.1.4 T67：新增 6 条函数式校验规则（`username-validate` / `sex-validate` /
    /// `birth-validate` / `idcard-validate` / `phone-validate` / `address-validate`），
    /// `with_defaults()` 共 16 条。seed 幂等（只加行不加列），不改 SCHEMA_VERSION。
    /// v1.1.4 续轮 T70：新增 `generic-validate` 函数式校验规则（字符类白名单 +
    /// 长度范围），`with_defaults()` 共 17 条。
    pub fn seed_builtin_rules(&self) -> Result<(), DbError> {
        // 用 core 的 RuleRegistry::with_defaults() 拿到全部内置规则（v1.1.4 续轮
        // T70 起 17 条：3 name + simple-mask + segment-mask + 5 条 extract +
        // 6 条 T67 函数式校验 + 1 条 generic-validate）。
        let reg = RuleRegistry::with_defaults();
        for rule in reg.list() {
            if self.get_rule(&rule.id)?.is_none() {
                self.upsert_rule(rule)?;
            }
        }
        // v1.1.5：phone-extract 正则从 `\b1\d{10}\b` 升级为 `\b[1-9]\d{10}\b`
        // （放宽召回，兼容 7xx 等非标准前缀）。老 DB 存的是旧正则，需精确替换。
        // 用 WHERE pattern = ? 精确匹配旧值，避免覆盖用户自定义的其他正则。
        self.migrate_phone_extract_pattern()?;
        // T50：清理 v1.1.3 T48 遗留的 4 条独立脱敏规则（T49 收敛为预设）。
        // T54：追加 general-mask（拆分为 simple-mask + segment-mask 后废弃）。
        self.cleanup_deprecated_rules()?;
        Ok(())
    }

    /// v1.1.5 迁移：phone-extract 正则从 `\b1\d{10}\b` 升级为 `\b[1-9]\d{10}\b`。
    ///
    /// 老用户 DB 中 phone-extract 的 pattern 列可能存的是 v1.1.3 的旧正则
    /// `\b1\d{10}\b`（只匹配 1 开头），导致 7xx 等非标准前缀手机号全部漏召回。
    /// 本方法用 `WHERE pattern = ?` 精确匹配旧值后替换，不影响用户自定义的
    /// 其他正则（若用户已手动改过，不会被覆盖）。幂等：已是新正则则匹配 0 行。
    fn migrate_phone_extract_pattern(&self) -> Result<(), DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        conn.execute(
            "UPDATE rules SET pattern = ?1 WHERE id = 'phone-extract' AND pattern = ?2",
            params![r"\b[1-9]\d{10}\b", r"\b1\d{10}\b"],
        )?;
        Ok(())
    }

    /// 删除已废弃的内置规则 id（T50 + T54 + T55b）。用参数绑定，不拼接 SQL。
    ///
    /// v1.1.3 T48 曾落地的 4 条独立脱敏规则（`idcard-mask`/`phone-mask`/
    /// `birthdate-mask`/`bankcard-mask`）在 T49 收敛为预设，不再单独 seed。
    /// T54 原 `general-mask` 一条规则拆为 `simple-mask`（整段脱敏）+
    /// `segment-mask`（分段脱敏）两条独立规则，旧 `general-mask` id 废弃。
    /// T55b 原 `ip-extract` 拆为 `ip4-extract` + `ip6-extract` 两条独立规则，
    /// 旧 `ip-extract` id 废弃。
    /// 本方法在 `seed_builtin_rules` 末尾调用，从用户 DB 中删除这些遗留 id
    /// （幂等：id 不存在时 DELETE 影响 0 行，不报错）。
    fn cleanup_deprecated_rules(&self) -> Result<(), DbError> {
        let conn = self.conn.lock().expect("db mutex poisoned");
        for id in &[
            "idcard-mask",
            "phone-mask",
            "birthdate-mask",
            "bankcard-mask",
            "general-mask",
            "ip-extract",
        ] {
            conn.execute("DELETE FROM rules WHERE id = ?1", params![id])?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_support;

    #[test]
    fn rule_crud_upsert_list_get() {
        let (_dir, mgr) = test_support::open();
        assert_eq!(mgr.count_rules().unwrap(), 0);
        let r = RuleRegistry::name_validate_rule();
        mgr.upsert_rule(&r).unwrap();
        assert_eq!(mgr.count_rules().unwrap(), 1);
        // list
        let list = mgr.list_rules().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "name-validate");
        assert_eq!(list[0].kind, RuleKind::Validate);
        assert!(list[0].enabled);
        // get
        let got = mgr.get_rule("name-validate").unwrap().unwrap();
        assert_eq!(got.name, "姓名校验");
        assert!(mgr.get_rule("nope").unwrap().is_none());
    }

    #[test]
    fn rule_crud_upsert_overwrites_same_id() {
        let (_dir, mgr) = test_support::open();
        let mut r = RuleRegistry::name_validate_rule();
        mgr.upsert_rule(&r).unwrap();
        // 修改 pattern 后再 upsert → 覆盖
        r.pattern = Some(r"^[\u4e00-\u9fa5]{2,8}$".into());
        mgr.upsert_rule(&r).unwrap();
        assert_eq!(mgr.count_rules().unwrap(), 1);
        let got = mgr.get_rule("name-validate").unwrap().unwrap();
        assert_eq!(got.pattern.as_deref(), Some(r"^[\u4e00-\u9fa5]{2,8}$"));
    }

    #[test]
    fn rule_crud_toggle_enabled() {
        let (_dir, mgr) = test_support::open();
        mgr.upsert_rule(&RuleRegistry::name_validate_rule())
            .unwrap();
        assert!(mgr.get_rule("name-validate").unwrap().unwrap().enabled);
        assert!(mgr.set_rule_enabled("name-validate", false).unwrap());
        assert!(!mgr.get_rule("name-validate").unwrap().unwrap().enabled);
        assert!(!mgr.set_rule_enabled("nope", true).unwrap());
    }

    #[test]
    fn rule_crud_update_params() {
        let (_dir, mgr) = test_support::open();
        mgr.upsert_rule(&RuleRegistry::name_validate_rule())
            .unwrap();
        // 更新 pattern
        assert!(mgr
            .update_rule_params("name-validate", Some(r"^[\u4e00-\u9fa5]{2,8}$"), None)
            .unwrap());
        let got = mgr.get_rule("name-validate").unwrap().unwrap();
        assert_eq!(got.pattern.as_deref(), Some(r"^[\u4e00-\u9fa5]{2,8}$"));
        // 更新 replacement（mask 规则）
        mgr.upsert_rule(&RuleRegistry::name_mask_rule()).unwrap();
        assert!(mgr
            .update_rule_params("name-mask", None, Some("***"))
            .unwrap());
        let got2 = mgr.get_rule("name-mask").unwrap().unwrap();
        assert_eq!(got2.replacement.as_deref(), Some("***"));
        // 无字段更新 → 仅返回存在性
        assert!(mgr.update_rule_params("name-mask", None, None).unwrap());
        assert!(!mgr.update_rule_params("nope", None, None).unwrap());
    }

    #[test]
    fn seed_builtin_rules_inserts_ten_when_empty() {
        let (_dir, mgr) = test_support::open();
        assert_eq!(mgr.count_rules().unwrap(), 0);
        mgr.seed_builtin_rules().unwrap();
        // v1.1.4 续轮 T70：3 name + simple-mask + segment-mask + 5 条 extract
        // + 6 条 T67 validate（username/sex/birth/idcard/phone/address）
        // + 1 条 generic-validate
        // v1.1.5 T81：+ 1 条 email-validate = 18 条
        assert_eq!(mgr.count_rules().unwrap(), 18);
        let kinds: Vec<RuleKind> = mgr.list_rules().unwrap().iter().map(|r| r.kind).collect();
        assert!(kinds.contains(&RuleKind::Mask));
        assert!(kinds.contains(&RuleKind::Validate));
        assert!(kinds.contains(&RuleKind::Extract));
        // 再次 seed 不重复插入（已存在的 id 跳过）
        mgr.seed_builtin_rules().unwrap();
        assert_eq!(mgr.count_rules().unwrap(), 18);
    }

    #[test]
    fn seed_builtin_rules_preserves_user_param_modifications() {
        let (_dir, mgr) = test_support::open();
        mgr.seed_builtin_rules().unwrap();
        // 用户修改了 pattern
        mgr.update_rule_params("name-validate", Some(r"^[\u4e00-\u9fa5]{2,8}$"), None)
            .unwrap();
        // 再次 seed 不应覆盖用户修改
        mgr.seed_builtin_rules().unwrap();
        let got = mgr.get_rule("name-validate").unwrap().unwrap();
        assert_eq!(got.pattern.as_deref(), Some(r"^[\u4e00-\u9fa5]{2,8}$"));
    }

    #[test]
    fn seed_builtin_rules_upserts_missing_on_existing_db() {
        // v1.1.3 T55 语义：老用户已有 3 条 name 规则，再次 seed 应补 simple-mask
        // + segment-mask + phone-extract + bankcard-extract + ip4-extract
        // + ip6-extract + idcard-extract 七条规则（4 条原独立脱敏规则已收敛为预设，
        // 不再单独 seed；原 general-mask 拆分为 simple-mask + segment-mask；T55 新增
        // 3 条 extract 规则；T55b 拆 ip-extract 为 ip4/ip6 两条；T55c 新增
        // idcard-extract），且已存在规则参数不丢。
        let (_dir, mgr) = test_support::open();
        // 模拟 v1.1.2 老 DB：只 seed 3 条 name 规则。
        mgr.upsert_rule(&RuleRegistry::name_validate_rule())
            .unwrap();
        mgr.upsert_rule(&RuleRegistry::name_mask_rule()).unwrap();
        mgr.upsert_rule(&RuleRegistry::name_extract_rule()).unwrap();
        assert_eq!(mgr.count_rules().unwrap(), 3);
        // 用户修改 name-validate pattern
        mgr.update_rule_params("name-validate", Some(r"^[\u4e00-\u9fa5]{2,8}$"), None)
            .unwrap();
        // 再次 seed → 补 simple-mask + segment-mask + 5 条 extract 规则 + 6 条
        // T67 validate 规则 + 1 条 T70 generic-validate
        // v1.1.5 T81：+ 1 条 email-validate = 18 条
        mgr.seed_builtin_rules().unwrap();
        assert_eq!(mgr.count_rules().unwrap(), 18);
        // 用户修改的 pattern 仍在
        let got = mgr.get_rule("name-validate").unwrap().unwrap();
        assert_eq!(got.pattern.as_deref(), Some(r"^[\u4e00-\u9fa5]{2,8}$"));
        // 新规则已 seed（simple-mask 持空 Simple 模板）
        let sm = mgr.get_rule("simple-mask").unwrap().unwrap();
        assert!(sm.template.is_some());
        assert!(sm.template.as_ref().unwrap().is_empty());
        // segment-mask 持空 Segment 模板
        let segm = mgr.get_rule("segment-mask").unwrap().unwrap();
        assert!(segm.template.is_some());
        assert!(segm.template.as_ref().unwrap().is_empty());
        // T55：phone / bankcard extract 规则已 seed，且 params 已落库
        let phone = mgr.get_rule("phone-extract").unwrap().unwrap();
        assert_eq!(phone.kind, RuleKind::Extract);
        assert!(phone.params.is_some());
        let bank = mgr.get_rule("bankcard-extract").unwrap().unwrap();
        assert_eq!(bank.kind, RuleKind::Extract);
        assert!(bank.params.is_some());
        // T55b：ip4 / ip6 extract 规则已 seed，且 params 已落库
        let ip4 = mgr.get_rule("ip4-extract").unwrap().unwrap();
        assert_eq!(ip4.kind, RuleKind::Extract);
        assert!(ip4.params.is_some());
        let ip6 = mgr.get_rule("ip6-extract").unwrap().unwrap();
        assert_eq!(ip6.kind, RuleKind::Extract);
        assert!(ip6.params.is_some());
        // T55c：idcard-extract 规则已 seed，且 params 已落库
        let idcard = mgr.get_rule("idcard-extract").unwrap().unwrap();
        assert_eq!(idcard.kind, RuleKind::Extract);
        assert!(idcard.params.is_some());
        // 旧 id 不应出现（T49：4 条原独立规则已收敛为预设；T54：general-mask 已拆分；
        // T55b：ip-extract 已拆分为 ip4-extract + ip6-extract）
        assert!(mgr.get_rule("idcard-mask").unwrap().is_none());
        assert!(mgr.get_rule("phone-mask").unwrap().is_none());
        assert!(mgr.get_rule("birthdate-mask").unwrap().is_none());
        assert!(mgr.get_rule("bankcard-mask").unwrap().is_none());
        assert!(mgr.get_rule("general-mask").unwrap().is_none());
        assert!(mgr.get_rule("ip-extract").unwrap().is_none());
    }

    #[test]
    fn cleanup_deprecated_rules_removes_legacy_ids() {
        // T50 + T54 + T55b：模拟老 DB（含 4 条已废弃独立脱敏规则 + 1 条已拆分的
        // general-mask + 1 条已拆分的 ip-extract），seed 后应被清理。
        let (_dir, mgr) = test_support::open();
        // 手动 upsert 6 条废弃 id（模拟 T48 落地的老 DB + T49~T53 的 general-mask
        // + T55 的 ip-extract）
        for id in &[
            "idcard-mask",
            "phone-mask",
            "birthdate-mask",
            "bankcard-mask",
            "general-mask",
            "ip-extract",
        ] {
            let rule = Rule {
                id: (*id).into(),
                name: format!("遗留-{id}"),
                kind: RuleKind::Mask,
                field: None,
                pattern: None,
                replacement: None,
                enabled: true,
                description: String::new(),
                template: None,
                params: None,
            };
            mgr.upsert_rule(&rule).unwrap();
        }
        assert_eq!(mgr.count_rules().unwrap(), 6);
        // seed → 补 18 条内置规则 + 清理 6 条废弃 id = 18 条
        // （v1.1.5 T81：email-validate 新增，17 → 18）
        mgr.seed_builtin_rules().unwrap();
        assert_eq!(mgr.count_rules().unwrap(), 18);
        // 6 条废弃 id 已删除
        assert!(mgr.get_rule("idcard-mask").unwrap().is_none());
        assert!(mgr.get_rule("phone-mask").unwrap().is_none());
        assert!(mgr.get_rule("birthdate-mask").unwrap().is_none());
        assert!(mgr.get_rule("bankcard-mask").unwrap().is_none());
        assert!(mgr.get_rule("general-mask").unwrap().is_none());
        assert!(mgr.get_rule("ip-extract").unwrap().is_none());
        // 10 条内置规则仍在（3 name + simple-mask + segment-mask + 5 extract
        // + 6 条 T67 validate 规则也已被 seed + 1 条 T70 generic-validate）
        assert!(mgr.get_rule("name-validate").unwrap().is_some());
        assert!(mgr.get_rule("name-mask").unwrap().is_some());
        assert!(mgr.get_rule("name-extract").unwrap().is_some());
        assert!(mgr.get_rule("simple-mask").unwrap().is_some());
        assert!(mgr.get_rule("segment-mask").unwrap().is_some());
        assert!(mgr.get_rule("phone-extract").unwrap().is_some());
        assert!(mgr.get_rule("bankcard-extract").unwrap().is_some());
        assert!(mgr.get_rule("ip4-extract").unwrap().is_some());
        assert!(mgr.get_rule("ip6-extract").unwrap().is_some());
        // T55c：idcard-extract 是新规则，不在废弃列表，应存在
        assert!(mgr.get_rule("idcard-extract").unwrap().is_some());
        // T70：generic-validate 是新规则，应存在
        assert!(mgr.get_rule("generic-validate").unwrap().is_some());
    }

    #[test]
    fn update_rule_template_persists_and_reads_back() {
        // T49：update_rule_template 写入 template JSON，list_rules / get_rule 读回。
        let (_dir, mgr) = test_support::open();
        mgr.seed_builtin_rules().unwrap();
        // simple-mask 初始持空 Simple 模板
        let sm0 = mgr.get_rule("simple-mask").unwrap().unwrap();
        assert!(sm0.template.as_ref().unwrap().is_empty());
        // 更新为 idcard 预设
        let tpl = TemplateParams::new(6, 4, 8);
        assert!(mgr.update_rule_template("simple-mask", Some(&tpl)).unwrap());
        let sm1 = mgr.get_rule("simple-mask").unwrap().unwrap();
        let got = sm1.template.expect("template should be Some");
        // TemplateParams 是 untagged enum，idcard 预设走 Simple 变体
        let got = match got {
            TemplateParams::Simple(s) => s,
            _ => panic!("expected Simple variant"),
        };
        assert_eq!(got.keep_prefix, Some(6));
        assert_eq!(got.keep_suffix, Some(4));
        assert_eq!(got.mask_min_len, Some(8));
        // 清空模板 → 写 NULL
        assert!(mgr.update_rule_template("simple-mask", None).unwrap());
        let sm2 = mgr.get_rule("simple-mask").unwrap().unwrap();
        assert!(sm2.template.is_none());
        // 不存在的 id → false
        assert!(!mgr.update_rule_template("nope", Some(&tpl)).unwrap());
    }

    #[test]
    fn rule_kind_round_trip_through_db() {
        let (_dir, mgr) = test_support::open();
        for r in RuleRegistry::with_defaults().list() {
            mgr.upsert_rule(r).unwrap();
        }
        let list = mgr.list_rules().unwrap();
        // T55c 起 16 条内置规则（3 name + simple-mask + segment-mask + 5 extract
        // + 6 条 T67 validate：username/sex/birth/idcard/phone/address）。
        // v1.1.4 续轮 T70：+ generic-validate
        // v1.1.5 T81：+ email-validate = 18 条
        assert_eq!(list.len(), 18);
        for r in &list {
            // 确认 kind 字符串化 + 反序列化闭环
            let s = r.kind.to_string();
            assert_eq!(RuleKind::from_str_lowercase(&s), Some(r.kind));
        }
    }

    #[test]
    fn list_rules_sorted_by_name() {
        // T79：list_rules 按 name 升序（Unicode 码点）返回。
        // 插入顺序故意打乱：手机号提取（U+624B）→ IPv4地址提取（U+0049）→ 姓名校验（U+59D3）。
        // 期望返回顺序：IPv4 < 姓名 < 手机号。
        let (_dir, mgr) = test_support::open();
        let r1 = Rule {
            id: "phone-extract-test".into(),
            name: "手机号提取".into(),
            kind: RuleKind::Extract,
            field: None,
            pattern: Some(r"\d+".into()),
            replacement: None,
            template: None,
            enabled: true,
            description: String::new(),
            params: None,
        };
        let r2 = Rule {
            id: "ip4-extract-test".into(),
            name: "IPv4地址提取".into(),
            kind: RuleKind::Extract,
            field: None,
            pattern: Some(r"\d+".into()),
            replacement: None,
            template: None,
            enabled: true,
            description: String::new(),
            params: None,
        };
        let r3 = Rule {
            id: "name-validate-test".into(),
            name: "姓名校验".into(),
            kind: RuleKind::Validate,
            field: None,
            pattern: None,
            replacement: None,
            template: None,
            enabled: true,
            description: String::new(),
            params: None,
        };
        mgr.upsert_rule(&r1).unwrap();
        mgr.upsert_rule(&r2).unwrap();
        mgr.upsert_rule(&r3).unwrap();
        let list = mgr.list_rules().unwrap();
        assert_eq!(list.len(), 3);
        // Unicode 码点：I(U+0049) < 姓(U+59D3) < 手(U+624B)
        assert_eq!(list[0].name, "IPv4地址提取");
        assert_eq!(list[1].name, "姓名校验");
        assert_eq!(list[2].name, "手机号提取");
    }
}
