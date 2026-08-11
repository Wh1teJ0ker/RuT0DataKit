import { Button, Empty, Layout } from "antd";
import MaskPanel from "../panels/MaskPanel";
import ValidatePanel from "../panels/ValidatePanel";
import ExtractPanel from "../panels/ExtractPanel";
import ColumnOpsPanel from "../panels/ColumnOpsPanel";
import CryptoPanel from "../panels/CryptoPanel";

const { Sider } = Layout;

// v1.1.0：`rules` 能力改在 Workbench 主区渲染两栏布局，不再走 260px SidePanel。
// v1.1.1：新增 `columnOps` 列操作面板。
// v1.1.2：新增 `crypto` 加解密面板（Base64 从 columnOps 迁入，后续扩展哈希/AES 等）。
// v1.1.4 T67：行级校验从独立能力合并入「校验」模块（ValidatePanel 内 Tabs 双页），
//   移除原 `rowValidate` 面板入口与 RowValidatePanel.jsx。
const PANELS = {
  mask: MaskPanel,
  validate: ValidatePanel,
  extract: ExtractPanel,
  columnOps: ColumnOpsPanel,
  crypto: CryptoPanel,
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
