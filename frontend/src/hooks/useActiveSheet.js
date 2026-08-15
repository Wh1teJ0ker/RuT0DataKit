// 共享 hook：从 AppContext state 中选取当前激活 Sheet + headers。
//
// 消除 8 处重复的 `state.sheets.find((s) => s.id === state.activeSheetId)` 查找。
// 组件调用 `const { sheet, headers } = useActiveSheet()` 即可。
import { useAppContext } from "../state";

export function useActiveSheet() {
  const { state } = useAppContext();
  const sheet = state.sheets.find((s) => s.id === state.activeSheetId);
  return { sheet, headers: sheet?.headers || [] };
}
