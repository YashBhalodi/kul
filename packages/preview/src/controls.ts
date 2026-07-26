/**
 * `currentColor` lets icons track the button's themed `color` via a single
 * `--kul-control-fg` token (ADR-0016).
 */
const ICON_ZOOM_IN = `<svg viewBox="0 0 16 16" aria-hidden="true" focusable="false"><path d="M8 3.5v9M3.5 8h9" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>`;
const ICON_ZOOM_OUT = `<svg viewBox="0 0 16 16" aria-hidden="true" focusable="false"><path d="M3.5 8h9" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>`;
const ICON_RESET = `<svg viewBox="0 0 16 16" aria-hidden="true" focusable="false"><path d="M3 6V3.5A.5.5 0 0 1 3.5 3H6M10 3h2.5a.5.5 0 0 1 .5.5V6M13 10v2.5a.5.5 0 0 1-.5.5H10M6 13H3.5a.5.5 0 0 1-.5-.5V10" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>`;
const ICON_INFO = `<svg viewBox="0 0 16 16" aria-hidden="true" focusable="false"><circle cx="8" cy="8" r="6.25" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="8" cy="4.75" r="0.85" fill="currentColor"/><path d="M8 7.25v4.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>`;
// Triangle-with-exclamation: the conventional severity glyph, themed via the
// dedicated `--kul-error-icon-color` token (#203).
const ICON_ERROR = `<svg viewBox="0 0 16 16" aria-hidden="true" focusable="false"><path d="M8 1.75 14.5 13.5h-13Z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><path d="M8 6.25v3.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/><circle cx="8" cy="11.75" r="0.85" fill="currentColor"/></svg>`;

/**
 * Overlay-region member (not a child of #root) so the per-render
 * `root.innerHTML = …` swap never wipes it. The pan/zoom + legend group hides
 * until the first successful render; the error button hides until at least one
 * error fires.
 */
export const CONTROLS_HTML = `<div id="kul-controls" class="kul-preview-controls" role="group" aria-label="Diagram view controls" hidden>
<div id="kul-controls-group" class="kul-controls-group" hidden>
<button type="button" class="kul-control-btn" data-action="zoom-in" title="Zoom in" aria-label="Zoom in">${ICON_ZOOM_IN}</button>
<button type="button" class="kul-control-btn" data-action="reset" title="Reset view" aria-label="Reset view">${ICON_RESET}</button>
<button type="button" class="kul-control-btn" data-action="zoom-out" title="Zoom out" aria-label="Zoom out">${ICON_ZOOM_OUT}</button>
<span class="kul-control-divider" aria-hidden="true"></span>
<button type="button" class="kul-control-btn" data-action="toggle-legend" title="Show legend" aria-label="Show legend" aria-pressed="false">${ICON_INFO}</button>
</div>
<button type="button" id="kul-error-button" class="kul-control-btn kul-error-button" data-action="toggle-errors" title="Show errors" aria-label="Show errors" aria-pressed="false" hidden>${ICON_ERROR}<span class="kul-error-count" aria-hidden="true">0</span></button>
</div>`;

/** Click-to-source popover the error button opens. Populated on first error. */
export const ERROR_POPOVER_HTML = `<div id="kul-error-popover" class="kul-error-popover" role="region" aria-label="Render errors" hidden></div>`;

/** ADR-0022: overlay-region member, populated from the rendered SVG on each render. */
export const LEGEND_HTML = `<div id="kul-legend" class="kul-preview-legend" role="region" aria-label="Diagram legend" hidden></div>`;

/**
 * The stage and its five regions (ADR-0036). Every piece of chrome belongs to
 * exactly one region, and the region — not the widget — owns placement and
 * stacking, so two panels can no longer be pinned to the same inset by two
 * independent rules.
 *
 * The overlay stack is ordered bottom-up by DOM order (`column-reverse`):
 * controls, then the error popover, then the legend. Opening a second panel
 * pushes the ones above it up instead of landing on top of them.
 *
 * `flow`, `float` and `notify` are declared empty. Later chrome joins one by
 * appending an element, not by inventing an inset.
 */
export const PREVIEW_BODY_HTML = `<div class="kul-stage">
<div id="kul-region-flow" class="kul-region-flow"></div>
<div class="kul-region-canvas"><div id="root" tabindex="-1" style="outline: none;"></div></div>
<div id="kul-region-overlay" class="kul-region-overlay">
${CONTROLS_HTML}
${ERROR_POPOVER_HTML}
${LEGEND_HTML}
</div>
<div id="kul-region-float" class="kul-region-float"></div>
<div id="kul-region-notify" class="kul-region-notify"></div>
</div>`;

/** Minimal svg-pan-zoom surface keyboard pan needs. */
export interface KeyboardPanZoom {
    panBy(p: { x: number; y: number }): void;
    zoomIn(): void;
    zoomOut(): void;
    reset(): void;
}

const PAN_SPEED = 12;

/**
 * Text-entry hosts whose keystrokes belong to the field, not to the canvas.
 * `closest` covers a keystroke that surfaces from a descendant of an editable
 * host rather than from the host itself.
 */
const TEXT_ENTRY_SELECTOR =
    'input, textarea, select, [contenteditable=""], [contenteditable="true"], [contenteditable="plaintext-only"]';

function isTextEntry(target: EventTarget | null): boolean {
    const el = target as Element | null;
    return typeof el?.closest === "function" && el.closest(TEXT_ENTRY_SELECTOR) !== null;
}

/**
 * Held arrows drive a rAF loop instead of one-shot per keydown, avoiding OS
 * key-repeat stutter. Modifier (ctrl/meta/alt) bails without preventDefault so
 * VSCode shortcuts like Cmd+0 still pass through. blur clears the held set.
 *
 * Keys bind on `window`, so a keystroke originating in a text field would
 * otherwise pan the canvas and swallow its own character. The target guard
 * keeps arrows and `+` / `-` / `0` out of any editable host.
 *
 * Returns a teardown that removes the listeners — the consumer rarely needs
 * it because pan/zoom is window-scoped, but `dispose()` calls it for hygiene.
 */
export function mountKeyboardPan(getPanZoom: () => KeyboardPanZoom | null): () => void {
    const heldPan = new Set<string>();
    let panRaf: number | null = null;
    function panFrame(): void {
        const pz = getPanZoom();
        if (!pz || heldPan.size === 0) {
            panRaf = null;
            return;
        }
        let dx = 0;
        let dy = 0;
        if (heldPan.has("ArrowDown")) {
            dy -= PAN_SPEED;
        }
        if (heldPan.has("ArrowUp")) {
            dy += PAN_SPEED;
        }
        if (heldPan.has("ArrowRight")) {
            dx -= PAN_SPEED;
        }
        if (heldPan.has("ArrowLeft")) {
            dx += PAN_SPEED;
        }
        if (dx !== 0 || dy !== 0) {
            pz.panBy({ x: dx, y: dy });
        }
        panRaf = requestAnimationFrame(panFrame);
    }
    function onKeyDown(event: KeyboardEvent): void {
        if (event.ctrlKey || event.metaKey || event.altKey) {
            return;
        }
        if (isTextEntry(event.target)) {
            return;
        }
        const pz = getPanZoom();
        if (!pz) {
            return;
        }
        switch (event.key) {
            case "ArrowDown":
            case "ArrowUp":
            case "ArrowRight":
            case "ArrowLeft":
                heldPan.add(event.key);
                if (panRaf === null) {
                    panRaf = requestAnimationFrame(panFrame);
                }
                break;
            case "+":
            case "=":
                pz.zoomIn();
                break;
            case "-":
                pz.zoomOut();
                break;
            case "0":
                pz.reset();
                break;
            default:
                return;
        }
        event.preventDefault();
    }
    function onKeyUp(event: KeyboardEvent): void {
        heldPan.delete(event.key);
    }
    function onBlur(): void {
        heldPan.clear();
    }
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);
    window.addEventListener("blur", onBlur);
    return () => {
        window.removeEventListener("keydown", onKeyDown);
        window.removeEventListener("keyup", onKeyUp);
        window.removeEventListener("blur", onBlur);
        if (panRaf !== null) {
            cancelAnimationFrame(panRaf);
            panRaf = null;
        }
    };
}
