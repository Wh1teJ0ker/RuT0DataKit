import { Grid } from "antd";

const { useBreakpoint: antdUseBreakpoint } = Grid;

// v1.2.0 T98：断点感知 hook，封装 antd Grid.useBreakpoint()。
// 返回各断点布尔值 + isCompact 便捷标志（窄于 md = true）。
// antd 默认断点：xs<576, sm≥576, md≥768, lg≥992, xl≥1200, xxl≥1600
//
// 用法：
//   const { isCompact } = useBreakpoint();
//   // isCompact === true 时，窗口宽度 < 768px，按窄屏渲染
export function useBreakpoint() {
  const screens = antdUseBreakpoint();
  const isMd = !!screens.md;
  return {
    isXs: !!screens.xs,
    isSm: !!screens.sm,
    isMd,
    isLg: !!screens.lg,
    isXl: !!screens.xl,
    isXxl: !!screens.xxl,
    // 窄于 md（< 768px）= 紧凑模式
    isCompact: !isMd,
    screens,
  };
}
