// The query surface — the composition root for query chrome.
//
// It owns the one thing every later slice needs and nothing that any of them
// owns: the {@link SelectionStore}, the paint that shows it, the details panel
// it feeds, the panel-driven walk, and the editor-sync suspension that keeps
// the two meanings of "highlighted" from co-painting (#276 point 9).
//
// `mount.ts` reaches for six members and no more: `handleCanvasClick` (a click
// inside the rendered SVG), `repaintQueryChrome` (a render swapped the SVG),
// `syncHighlight` (an inbound editor highlight, which becomes the handle's
// `highlightEntity`), `refresh` (the locale toggle changed language),
// `occupiedBox` (the ghost badge's jump must steer around the panel) and
// `dispose`. Everything else about the selection is this module's business,
// which is what keeps the chrome's composition in one readable place rather
// than spread across the mount.

import type {
    DetailLookupResult,
    DetailTarget,
    EntityDetail,
    QueryEnvelope,
} from "./engine-wire.js";
import { isQueryOk } from "./engine-wire.js";
import { type DetailPanel, createDetailPanel } from "./detail-panel.js";
import {
    type HighlightPanZoom,
    type ScreenBox,
    canonicalCardFor,
    panToElement,
} from "./highlight.js";
import {
    type EntitySelection,
    type SelectionStore,
    createSelectionStore,
    paintSelection,
    selectionAtElement,
} from "./selection.js";
import type { EntityRef, HostAdapter } from "./types.js";

/** Text of the transient hint shown while editor sync is suspended. */
export const SYNC_SUSPENDED_HINT = "Editor sync paused · Esc to resume";

/**
 * Why editor sync is suspended. `"selection"` is this slice's reason; #303's
 * active filter is the other one #276 point 9 names, and it registers itself
 * through {@link QuerySurface.setSyncSuspended} rather than by widening any
 * condition here.
 */
export type SyncSuspensionReason = string;

export interface QuerySurfaceOptions {
    /** The rendered-SVG host (`#root`). Selection paint lands inside it. */
    root: HTMLElement;
    /**
     * The float layer's edge dock (`#kul-region-float-dock`) — the details
     * panel's home (ADR-0038, ADR-0042). The dock owns the edge and the inset.
     */
    floatDock: HTMLElement;
    /** Transient-notification region — the sync hint's home. */
    notifyRegion: HTMLElement | null;
    adapter: HostAdapter;
    /** The batched detail lookup, as the mounted preview exposes it (ADR-0037). */
    lookup(
        targets: DetailTarget[],
    ): Promise<QueryEnvelope<DetailLookupResult> | null>;
    getPanZoom(): HighlightPanZoom | null;
    /** Paint + centre an editor-sync highlight. Called only while sync is live. */
    applySyncHighlight(ref: EntityRef | null): void;
}

export interface QuerySurface {
    /**
     * The selection seam. Later slices read `current`, `subscribe` to changes,
     * and `select` to move it; they never reach past it to the DOM.
     */
    readonly selection: SelectionStore;
    /** A click landed inside the rendered SVG: an entity moves it, the canvas clears it. */
    handleCanvasClick(target: Element | null): void;
    /**
     * An inbound editor-sync highlight. Painted only while sync is live; while
     * it is suspended the ref is *held* and replayed the moment the last
     * suspension reason lifts, so resuming does not wait for the reader to
     * move the cursor again.
     */
    syncHighlight(ref: EntityRef | null): void;
    /**
     * Re-apply **every** piece of query paint after a render swapped the SVG
     * out. Today that is the selection outline; #301's kin results and #303's
     * filter dim land on the same replaced picture and repaint from inside
     * here.
     *
     * Deliberately not `repaintSelection()`: `mount.ts` gives query chrome one
     * post-render hook, and a name scoped to the selection would force the next
     * two slices to either contradict this docstring or add a second call to
     * the mount. Widening it while there is one implementation costs a line.
     */
    repaintQueryChrome(): void;
    /**
     * Drop the selection, its panel and its paint. Esc and a canvas click call
     * it, and it is what #304's render-ends-query-mode rule will call first.
     *
     * It is deliberately **not** named `clear()`: it clears one surface, and
     * `selection.clear()` is a documented no-op when nothing is selected, so a
     * general-sounding name would silently encode "query mode == a selection
     * exists" — false the moment #303's filter can be active on its own. #304
     * composes the real mode boundary from this call, the kin paint and the
     * filter (ADR-0042).
     */
    clearSelection(): void;
    /**
     * Register or lift one reason to suspend editor sync. The selection
     * registers `"selection"` itself; a later surface with its own reason
     * (#303's active filter) calls this rather than widening a condition, and
     * the hint follows the reason set rather than the selection.
     */
    setSyncSuspended(reason: SyncSuspensionReason, suspended: boolean): void;
    /**
     * Redraw the open panel from the answer it is already showing — no engine
     * call, no selection change. The lever for anything that changes how the
     * *same* detail reads: ADR-0041's locale toggle already subscribes, and
     * #301's kin rows inherit that wiring. A no-op when no panel is open.
     *
     * It rebuilds the panel element, so the panel's scroll position resets.
     * Harmless while the panel is short; worth revisiting once #301's kin list
     * makes it long enough to scroll.
     */
    refresh(): void;
    /** The panel's viewport box while it is open — the region a pan must avoid. */
    occupiedBox(): ScreenBox | null;
    dispose(): void;
}

export function createQuerySurface(options: QuerySurfaceOptions): QuerySurface {
    const {
        root,
        floatDock,
        notifyRegion,
        adapter,
        lookup,
        getPanZoom,
        applySyncHighlight,
    } = options;

    const selection = createSelectionStore();
    let inFlightPan: { cancel(): void } | null = null;
    // Every selection change invalidates any answer still in flight, so a slow
    // lookup can never open a panel about an entity that is no longer selected.
    let generation = 0;
    // The last thing the editor asked us to highlight. Held while sync is
    // suspended and replayed the moment it resumes.
    let heldSyncRef: EntityRef | null = null;
    // The answer the open panel is showing, kept so it can be redrawn without
    // asking the engine again.
    let shownDetail: EntityDetail | null = null;
    let syncHint: HTMLElement | null = null;
    const suspensions = new Set<SyncSuspensionReason>();

    const panel: DetailPanel = createDetailPanel({
        host: floatDock,
        onSelectPerson: walkToPerson,
        onReveal(id) {
            adapter.onRevealRequest({ kind: "entity", id });
        },
    });

    function closePanel(): void {
        shownDetail = null;
        panel.close();
    }

    function showPanel(detail: EntityDetail): void {
        shownDetail = detail;
        panel.show(detail);
    }

    function cancelPan(): void {
        if (inFlightPan) {
            inFlightPan.cancel();
            inFlightPan = null;
        }
    }

    function occupiedBox(): ScreenBox | null {
        const box = panel.box();
        if (!box) {
            return null;
        }
        // svg-pan-zoom's viewport is the rendered `<svg>`; translate the
        // panel's viewport box into that element's local pixels so the two
        // coordinate systems agree.
        const host = (root.querySelector("svg") ?? root) as Element;
        if (typeof host.getBoundingClientRect !== "function") {
            return null;
        }
        const base = host.getBoundingClientRect();
        return {
            left: box.left - base.left,
            right: box.right - base.left,
            top: box.top - base.top,
            bottom: box.bottom - base.top,
        };
    }

    /**
     * The panel's navigation path: move the selection onto a person named in
     * it, then centre that person's canonical card in the visible region. The
     * pan is not a flourish — the panel is how off-screen ancestors get
     * discovered, and a selection the reader cannot see is a dead end.
     */
    function walkToPerson(id: string): void {
        selection.select({ kind: "person", id });
        const card = canonicalCardFor(root, id);
        if (!card) {
            return;
        }
        cancelPan();
        inFlightPan = panToElement(getPanZoom(), card, occupiedBox());
    }

    function reconcileSyncHint(): void {
        const visible = suspensions.size > 0;
        if (!notifyRegion) {
            return;
        }
        if (!visible) {
            if (syncHint) {
                syncHint.remove();
                syncHint = null;
            }
            return;
        }
        if (syncHint) {
            return;
        }
        const hint = document.createElement("div");
        hint.className = "kul-sync-hint";
        hint.textContent = SYNC_SUSPENDED_HINT;
        notifyRegion.appendChild(hint);
        syncHint = hint;
    }

    async function loadPanel(
        current: EntitySelection,
        mine: number,
    ): Promise<void> {
        const envelope = await lookup([current]);
        if (mine !== generation) {
            return;
        }
        if (!envelope || !isQueryOk(envelope)) {
            closePanel();
            return;
        }
        const detail = envelope.result[0];
        if (!detail) {
            // A target naming no entity is a complete answer (ADR-0037), and an
            // empty panel would be a claim the engine did not make.
            closePanel();
            return;
        }
        showPanel(detail);
    }

    function setSyncSuspended(
        reason: SyncSuspensionReason,
        suspended: boolean,
    ): void {
        const was = suspensions.size > 0;
        if (suspended) {
            suspensions.add(reason);
        } else {
            suspensions.delete(reason);
        }
        const now = suspensions.size > 0;
        if (was === now) {
            return;
        }
        if (now) {
            // The takeover half of "the two meanings of highlighted never
            // co-paint": an editor-sync highlight is usually already on screen
            // when the reader clicks, because the cursor is usually on some
            // entity. Suspending has to *strip* it, not merely stop painting
            // new ones. `heldSyncRef` is untouched, so Esc still replays it.
            applySyncHighlight(null);
        } else {
            applySyncHighlight(heldSyncRef);
        }
        reconcileSyncHint();
    }

    selection.subscribe((current) => {
        generation += 1;
        paintSelection(root, current);
        if (!current) {
            closePanel();
            cancelPan();
        }
        // Tear down before resuming, so the replayed sync highlight is the last
        // thing to paint and the last thing to pan.
        setSyncSuspended("selection", current !== null);
        if (current) {
            void loadPanel(current, generation);
        }
    });

    function onKeyDown(event: KeyboardEvent): void {
        if (event.key === "Escape" && selection.current) {
            selection.clear();
        }
    }
    window.addEventListener("keydown", onKeyDown);

    return {
        selection,
        handleCanvasClick(target) {
            const next = selectionAtElement(target);
            if (next) {
                selection.select(next);
            } else {
                selection.clear();
            }
        },
        syncHighlight(ref) {
            heldSyncRef = ref;
            if (suspensions.size > 0) {
                // Held: the query selection owns the paint, and the two
                // meanings of "highlighted" never co-paint.
                return;
            }
            applySyncHighlight(ref);
        },
        repaintQueryChrome() {
            paintSelection(root, selection.current);
        },
        clearSelection() {
            selection.clear();
        },
        setSyncSuspended,
        refresh() {
            if (shownDetail) {
                panel.show(shownDetail);
            }
        },
        occupiedBox,
        dispose() {
            window.removeEventListener("keydown", onKeyDown);
            // Drop the held ref before clearing, so lifting the selection's
            // suspension replays nothing onto chrome being torn down.
            heldSyncRef = null;
            // Clearing runs the normal teardown path — paint off, panel shut,
            // pan cancelled, suspension lifted. A disposed surface stops
            // listening, so a violet outline left behind would have nothing
            // left to clear it.
            selection.clear();
            cancelPan();
            closePanel();
            paintSelection(root, null);
            suspensions.clear();
            reconcileSyncHint();
        },
    };
}
