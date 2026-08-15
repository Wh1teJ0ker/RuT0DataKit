import { buildGenericParamsForRun } from "../validateParams";

// 组装后端契约：每条 { column, ruleId, crossField?, paramsOverride? }。
// crossField 仅在 idcard-validate 且勾选了性别/出生比对时传，否则传 null。
// paramsOverride 仅在 generic-validate / birth-validate 时携带，覆盖 DB 默认 params
// （字符类 + 长度限制 / 生日格式），其他规则传 null（后端忽略）。
export function buildMultiRules(ruleRows) {
  return ruleRows
    .filter((r) => r?.column && r?.ruleId)
    .map((r) => {
      const item = {
        column: r.column,
        ruleId: r.ruleId,
        crossField:
          r.ruleId === "idcard-validate" && (r.checkSex || r.checkBirth)
            ? {
                checkSex: !!r.checkSex,
                sexColumn: r.sexColumn || null,
                checkBirth: !!r.checkBirth,
                birthColumn: r.birthColumn || null,
              }
            : null,
        paramsOverride: null,
      };
      // v1.1.4 续轮 T71：generic-validate 行附带 paramsOverride。
      // T77：特殊符号从布尔全开/全关改为自定义白名单字符串。
      if (r.ruleId === "generic-validate") {
        const classes = r.charClasses || [];
        item.paramsOverride = buildGenericParamsForRun({
          allowDigits: classes.includes("digits"),
          allowLetters: classes.includes("letters"),
          allowSpecialChars: r.specialChars || "",
          minLen: r.minLen ?? null,
          maxLen: r.maxLen ?? null,
        });
      }
      // v1.1.5 T88：birth-validate 行附带 paramsOverride，仅勾选了格式时携带。
      // 全不选 = 接受所有格式（默认，向后兼容），不传 paramsOverride，沿用 DB 默认。
      if (r.ruleId === "birth-validate" && r.birthFormats?.length) {
        item.paramsOverride = { validator: "birth", formats: r.birthFormats };
      }
      return item;
    });
}
