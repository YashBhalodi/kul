// Bundled webview entry. esbuild emits dist/preview-webview.js (IIFE), which
// the VSCode extension's HTML shell loads via <script src>. Mounts the chrome
// inside `#kul-preview-mount` and wires the VSCode message bridge.

import { createVscodeAdapter, installVscodeInboundBridge } from "./adapter-vscode.js";
import { createQueryEngine } from "./engine.js";
import { ENGINE_MODULE_ATTR, ENGINE_WASM_ATTR, MOUNT_POINT_ID } from "./html.js";
import { mountPreview } from "./mount.js";

const mount = document.getElementById(MOUNT_POINT_ID);
if (mount) {
    // The engine's location comes off the shell, not off an import — the host
    // owns where the asset lives (ADR-0040). Constructing the engine here loads
    // nothing: the module is fetched on the first query.
    const moduleUri = mount.getAttribute(ENGINE_MODULE_ATTR);
    const wasmUri = mount.getAttribute(ENGINE_WASM_ATTR);
    const engine =
        moduleUri && wasmUri
            ? createQueryEngine({ moduleUri, wasmUri })
            : undefined;
    const adapter = createVscodeAdapter();
    const handle = mountPreview(mount, adapter, { engine });
    installVscodeInboundBridge(handle);
}
