# Plugin Schema Reference

> Concise type definitions, interfaces, and APIs for building Galncelet plugins.
> Target audience: AI coding agents. No prose, just types and signatures.

## PluginDef Interface

```ts
// src/addons/registry.ts
interface PluginDef {
  id: string;                    // Unique ID, used as settings key & window label suffix
  title: string;                 // Display title in header & management page
  description?: string;          // Short description for management page
  icon?: string;                 // Emoji icon for management page
  collapsedHeight?: number;      // Height when collapsed (logical px). 0 = hide entirely
  defaultWidth?: number;         // Default window width (logical px)
  defaultHeight?: number;        // Default window height (logical px)
  showCloseButton?: boolean;     // Header close button (default: true)
  showCollapseButton?: boolean;  // Header collapse button (default: true)
  showAttachButton?: boolean;    // Header attach-to-foreground toggle (default: true)
  defaultAttachEnabled?: boolean;// Attach to foreground by default (default: true)
  defaultAttachRemember?: boolean;// "Remember position" mode default (default: false)
  defaultWhitelist?: string[];   // Window title substrings to restrict attachment
  component: FC;                 // React functional component (the widget content)
}
```

## Registry API

```ts
// src/addons/registry.ts
function registerPlugin(def: PluginDef): void;  // Call at module load time (side-effect import)
function getPlugin(id: string): PluginDef | undefined;
function getAllPlugins(): PluginDef[];
```

## WidgetContext (app-level, from src/lib/context.tsx)

```ts
interface WidgetContext {
  refresh: () => Promise<void>;           // Trigger full data refresh (currently no-op)
  showResult: (msg: string) => void;      // Show transient success message (console.log)
  showError: (msg: string) => void;       // Show transient error message (console.error)
  onStatusChange: (status: string | null) => void; // Update window subtitle
}
function useWidget(): WidgetContext;
```

## WidgetContextValue (shell-level, from src/widgets/WidgetContext.ts)

```ts
export const HEADER_H = 36;  // Widget header height (logical px)

interface ContextMenuItem {
  label: string;
  icon?: string;
  onClick: () => void;
  danger?: boolean;  // Red styling
}

interface WidgetContextValue {
  collapsed: boolean;
  contextMenuItems: ContextMenuItem[];
  registerContextMenuItems: (items: ContextMenuItem[]) => void;
}
function useWidgetContext(): WidgetContextValue;
function useWidgetContextMenu(items: ContextMenuItem[]): void; // Auto-unregisters on unmount
```

## Hooks

### useAutoResize

```ts
// src/widgets/useAutoResize.ts
function useAutoResize(containerRef: React.RefObject<HTMLElement | null>): void;
```

Auto-resizes the Tauri window to fit content. Uses `ResizeObserver` + `MutationObserver`. Adds `HEADER_H` (36px) to measured content height. Has 4px deadband. Uses `requestAnimationFrame` for throttling.

```tsx
const containerRef = useRef<HTMLDivElement>(null);
useAutoResize(containerRef);
return <div ref={containerRef}>...</div>;
```

### useWidget

```ts
// src/lib/context.tsx
const { refresh, showResult, showError, onStatusChange } = useWidget();
```

### useWidgetContextMenu

```ts
// src/widgets/WidgetContext.ts
useWidgetContextMenu([
  { label: "Refresh", icon: "🔄", onClick: () => refresh() },
  { label: "Clear", icon: "🗑️", danger: true, onClick: () => clear() },
]);
```

### useWidgetContext

```ts
// src/widgets/WidgetContext.ts
const { collapsed, contextMenuItems, registerContextMenuItems } = useWidgetContext();
```

## AppSettings & WindowState

```ts
// src/lib/types.ts
interface WindowState {
  x?: number;
  y?: number;
  height?: number;
  attachEnabled?: boolean;
  whitelist?: string[];
  attachRemember?: boolean;
}

interface AppSettings {
  refreshIntervalMs: number;    // default: 2000
  cardWidth: number;            // default: 360
  logMaxCount: number;          // default: 50
  alwaysOnTop: boolean;         // default: true
  pullRebase: boolean;          // default: true
  savedRepos: string[];
  currentRepo?: string;
  panelVisibility: Record<string, boolean>;
  windowStates: Record<string, WindowState>;
  hideFullscreen: boolean;      // default: true
  pluginHotkeys: Record<string, string>;
  widgetSequence: string[];
  sequenceHotkey: string | null;
}

export const DEFAULT_SETTINGS: AppSettings = {
  refreshIntervalMs: 2000,
  cardWidth: 360,
  logMaxCount: 50,
  alwaysOnTop: true,
  pullRebase: true,
  savedRepos: [],
  panelVisibility: {},
  windowStates: {},
  hideFullscreen: true,
  pluginHotkeys: {},
  widgetSequence: [],
  sequenceHotkey: null,
};
```

## WidgetShell Props

```ts
// src/widgets/WidgetShell.tsx
interface WidgetShellProps {
  title: ReactNode;
  children: ReactNode;
  headerRight?: ReactNode;
  showCloseButton?: boolean;      // default: true
  showCollapseButton?: boolean;   // default: true
  showAttachButton?: boolean;     // default: true
  defaultAttachEnabled?: boolean; // default: true
  defaultAttachRemember?: boolean;// default: false
  defaultWhitelist?: string[];    // default: []
  onClose?: () => void;
}
```

## Tauri IPC Bridge (frontend api.ts)

```ts
// src/lib/api.ts
function loadSettings(): Promise<AppSettings>;
function saveSettings(settings: AppSettings): Promise<void>;
function setPluginVisible(pluginId: string, visible: boolean): Promise<void>;
function updateCardWidth(width: number): Promise<void>;
function setBodyCollapsed(windowLabel: string, height: number | null, expandHeight?: number): Promise<void>;
function setAttachEnabled(windowLabel: string, enabled: boolean): Promise<void>;
function createPluginWindow(pluginId: string, title: string, width: number, height: number,
  defaultAttachEnabled?: boolean, defaultAttachRemember?: boolean, defaultWhitelist?: string[]): Promise<void>;
function openManageWindow(): Promise<void>;
function openSettingsWindow(): Promise<void>;
function openPluginSettings(pluginId: string): Promise<void>;
function saveWindowState(windowId: string, state: WindowState): Promise<void>;
function setAttachWhitelist(windowLabel: string, patterns: string[]): Promise<void>;
function setAttachRemember(windowLabel: string, remember: boolean): Promise<void>;
function setHideInFullscreen(enabled: boolean): Promise<void>;
function listVisibleWindows(): Promise<WindowEntry[]>;
function setPluginHotkey(pluginId: string, hotkey: string | null): Promise<void>;
function setWidgetSequence(sequence: string[]): Promise<void>;
function setSequenceHotkey(hotkey: string | null): Promise<void>;
```

## Shared Components

All in `src/components/`:

| Component | Props | Usage |
|-----------|-------|-------|
| `RadialGauge` | `value: number, label: string, color: string, sub: string, sub2?: string, sub2Color?: string` | Circular gauge chart |
| `AnimatedNumber` | `value: number, format?: (n: number) => string, duration?: number` | Number with counting animation |
| `ProgressBar` | `value: number, color: string, label?: string` | Horizontal progress bar |
| `StatCard` | `label: string, value: string, sub?: string` | Metric card with label/value/sub |
| `MetricRow` | `label: string, value: string` | Single-row metric display |
| `Toggle` | `checked: boolean, onChange: (v: boolean) => void, label?: string` | Toggle switch |
| `EmptyState` | `message: string` | Empty/loading state placeholder |

## Format Utilities

All in `src/lib/format.ts`:

| Function | Signature | Example |
|----------|-----------|---------|
| `fmtNumber` | `(n: number) => string` | `1234567 → "1.2M"` |
| `fmtMs` | `(ms: number) => string` | `1500 → "1.5s"` |
| `fmtBytes` | `(bytes: number) => string` | `1048576 → "1.0 MB"` |
| `fmtHz` | `(hz: number) => string` | `3600000000 → "3.60 GHz"` |
| `fmtPercent` | `(v: number) => string` | `0.85 → "85.0%"` |
| `fmtUptime` | `(startedAt: string) => string` | ISO timestamp → `"3h42m"` |

## CSS Variables (global theme)

```css
--glass-bg: rgba(18, 18, 24, 0.99)
--glass-border: rgba(255, 255, 255, 0.08)
--glass-highlight: rgba(255, 255, 255, 0.04)
--text-primary: #e4e4e7
--text-secondary: #a1a1aa
--text-muted: #71717a
--mcha-cyan: #22d3ee
--mcha-green: #4ade80
--mcha-amber: #fbbf24
--mcha-red: #f87171
--mcha-surface: rgba(255, 255, 255, 0.03)
--mcha-border: rgba(255, 255, 255, 0.06)
```

## manifest.json Schema

```jsonc
{
  "id": string,              // Required. Unique ID
  "title": string,           // Required. Display title
  "description": string,     // Optional. Management page description
  "icon": string,            // Optional. Emoji icon
  "defaultWidth": number,    // Optional. Logical px
  "defaultHeight": number,   // Optional. Logical px
  "collapsedHeight": number, // Optional. Logical px when collapsed. 0 = hide entirely
  "showCloseButton": bool,   // Optional. Default: true
  "showCollapseButton": bool,// Optional. Default: true
  "showAttachButton": bool,  // Optional. Default: true
  "defaultAttachEnabled": bool, // Optional. Default: true
  "defaultAttachRemember": bool, // Optional. Default: false
  "defaultWhitelist": string[]   // Optional. Default: []
}
```

## Rust Backend Plugin Contract

Every backend plugin module must implement:

```rust
// src-tauri/src/<plugin_id>/mod.rs
pub fn setup(app: &tauri::AppHandle);  // Called by _plugins::setup_all() at startup

// Optional: Tauri commands (must be registered in main.rs generate_handler!)
#[tauri::command]
pub fn my_command(...) -> Result<..., String>;
```

## build.rs Auto-Discovery Rules

Scans `src-tauri/src/` for directories containing `mod.rs`. Skips:
- `target`, `.git`, `.idea`, `.vscode`
- `acrylic`, `plugins`, `settings`, `tray`, `window_attach`

Generates `src/_plugins.rs` with:
- `#[path = "<id>/mod.rs"] pub mod <id>;` for each plugin
- `pub fn setup_all(app: &tauri::AppHandle) { <id>::setup(app); ... }`
- Embedded manifest JSON from `src/addons/*/manifest.json`

Manifests are loaded at runtime via `plugins::load_manifests()` → `embedded_plugin_manifests()`.
