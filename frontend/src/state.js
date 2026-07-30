import { useReducer, useCallback } from "react";

// v1.0.0 全局 state（useReducer 单一真相源）。
// 本任务（T2）仅落 activeCapability / currentView / aiPanel 三个字段；
// sheets / activeSheetId / selection / columnOrder / columnVisibility / updateStatus / sessions
// 等留给 T3/T6/T8 等下游任务扩展，此处不预声明（见 TASK-T2-HANDOFF out_of_scope）。

export const initialState = {
  currentView: "workbench", // 'workbench' | 'settings'
  activeCapability: null, // null | 'mask' | 'validate' | 'extract' | 'rules'
  aiPanel: { visible: true }, // lastSuggestion 由 T7 扩展
};

export const ACTION = {
  SET_VIEW: "SET_VIEW",
  SET_ACTIVE_CAPABILITY: "SET_ACTIVE_CAPABILITY",
  SET_AI_PANEL_VISIBLE: "SET_AI_PANEL_VISIBLE",
};

export function reducer(state, action) {
  switch (action.type) {
    case ACTION.SET_VIEW:
      return { ...state, currentView: action.payload };
    case ACTION.SET_ACTIVE_CAPABILITY: {
      // 再次点击同一能力按钮 → 取消激活（置 null）。
      const next =
        action.payload === state.activeCapability ? null : action.payload;
      return { ...state, activeCapability: next };
    }
    case ACTION.SET_AI_PANEL_VISIBLE:
      return {
        ...state,
        aiPanel: { ...state.aiPanel, visible: Boolean(action.payload) },
      };
    default:
      return state;
  }
}

export function useAppState() {
  const [state, dispatch] = useReducer(reducer, initialState);

  const setView = useCallback(
    (view) => dispatch({ type: ACTION.SET_VIEW, payload: view }),
    []
  );
  const setActiveCapability = useCallback(
    (capability) =>
      dispatch({ type: ACTION.SET_ACTIVE_CAPABILITY, payload: capability }),
    []
  );
  const setAiPanelVisible = useCallback(
    (visible) => dispatch({ type: ACTION.SET_AI_PANEL_VISIBLE, payload: visible }),
    []
  );

  return { state, dispatch, setView, setActiveCapability, setAiPanelVisible };
}
