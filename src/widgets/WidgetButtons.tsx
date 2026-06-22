import type { FC } from "react";

/** 关闭按钮 — 隐藏当前窗口 */
export const CloseButton: FC<{ onClick: () => void }> = ({ onClick }) => (
  <button className="btn btn-close" onClick={onClick} title="关闭">
    &#10005;
  </button>
);

/** 折叠/展开按钮 */
export const CollapseButton: FC<{ collapsed: boolean; onClick: () => void }> = ({
  collapsed,
  onClick,
}) => (
  <button
    className={`btn${collapsed ? " btn-collapsed" : ""}`}
    onClick={onClick}
    title={collapsed ? "展开" : "收起"}
  >
    &#9776;
  </button>
);

/** 吸附开关按钮 */
export const AttachButton: FC<{ enabled: boolean; onClick: () => void }> = ({
  enabled,
  onClick,
}) => (
  <button
    className={`btn btn-attach${enabled ? " btn-attach-on" : ""}`}
    onClick={onClick}
    title={enabled ? "停止吸附" : "开启吸附"}
  >
    &#128279;
  </button>
);

/** 记忆位置按钮 — 仅在吸附启用时显示 */
export const RememberButton: FC<{ active: boolean; onClick: () => void }> = ({
  active,
  onClick,
}) => (
  <button
    className={`btn btn-remember${active ? " btn-remember-on" : ""}`}
    onClick={onClick}
    title={active ? "跟随位置" : "记住位置"}
  >
    &#128204;
  </button>
);
