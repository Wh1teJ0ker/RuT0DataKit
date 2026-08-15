import { CSS } from "@dnd-kit/utilities";
import { useSortable } from "@dnd-kit/sortable";

// 可拖拽列头单元格（@dnd-kit/sortable）。
// antd Table 通过 components.header.cell 注入；列 key 经 onHeaderCell 以 data-colkey 传入。
export function SortableHeaderCell({ colkey, style, ...rest }) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: colkey });

  const composedStyle = {
    ...style,
    transform: CSS.Translate.toString(transform),
    transition,
    cursor: "move",
    opacity: isDragging ? 0.5 : 1,
    background: isDragging ? "#fff" : style?.background,
    touchAction: "none",
  };

  return (
    <th
      ref={setNodeRef}
      style={composedStyle}
      {...attributes}
      {...listeners}
      {...rest}
    />
  );
}

// components.header.cell 包装：带 data-colkey 的走 SortableHeaderCell，否则普通 th。
export function HeaderCell({ "data-colkey": colkey, ...rest }) {
  if (colkey) return <SortableHeaderCell colkey={colkey} {...rest} />;
  return <th {...rest} />;
}
