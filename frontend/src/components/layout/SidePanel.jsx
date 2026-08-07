import { Button, Empty, Layout } from "antd";
import MaskPanel from "../panels/MaskPanel";
import ValidatePanel from "../panels/ValidatePanel";
import ExtractPanel from "../panels/ExtractPanel";

const { Sider } = Layout;

// v1.1.0：`rules` 能力改在 Workbench 主区渲染两栏布局，不再走 260px SidePanel。
const PANELS = {
  mask: MaskPanel,
  validate: ValidatePanel,
  extract: ExtractPanel,
};

// 左侧动态能力面板容器：由 activeCapability 驱动切换面板；
// activeCapability === null 显示空态。「⚙ 设置」入口已移至右侧 AiPanel。
// v1.1.0：`rules` 不在此列（改由 App.jsx 路由到主区 RulesPanel）。
export default function SidePanel({ activeCapability }) {
  const PanelComp = activeCapability ? PANELS[activeCapability] : null;

  return (
    <Sider
      width={260}
      theme="light"
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        overflow: "auto",
        borderRight: "1px solid #f0f0f0",
      }}
    >
      <div style={{ flex: 1, padding: 8 }}>
        {PanelComp ? (
          <PanelComp />
        ) : (
          <Empty
            style={{ marginTop: 48 }}
            description="点击上方能力按钮展开面板"
          />
        )}
      </div>
    </Sider>
  );
}
