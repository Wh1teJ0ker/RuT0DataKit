// AppContext：Context + Provider + useAppContext hook（T13 引入）。
// 替代原 useAppState hook，让深层组件直接消费 state / dispatch / ACTION，
// 消除 prop drilling。
//
// v1.2.1 第三轮精简：原 24 个 useCallback dispatch forwarder 全部移除——
// dispatch 来自 useReducer 本身已稳定，各 forwarder 仅 `(payload) =>
// dispatch({type, payload})` 且 deps=[] 无额外 memo 价值。组件直接
// `dispatch({ type: ACTION.SET_VIEW, payload })` 即可。同时删除巨型 useMemo
// 依赖数组（原 26 项）。dead 导出 applySearchHits 也一并删除（无调用方）。
import { createContext, useContext, useMemo, useReducer } from "react";
import { ACTION, initialState } from "./constants";
import { reducer } from "./reducer";

export const AppContext = createContext(null);

export function AppProvider({ children }) {
  const [state, dispatch] = useReducer(reducer, initialState);

  const value = useMemo(() => ({ state, dispatch }), [state, dispatch]);

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
}

// 供深层组件消费。必须在组件顶层调用（Rules of Hooks）。
// 返回 { state, dispatch, ACTION }：组件用 `dispatch({ type: ACTION.X, payload })` 发 action。
export function useAppContext() {
  const ctx = useContext(AppContext);
  if (!ctx) {
    throw new Error("useAppContext must be used within <AppProvider>");
  }
  return ctx;
}
