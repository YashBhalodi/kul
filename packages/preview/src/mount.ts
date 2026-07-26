import svgPanZoom from "svg-pan-zoom";

import { PREVIEW_BODY_HTML, mountKeyboardPan } from "./controls.js";
import type {
    DetailTarget,
    ExportedDiagnostic,
    Query,
    QueryEnvelope,
} from "./engine-wire.js";
import { isQueryOk } from "./engine-wire.js";
import type { ProjectSnapshot, QueryEngine } from "./engine.js";
import { createErrorsController, setStaleSvg } from "./errors.js";
import { injectGhostBadges } from "./ghost-badge.js";
import { type HighlightPanZoom, highlightEntity } from "./highlight.js";
import { createLegendController } from "./legend.js";
import { createLocaleController } from "./locale.js";
import type { LocaleStore } from "./locale-store.js";
import { createQuerySurface } from "./query-surface.js";
import type { EntityRef, ErrorRow, HostAdapter, PreviewHandle } from "./types.js";

/** Optional collaborators a host can supply to {@link mountPreview}. */
export interface MountOptions {
    /**
     * The query engine this preview asks. Omitted by hosts that ship no engine
     * asset (and by tests that do not query): {@link PreviewHandle.queryDetail}
     * then resolves `null` instead of loading anything.
     */
    engine?: QueryEngine;
    /**
     * Where the reader's language choice persists. Defaults to in-memory, so
     * a plain embedding needs nothing; the VSCode entry hands in
     * `createVscodeLocaleStore()` and the choice then survives a reopen.
     */
    localeStore?: LocaleStore;
}

/**
 * A query's diagnostics become popover rows. Only error-severity rows surface:
 * the popover is the error surface, and ADR-0009 guarantees the error arm
 * carries at least one. `ExportedDiagnostic` anchors to a byte span in a bare
 * file name, not to a document URI, so the rows carry no location and are not
 * click-to-source — unlike the render diagnostics the host already anchors.
 */
function diagnosticsToErrorRows(diagnostics: ExportedDiagnostic[]): ErrorRow[] {
    return diagnostics
        .filter((d) => d.severity === "error")
        .map((d) => ({ message: d.message, code: d.code }));
}

/**
 * Mount the chrome inside `container`. The container is rewritten with the
 * stage and its regions (ADR-0036) — `#root` in the canvas region, the locale
 * toggle in the flow region, controls / error popover / legend in the overlay
 * stack, the details panel in the float layer's dock, the hover lens's pill
 * anchored on the float layer itself and the sync hint in the notify region —
 * then the runtime wires click / hover / pan-zoom / keyboard / selection /
 * selection-sync / error-popover against `adapter`. Returns the imperative
 * {@link PreviewHandle}.
 */
export function mountPreview(
    container: HTMLElement,
    adapter: HostAdapter,
    options: MountOptions = {},
): PreviewHandle {
    const engine = options.engine ?? null;
    container.innerHTML = PREVIEW_BODY_HTML;
    const root = container.querySelector("#root") as HTMLElement;
    const floatDock = container.querySelector(
        "#kul-region-float-dock",
    ) as HTMLElement;
    const floatLayer = container.querySelector(
        "#kul-region-float",
    ) as HTMLElement;
    const controls = container.querySelector("#kul-controls") as HTMLElement | null;
    const controlsGroup = container.querySelector(
        "#kul-controls-group",
    ) as HTMLElement | null;
    const errorButton = container.querySelector(
        "#kul-error-button",
    ) as HTMLElement | null;
    const errorPopover = container.querySelector(
        "#kul-error-popover",
    ) as HTMLElement | null;
    const legend = container.querySelector("#kul-legend") as HTMLElement | null;
    const flowRegion = container.querySelector(
        "#kul-region-flow",
    ) as HTMLElement | null;
    const notifyRegion = container.querySelector(
        "#kul-region-notify",
    ) as HTMLElement | null;

    // Standing chrome: the toggle is live before the first render, because it
    // is a reading preference rather than a view control.
    const locale = createLocaleController(
        container.querySelector("#kul-locale") as HTMLElement | null,
        options.localeStore,
    );

    let panZoom: ReturnType<typeof svgPanZoom> | null = null;
    let hasRender = false;
    let inFlightPan: { cancel(): void } | null = null;
    // The source the current picture came from. Replaced by every render, so a
    // query can never answer about a project the reader is no longer looking at.
    let project: ProjectSnapshot | null = null;

    function cancelInFlightPan(): void {
        if (inFlightPan) {
            inFlightPan.cancel();
            inFlightPan = null;
        }
    }

    function panZoomForReader(): HighlightPanZoom | null {
        return panZoom as unknown as HighlightPanZoom | null;
    }

    const errors = createErrorsController({
        errorButton,
        errorPopover,
        adapter,
        onReconcile: reconcileControlsVisibility,
    });

    const legendCtl = createLegendController(legend, () =>
        container.querySelector(
            'button[data-action="toggle-legend"]',
        ) as HTMLElement | null,
    );

    function reconcileControlsVisibility(): void {
        if (controlsGroup) {
            controlsGroup.hidden = !hasRender;
        }
        if (errorButton) {
            errorButton.hidden = errors.length === 0;
        }
        if (controls) {
            controls.hidden = !hasRender && errors.length === 0;
        }
    }

    // Every piece of query chrome lives behind this one surface: the store
    // later slices build on, the paint, the details panel, the panel-driven
    // walk, the Explore-kin list, the hover lens, the filter bar, and the
    // editor-sync suspension.
    const querySurface = createQuerySurface({
        root,
        floatDock,
        floatLayer,
        flowRegion,
        notifyRegion,
        adapter,
        lookup: queryDetail,
        runKinQuery: queryKin,
        runQuery: runFilterQuery,
        resolve: queryResolve,
        locale,
        getPanZoom: panZoomForReader,
        applySyncHighlight,
    });

    // A locale change re-reads what is already on screen (ADR-0041), and the
    // open panel is on screen. Today's panel labels are chrome and stay English
    // (#276 point 6), so this redraws identical words — it is wired now so the
    // slice that puts phrased rows in the panel adds none of this plumbing.
    const unsubscribeLocale = locale.subscribe(() => querySurface.refresh());

    // Click selects (ADR-0035). Click-to-source is gone from the tree —
    // revealing is now the panel header's explicit control, so walking the
    // family no longer drags the editor's scroll position along with it.
    root.addEventListener("click", (event) => {
        // Clicking a non-focusable SVG doesn't move focus off the text editor;
        // focus #root explicitly so the window keydown handler receives
        // arrows/+/-/0.
        root.focus();
        querySurface.handleCanvasClick(event.target as Element | null);
    });

    // The hover lens (#302). Pointer *movement* is its whole input — no click,
    // no mode — and the surface debounces, so this handler stays a cheap
    // hit-test at pointer-move frequency.
    root.addEventListener("pointermove", (event) => {
        querySurface.handleCanvasHover(event.target as Element | null);
    });
    // Leaving the canvas passes what the pointer moved *onto*, so moving onto
    // the lens's own pill to read a term does not dismiss the pill.
    root.addEventListener("pointerleave", (event) => {
        querySurface.handleCanvasHover(event.relatedTarget as Element | null);
    });

    if (controls) {
        controls.addEventListener("click", (event) => {
            const btn = (event.target as Element | null)?.closest?.(
                "button[data-action]",
            );
            if (!btn) {
                return;
            }
            const action = btn.getAttribute("data-action");
            if (action === "toggle-legend") {
                legendCtl.toggle();
                return;
            }
            if (action === "toggle-errors") {
                errors.toggle();
                return;
            }
            if (!panZoom) {
                return;
            }
            if (action === "zoom-in") {
                panZoom.zoomIn();
            } else if (action === "zoom-out") {
                panZoom.zoomOut();
            } else if (action === "reset") {
                panZoom.reset();
            }
        });
    }

    const teardownKeyboard = mountKeyboardPan(() =>
        panZoom as unknown as {
            panBy(p: { x: number; y: number }): void;
            zoomIn(): void;
            zoomOut(): void;
            reset(): void;
        } | null,
    );

    function teardownPanZoom(): void {
        cancelInFlightPan();
        if (panZoom) {
            panZoom.destroy();
            panZoom = null;
        }
    }

    function render(
        svgString: string,
        nextProject: ProjectSnapshot | null = null,
    ): void {
        project = nextProject;
        let savedPan: { x: number; y: number } | null = null;
        let savedZoom: number | null = null;
        if (panZoom) {
            cancelInFlightPan();
            savedPan = panZoom.getPan();
            savedZoom = panZoom.getZoom();
            panZoom.destroy();
            panZoom = null;
        }
        root.innerHTML = svgString;
        const svg = root.querySelector("svg") as SVGSVGElement | null;
        // A successful render is its own cue that the prior error state is
        // gone, so any pending errors clear here (#203).
        errors.set([]);
        if (!svg) {
            legendCtl.hide();
            reconcileControlsVisibility();
            return;
        }
        injectGhostBadges({
            svgRoot: svg,
            root,
            getPanZoom: panZoomForReader,
            getOccluder: querySurface.occupiedBox,
        });
        legendCtl.render(svg);
        panZoom = svgPanZoom(svg, {
            zoomEnabled: true,
            panEnabled: true,
            controlIconsEnabled: false,
            fit: true,
            center: true,
            minZoom: 0.25,
            maxZoom: 20,
            zoomScaleSensitivity: 0.3,
            dblClickZoomEnabled: true,
            mouseWheelZoomEnabled: true,
        });
        if (savedZoom !== null && savedPan !== null) {
            panZoom.zoom(savedZoom);
            panZoom.pan(savedPan);
        }
        // The SVG every piece of query paint was on has just been replaced —
        // and a render is how an edit reaches this webview. **An edit ends
        // query mode** (ADR-0035, ADR-0046): the selection, its panel, any kin
        // paint, the lens and the filter all let go here, the tree returns to
        // plain, and editor sync resumes on its own. This is the *only*
        // post-render hook query chrome gets, so the whole rule lives behind
        // this one call rather than as a second call bolted in here.
        querySurface.endQueryMode();
        hasRender = true;
        reconcileControlsVisibility();
    }

    function showErrors(next: ErrorRow[]): void {
        // Issue #203 contract: do NOT wipe #root. Keep the last-good SVG
        // mounted (with its pan/zoom state and class), dim it via the
        // kul-render-stale class, and surface the errors through the popover.
        // First-open with errors → no SVG yet, panel stays empty; the error
        // button alone signals the failure.
        setStaleSvg(root, true);
        errors.set(next);
    }

    /**
     * Run one engine operation against the project the current picture came
     * from, routing both failure modes to the error popover: a transport
     * failure (the module is a fetched asset) and a project that failed its
     * checks (ADR-0009 yields the error arm, never a partial answer).
     *
     * Shared by all three query entry points below, so they can never disagree
     * about where a failure surfaces.
     */
    async function runQuery<T>(
        run: (
            engine: QueryEngine,
            snapshot: ProjectSnapshot,
        ) => Promise<QueryEnvelope<T>>,
    ): Promise<QueryEnvelope<T> | null> {
        if (!engine || !project) {
            return null;
        }
        let envelope: QueryEnvelope<T>;
        try {
            envelope = await run(engine, project);
        } catch (err) {
            const detail = err instanceof Error ? err.message : String(err);
            showErrors([{ message: `Kul query failed: ${detail}` }]);
            return null;
        }
        if (!isQueryOk(envelope)) {
            showErrors(diagnosticsToErrorRows(envelope.diagnostics));
        }
        return envelope;
    }

    function queryDetail(targets: DetailTarget[]) {
        return runQuery((it, snapshot) => it.queryDetail(snapshot, targets));
    }

    function queryKin(query: Query) {
        return runQuery((it, snapshot) => it.queryKin(snapshot, query));
    }

    function runFilterQuery(query: Query) {
        return runQuery((it, snapshot) => it.runQuery(snapshot, query));
    }

    function queryResolve(xId: string, yId: string) {
        return runQuery((it, snapshot) => it.queryResolve(snapshot, xId, yId));
    }

    function applySyncHighlight(ref: EntityRef | null): void {
        cancelInFlightPan();
        inFlightPan = highlightEntity(root, panZoomForReader(), ref);
    }

    function dispose(): void {
        unsubscribeLocale();
        querySurface.dispose();
        teardownPanZoom();
        teardownKeyboard();
    }

    return {
        render,
        showErrors,
        highlightEntity: querySurface.syncHighlight,
        queryDetail,
        queryKin,
        runQuery: runFilterQuery,
        queryResolve,
        locale,
        dispose,
    };
}
