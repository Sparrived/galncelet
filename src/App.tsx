import { useState, useCallback } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getPlugin } from "./submodule/registry";
import { WidgetShell } from "./widgets/WidgetShell";
import { WidgetProvider } from "./lib/context";

// Side-effect imports: each plugin self-registers on load
import "./submodule/git";
import "./submodule/amkr";

import ManagePage from "./pages/ManagePage";

function getWidgetType(): string {
  const params = new URLSearchParams(window.location.search);
  return params.get("widget") || getCurrentWindow().label.replace("widget-", "");
}

export default function App() {
  const widgetType = getWidgetType();

  const [repoRoot, setRepoRoot] = useState<string | null>(null);
  const [branch, setBranch] = useState<string | null>(null);
  const refresh = useCallback(async () => {}, []);
  const showResult = useCallback((msg: string) => { console.log("[result]", msg); }, []);
  const showError = useCallback((msg: string) => { console.error("[error]", msg); }, []);
  const onStatusChange = useCallback((root: string | null, b: string | null) => {
    setRepoRoot(root);
    setBranch(b);
  }, []);

  if (widgetType === "manage") {
    return <div className="app"><ManagePage /></div>;
  }

  const plugin = getPlugin(widgetType);

  if (!plugin) {
    return <div className="app"><div className="manage-empty">未知插件: {widgetType}</div></div>;
  }

  const Component = plugin.component;

  // Dynamic title: show repo name for Git plugin, plugin title for others
  const widgetTitle = plugin.id === "git"
    ? (repoRoot ? repoRoot.split(/[\\/]/).pop() : plugin.title)
    : plugin.title;

  return (
    <WidgetProvider value={{ repoRoot, branch, refresh, showResult, showError, onStatusChange }}>
      <div className="app">
        <WidgetShell
          title={widgetTitle}
          showCloseButton={plugin.showCloseButton}
          showCollapseButton={plugin.showCollapseButton}
          showAttachButton={plugin.showAttachButton}
          defaultAttachEnabled={plugin.defaultAttachEnabled}
          defaultWhitelist={plugin.defaultWhitelist}
        >
          <Component />
        </WidgetShell>
      </div>
    </WidgetProvider>
  );
}
