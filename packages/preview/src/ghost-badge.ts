import { type HighlightPanZoom, PAN_ANIM_MS, panToElement } from "./highlight.js";

/**
 * ADR-0016: CSS cannot generate an SVG element, so the surface draws the
 * ghost ↺ badge. `createElementNS` is required — `document.createElement`
 * yields an inert HTML element that won't render inside `<svg>`. Each badge is
 * a `<g>` wrapping an invisible 24×24 hit-target rect, the visible glyph, and
 * a native `<title>` tooltip; the group carries a click handler that pans the
 * viewport to the person's canonical card and pulses it.
 */
export function injectGhostBadges(args: {
    svgRoot: SVGElement;
    root: HTMLElement;
    getPanZoom(): HighlightPanZoom | null;
}): void {
    const { svgRoot, root, getPanZoom } = args;
    const SVG_NS = "http://www.w3.org/2000/svg";
    const ghosts = svgRoot.querySelectorAll('.kul-card[data-kind="ghost"]');
    ghosts.forEach((card) => {
        const rect = card.querySelector("rect");
        if (!rect) {
            return;
        }
        const x = parseFloat(rect.getAttribute("x") ?? "");
        const y = parseFloat(rect.getAttribute("y") ?? "");
        const w = parseFloat(rect.getAttribute("width") ?? "");
        if (!isFinite(x) || !isFinite(y) || !isFinite(w)) {
            return;
        }
        const personId = card.getAttribute("data-person-id");
        const badge = document.createElementNS(SVG_NS, "g");
        badge.setAttribute("class", "kul-ghost-badge");
        const hit = document.createElementNS(SVG_NS, "rect");
        hit.setAttribute("x", String(x + w - 28));
        hit.setAttribute("y", String(y + 4));
        hit.setAttribute("width", "24");
        hit.setAttribute("height", "24");
        hit.setAttribute("fill", "transparent");
        hit.setAttribute("pointer-events", "all");
        badge.appendChild(hit);
        const glyph = document.createElementNS(SVG_NS, "text");
        glyph.setAttribute("x", String(x + w - 16));
        glyph.setAttribute("y", String(y + 20));
        glyph.setAttribute("text-anchor", "middle");
        glyph.textContent = "↺";
        badge.appendChild(glyph);
        const title = document.createElementNS(SVG_NS, "title");
        title.textContent = "Jump to canonical card";
        badge.appendChild(title);
        // Per-render listener; `stopPropagation` keeps the click from bubbling
        // to #root's click-to-source handler — jumping is viewport navigation
        // only, the editor cursor stays put.
        badge.addEventListener("click", (event) => {
            event.stopPropagation();
            if (!personId) {
                return;
            }
            const canonicalEl = root.querySelector(
                '[data-person-id="' + personId + '"][data-kind="canonical"]',
            );
            if (!canonicalEl) {
                return;
            }
            const canonical: Element = canonicalEl;
            panToElement(getPanZoom(), canonical);
            // Apply the pulse class after the pan tween settles so the glow
            // lands on the centred card, not on a sliding one.
            setTimeout(() => {
                function onEnd(): void {
                    canonical.classList.remove("kul-jump-target");
                    canonical.removeEventListener("animationend", onEnd);
                }
                canonical.addEventListener("animationend", onEnd);
                canonical.classList.add("kul-jump-target");
            }, PAN_ANIM_MS);
        });
        card.appendChild(badge);
    });
}
