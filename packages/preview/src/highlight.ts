/** Subset of svg-pan-zoom this module needs. */
export interface HighlightPanZoom {
    getSizes(): { width: number; height: number; realZoom: number };
    getPan(): { x: number; y: number };
    pan(p: { x: number; y: number }): void;
}

export const PAN_ANIM_MS = 500;

/**
 * Translate-only animated pan that centres `el`'s bbox in the viewport.
 * svg-pan-zoom maps a user-coord point to viewport pixels as `pan + realZoom *
 * point`, so the pan that centres a bbox is `width/2 - centre * realZoom`.
 * Returns a `cancel()` so a new highlight can preempt the in-flight tween.
 */
export function panToElement(
    panZoom: HighlightPanZoom | null,
    el: Element,
): { cancel(): void } {
    if (!panZoom || typeof (el as SVGGraphicsElement).getBBox !== "function") {
        return { cancel() {} };
    }
    const bbox = (el as SVGGraphicsElement).getBBox();
    if (!bbox || (bbox.width === 0 && bbox.height === 0)) {
        return { cancel() {} };
    }
    const sizes = panZoom.getSizes();
    const realZoom = sizes.realZoom;
    const cx = bbox.x + bbox.width / 2;
    const cy = bbox.y + bbox.height / 2;
    const targetX = sizes.width / 2 - cx * realZoom;
    const targetY = sizes.height / 2 - cy * realZoom;
    const start = panZoom.getPan();
    const dx = targetX - start.x;
    const dy = targetY - start.y;
    if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) {
        panZoom.pan({ x: targetX, y: targetY });
        return { cancel() {} };
    }
    let raf: number | null = null;
    const startTime = performance.now();
    function step(now: number) {
        if (!panZoom) {
            raf = null;
            return;
        }
        const t = Math.min(1, (now - startTime) / PAN_ANIM_MS);
        const eased = 1 - Math.pow(1 - t, 3);
        panZoom.pan({ x: start.x + dx * eased, y: start.y + dy * eased });
        raf = t < 1 ? requestAnimationFrame(step) : null;
    }
    raf = requestAnimationFrame(step);
    return {
        cancel() {
            if (raf !== null) {
                cancelAnimationFrame(raf);
                raf = null;
            }
        },
    };
}

/** Selection-sync highlighting is stateless: strip prior matches then re-apply. */
export function clearHighlight(root: HTMLElement): void {
    root.querySelectorAll(".kul-selected").forEach((el) => {
        el.classList.remove("kul-selected");
    });
}

/**
 * Highlight one entity in `root` and animate-pan it to the centre. A null `id`
 * is clear-only. Returns the in-flight pan handle so the caller can cancel on
 * subsequent highlights / teardown.
 */
export function highlightEntity(
    root: HTMLElement,
    panZoom: HighlightPanZoom | null,
    ref: { id: string; kind: "person" | "marriage" } | null,
): { cancel(): void } {
    clearHighlight(root);
    if (!ref || !ref.id) {
        return { cancel() {} };
    }
    const selector =
        ref.kind === "marriage"
            ? '[data-link-kind="marriage"][data-marriage-id="' + ref.id + '"]'
            : '[data-person-id="' + ref.id + '"]';
    const el = root.querySelector(selector);
    if (!el) {
        return { cancel() {} };
    }
    el.classList.add("kul-selected");
    return panToElement(panZoom, el);
}
