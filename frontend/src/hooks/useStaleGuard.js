// 共享 hook：异步请求 generation-token 防陈旧响应。
//
// 用户快速连续翻页/搜索时，旧响应可能晚于新响应返回，导致 UI 闪现错误结果。
// 使用方式：
//   const { run, isStale } = useStaleGuard();
//   const gen = run();            // 自增 token，发起请求前调用
//   if (isStale(gen)) return;    // 响应返回后检查，token 不匹配则丢弃
import { useRef, useCallback } from "react";

export function useStaleGuard() {
  const genRef = useRef(0);

  const run = useCallback(() => {
    return ++genRef.current;
  }, []);

  const isStale = useCallback((gen) => {
    return genRef.current !== gen;
  }, []);

  return { run, isStale };
}
