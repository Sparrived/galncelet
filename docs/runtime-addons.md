# Runtime Addons

Galncelet now supports runtime addons that are installed by copying a folder into the user addons directory. These addons do not require rebuilding Galncelet.

## Install and remove

Runtime addons live in:

```text
%APPDATA%\Galncelet\addons\<addon-id>\
```

From the Galncelet management window:

- click the puzzle button to open the addons directory;
- copy an addon folder into that directory;
- Galncelet watches the directory and refreshes the addon list automatically;
- the refresh button is still available for manual reloads;
- delete an addon folder to remove it from the list and close its runtime window.

Enabled/visible state is still stored in Galncelet settings using the addon `id`.

## Package layout

A frontend-only addon:

```text
my-addon/
  manifest.json
  ui/
    index.html
    main.js
    styles.css
```

An addon with an external backend:

```text
my-addon/
  manifest.json
  ui/
    index.html
    main.js
    styles.css
  backend/
    my-addon.exe
```

## manifest.json

```json
{
  "id": "my-addon",
  "title": "My Addon",
  "description": "A runtime addon loaded from the user addons folder.",
  "icon": "🧩",
  "entry": "ui/index.html",
  "defaultWidth": 360,
  "defaultHeight": 240,
  "showCloseButton": true,
  "showCollapseButton": true,
  "showAttachButton": true,
  "defaultAttachEnabled": false,
  "defaultAttachRemember": false,
  "defaultWhitelist": [],
  "permissions": [],
  "backend": {
    "type": "sidecar",
    "command": "backend/my-addon.exe",
    "protocol": "jsonrpc"
  }
}
```

Required fields:

- `id`: lowercase letters, digits, `-`, or `_` only; stable across releases.
- `title`: display name in the management page and window title.

Important fields:

- `entry`: relative path to the addon HTML entry. Defaults to `ui/index.html`.
- `backend`: optional sidecar backend. Galncelet currently supports `type: "sidecar"` and `protocol: "jsonrpc"`.
- `permissions`: declared for documentation and future enforcement. The current security boundary is the fixed host command set and addon-folder path sandbox.

Paths must stay inside the addon folder. Absolute paths and `..` traversal are rejected.

## Frontend API

Runtime addon pages are loaded as local files in their own Tauri WebView. Because `withGlobalTauri` is enabled, frontend code can call Galncelet commands through the global Tauri API:

```html
<script>
async function callBackend() {
  const result = await window.__TAURI__.core.invoke("invoke_runtime_addon", {
    addonId: "my-addon",
    method: "ping",
    params: { message: "hello" }
  });
  document.body.textContent = JSON.stringify(result);
}
callBackend();
</script>
```

The stable host commands for runtime addons are:

- `list_runtime_addons`
- `get_runtime_addons_dir`
- `open_runtime_addons_dir`
- `create_runtime_addon_window`
- `invoke_runtime_addon`
- `runtime_addon_storage_get`
- `runtime_addon_storage_set`
- `runtime_addon_storage_delete`

### Addon storage

Each addon gets isolated JSON key/value storage under Galncelet's app data directory. Keys may contain letters, digits, `-`, `_`, and `.` only.

```js
await window.__TAURI__.core.invoke("runtime_addon_storage_set", {
  addonId: "my-addon",
  key: "settings",
  value: { theme: "dark" }
});

const settings = await window.__TAURI__.core.invoke("runtime_addon_storage_get", {
  addonId: "my-addon",
  key: "settings"
});

await window.__TAURI__.core.invoke("runtime_addon_storage_delete", {
  addonId: "my-addon",
  key: "settings"
});
```

## Sidecar JSON-RPC protocol

When `invoke_runtime_addon` is called, Galncelet starts the addon sidecar, writes one JSON-RPC request to stdin, waits for stdout, and returns the `result` field.

Request example sent to stdin:

```json
{"jsonrpc":"2.0","id":1,"method":"ping","params":{"message":"hello"}}
```

Successful response on stdout:

```json
{"jsonrpc":"2.0","id":1,"result":{"reply":"pong"}}
```

Error response on stdout:

```json
{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"failed"}}
```

The current runtime uses one process per invocation and hides the sidecar console window on Windows. Addons that need richer native capabilities should ship a sidecar instead of requiring Galncelet to add new Tauri commands.

## Minimal example

`manifest.json`:

```json
{
  "id": "hello-runtime",
  "title": "Hello Runtime",
  "entry": "ui/index.html",
  "defaultWidth": 300,
  "defaultHeight": 160
}
```

`ui/index.html`:

```html
<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <style>
      body { margin: 0; padding: 16px; color: white; background: rgba(20,20,24,.85); font-family: sans-serif; }
    </style>
  </head>
  <body>
    <h3>Hello runtime addon</h3>
    <p>This addon was loaded from the user addons directory.</p>
  </body>
</html>
```

## Legacy source addons

The existing `src/addons/*` React addons and `src-tauri/src/*` Rust modules are still supported as built-in addons. They are compiled into Galncelet and are useful for core features. New third-party extensions should prefer runtime addons so users can install, update, or delete them without rebuilding the main application.
