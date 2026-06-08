// Bundled webview entry. esbuild emits dist/preview-webview.js (IIFE), which
// the VSCode extension's HTML shell loads via <script src>. Mounts the chrome
// inside `#kul-preview-mount` and wires the VSCode message bridge.

import { createVscodeAdapter, installVscodeInboundBridge } from "./adapter-vscode.js";
import { MOUNT_POINT_ID } from "./html.js";
import { mountPreview } from "./mount.js";

const mount = document.getElementById(MOUNT_POINT_ID);
if (mount) {
    const adapter = createVscodeAdapter();
    const handle = mountPreview(mount, adapter);
    installVscodeInboundBridge(handle);
}
