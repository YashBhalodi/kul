/** Subset of svg-pan-zoom this module needs. */
export interface HighlightPanZoom {
    getSizes(): { width: number; height: number; realZoom: number };
    getPan(): { x: number; y: number };
    pan(p: { x: number; y: number }): void;
}

export const PAN_ANIM_MS = 500;

/** A rectangle in viewport pixels. */
export interface ScreenBox {
    left: number;
    top: number;
    right: number;
    bottom: number;
}

/**
 * Where a centred entity should land, given a panel floating over the canvas.
 *
 * The details panel overlays rather than docks (ADR-0036), so the raw viewport
 * centre can park a card behind it — and the panel is precisely how off-screen
 * ancestors get discovered, which makes an invisible selection a dead end
 * (ADR-0035). Centring therefore targets the **visible** region.
 *
 * `occluder` is in viewport-local pixels (origin at the pan/zoom viewport's
 * top-left). The free strip on each side of it is measured and the wider one
 * wins, so nothing here assumes which edge the panel sits on. Only the
 * horizontal axis moves: the panel is a full-height side panel, so the vertical
 * centre is never the crowded one, and shifting it would fight the reader's
 * expectation that a centred card is vertically centred.
 */
export function visibleCentre(
    viewport: { width: number; height: number },
    occluder: ScreenBox | null,
): { x: number; y: number } {
    const full = { x: viewport.width / 2, y: viewport.height / 2 };
    if (!occluder) {
        return full;
    }
    const left = Math.max(0, Math.min(viewport.width, occluder.left));
    const right = Math.max(0, Math.min(viewport.width, occluder.right));
    if (right <= left) {
        return full;
    }
    const roomLeft = left;
    const roomRight = viewport.width - right;
    if (roomLeft <= 0 && roomRight <= 0) {
        // The panel spans the whole viewport: there is no visible region to
        // centre in, so the raw centre is the honest answer.
        return full;
    }
    return roomLeft >= roomRight
        ? { x: roomLeft / 2, y: full.y }
        : { x: right + roomRight / 2, y: full.y };
}

/**
 * The person's canonical card in `root`, or `null`. Ghost cards are excluded:
 * a person owns one canonical card and any number of ghosts, and only the
 * canonical one is a place to be taken to.
 */
export function canonicalCardFor(
    root: ParentNode,
    personId: string,
): Element | null {
    return (
        Array.from(root.querySelectorAll('[data-person-id][data-kind="canonical"]')).find(
            (card) => card.getAttribute("data-person-id") === personId,
        ) ?? null
    );
}

/**
 * Translate-only animated pan that centres `el`'s bbox in the visible region.
 * svg-pan-zoom maps a user-coord point to viewport pixels as `pan + realZoom *
 * point`, so the pan that centres a bbox is `centre - point * realZoom`, where
 * the centre accounts for any `occluder` floating over the canvas.
 * Returns a `cancel()` so a new highlight can preempt the in-flight tween.
 */
export function panToElement(
    panZoom: HighlightPanZoom | null,
    el: Element,
    occluder: ScreenBox | null = null,
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
    const centre = visibleCentre(sizes, occluder);
    const targetX = centre.x - cx * realZoom;
    const targetY = centre.y - cy * realZoom;
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
