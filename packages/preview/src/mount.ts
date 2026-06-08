import svgPanZoom from "svg-pan-zoom";

import { PREVIEW_BODY_HTML, mountKeyboardPan } from "./controls.js";
import { createErrorsController, setStaleSvg } from "./errors.js";
import { injectGhostBadges } from "./ghost-badge.js";
import { type HighlightPanZoom, highlightEntity } from "./highlight.js";
import { createLegendController } from "./legend.js";
import { mountHoverTooltip } from "./tooltip.js";
import type { EntityRef, ErrorRow, HostAdapter, PreviewHandle } from "./types.js";

/**
 * Mount the chrome inside `container`. The container is rewritten with the
 * scaffold (`#root` + controls + popover + legend siblings), then the runtime
 * wires hover / click / pan-zoom / keyboard / selection-sync / error-popover
 * against `adapter`. Returns the imperative {@link PreviewHandle}.
 */
export function mountPreview(
    container: HTMLElement,
    adapter: HostAdapter,
): PreviewHandle {
    container.innerHTML = PREVIEW_BODY_HTML;
    const root = container.querySelector("#root") as HTMLElement;
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

    let panZoom: ReturnType<typeof svgPanZoom> | null = null;
    let hasRender = false;
    let inFlightPan: { cancel(): void } | null = null;

    function cancelInFlightPan(): void {
        if (inFlightPan) {
            inFlightPan.cancel();
            inFlightPan = null;
        }
    }

    function panZoomForReader(): HighlightPanZoom | null {
        return panZoom as unknown as HighlightPanZoom | null;
    }

    const tooltip = mountHoverTooltip(root, () => (panZoom as unknown) as
        | { getSizes(): { realZoom: number } }
        | null);

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

    // Click-to-source. Birth/adoption edges also carry data-marriage-id, so
    // keying on data-link-kind="marriage" (not the bare attr) keeps them inert.
    root.addEventListener("click", (event) => {
        // Clicking a non-focusable SVG doesn't move focus off the text editor;
        // focus #root explicitly so the window keydown handler receives
        // arrows/+/-/0.
        root.focus();
        const target = event.target as Element | null;
        const person = target?.closest("[data-person-id]");
        if (person) {
            adapter.onRevealRequest({
                kind: "entity",
                id: person.getAttribute("data-person-id") ?? "",
            });
            return;
        }
        const marriage = target?.closest('[data-link-kind="marriage"]');
        if (marriage) {
            const id = marriage.getAttribute("data-marriage-id");
            if (id) {
                adapter.onRevealRequest({ kind: "entity", id });
            }
        }
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

    function render(svgString: string): void {
        // Drop the tooltip before its anchor SVG is swapped out.
        tooltip.close();
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
        injectGhostBadges({ svgRoot: svg, root, getPanZoom: panZoomForReader });
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
            // Any pan/zoom drops the tooltip so it never strands stale.
            onPan: () => tooltip.close(),
            onZoom: () => tooltip.close(),
        });
        if (savedZoom !== null && savedPan !== null) {
            panZoom.zoom(savedZoom);
            panZoom.pan(savedPan);
        }
        hasRender = true;
        reconcileControlsVisibility();
    }

    function showErrors(next: ErrorRow[]): void {
        // Issue #203 contract: do NOT wipe #root. Keep the last-good SVG
        // mounted (with its pan/zoom state and class), dim it via the
        // kul-render-stale class, and surface the errors through the popover.
        // First-open with errors → no SVG yet, panel stays empty; the error
        // button alone signals the failure.
        tooltip.close();
        setStaleSvg(root, true);
        errors.set(next);
    }

    function highlight(ref: EntityRef | null): void {
        cancelInFlightPan();
        inFlightPan = highlightEntity(root, panZoomForReader(), ref);
    }

    function dispose(): void {
        tooltip.close();
        teardownPanZoom();
        teardownKeyboard();
    }

    return {
        render,
        showErrors,
        highlightEntity: highlight,
        dispose,
    };
}
