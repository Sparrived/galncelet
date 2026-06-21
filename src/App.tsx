import { useState, useCallback, useMemo, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getPlugin } from "./addons/registry";
import { WidgetShell } from "./widgets/WidgetShell";
import { WidgetProvider } from "./lib/context";

// Dynamic plugin loading: auto-discover all addons/*/index.tsx
const pluginModules = import.meta.glob("./addons/*/index.tsx");

import ManagePage from "./pages/ManagePage";

function getWidgetType(): string {
  const params = new URLSearchParams(window.location.search);
  return params.get("widget") || getCurrentWindow().label.replace("widget-", "");
}

export default function App() {
  const widgetType = getWidgetType();

  const [repoRoot, setRepoRoot] = useState<string | null>(null);
  const [branch, setBranch] = useState<string | null>(null);
  const [pluginsLoaded, setPluginsLoaded] = useState(false);
  const refresh = useCallback(async () => {}, []);
  const showResult = useCallback((msg: string) => { console.log("[result]", msg); }, []);
  const showError = useCallback((msg: string) => { console.error("[error]", msg); }, []);
  const onStatusChange = useCallback((root: string | null, b: string | null) => {
    setRepoRoot(root);
    setBranch(b);
  }, []);

  // Dynamic plugin loading on mount
  useEffect(() => {
    const loadPlugins = async () => {
      const entries = Object.entries(pluginModules);
      await Promise.all(
        entries.map(async ([_path, loader]) => {
          try {
            await (loader as () => Promise<any>)();
          } catch (e) {
            console.error("[plugins] Failed to load plugin:", _path, e);
          }
        })
      );
      setPluginsLoaded(true);
    };
    loadPlugins();
  }, []);

  // Memoize context value — only changes when data actually changes
  const contextValue = useMemo(
    () => ({ repoRoot, branch, refresh, showResult, showError, onStatusChange }),
    [repoRoot, branch, refresh, showResult, showError, onStatusChange],
  );

  // Wait for plugins to load
  if (!pluginsLoaded) {
    return <div className="app"><div className="manage-empty">加载插件中…</div></div>;
  }

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
    <WidgetProvider value={contextValue}>
      <div className="app">
        <WidgetShell
          title={widgetTitle}
          showCloseButton={plugin.showCloseButton}
          showCollapseButton={plugin.showCollapseButton}
          showAttachButton={plugin.showAttachButton}
          defaultAttachEnabled={plugin.defaultAttachEnabled}
          defaultAttachRemember={plugin.defaultAttachRemember}
          defaultWhitelist={plugin.defaultWhitelist}
        >
          <Component />
        </WidgetShell>
      </div>
    </WidgetProvider>
  );
}
