import { useEffect, useState } from "react";
import { listRules } from "../tauri";

/**
 * useRules — 加载规则列表并按 kind 过滤。
 *
 * 供 MaskPanel / ValidatePanel / ExtractPanel / RulesPanel 共享，消除各面板
 * 各自 `useEffect(() => { listRules() → filter }, [])` 的重复 boilerplate。
 *
 * @param {string|null} kindFilter — `"mask"` / `"validate"` / `"extract"` / `null`（全部）
 * @param {object} [opts] — `{ patternOnly?: boolean }`：仅保留有 pattern 的规则（提取面板用）
 * @returns {{ rules: array, loading: boolean, reload: () => Promise<void> }}
 *
 * v1.2.0 T94：从各面板 useEffect 中抽出共享 hook。
 */
export function useRules(kindFilter = null, opts = {}) {
  const { patternOnly = false } = opts;
  const [rules, setRules] = useState([]);
  const [loading, setLoading] = useState(false);

  const reload = async () => {
    setLoading(true);
    try {
      const all = await listRules();
      let filtered = kindFilter ? all.filter((r) => r.kind === kindFilter) : all;
      if (patternOnly) filtered = filtered.filter((r) => r.pattern);
      setRules(filtered);
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("useRules load failed:", e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    let alive = true;
    (async () => {
      setLoading(true);
      try {
        const all = await listRules();
        if (!alive) return;
        let filtered = kindFilter ? all.filter((r) => r.kind === kindFilter) : all;
        if (patternOnly) filtered = filtered.filter((r) => r.pattern);
        setRules(filtered);
      } catch (e) {
        // eslint-disable-next-line no-console
        console.error("useRules load failed:", e);
      } finally {
        if (alive) setLoading(false);
      }
    })();
    return () => {
      alive = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [kindFilter, patternOnly]);

  return { rules, loading, reload };
}
