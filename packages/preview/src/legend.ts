export interface LegendRow {
    /** Stable key — surfaces on the row's `data-row` attribute. */
    key: string;
    /** Human label shown beside the swatch. English only (ADR-0022). */
    label: string;
    /** CSS selector tested against the rendered SVG; only present categories
     * yield a row. */
    presenceSelector: string;
}

/**
 * Normative legend table (ADR-0022). The chrome legend (here) and the CLI
 * baked legend (`crates/kul-svg/src/emit.rs`) share order + label strings;
 * `docs/canonical-ui-pattern.md` is the normative spec.
 */
export const LEGEND_ROWS: ReadonlyArray<LegendRow> = [
    {
        key: "gender-male",
        label: "Male",
        presenceSelector: '.kul-card[data-gender="male"]',
    },
    {
        key: "gender-female",
        label: "Female",
        presenceSelector: '.kul-card[data-gender="female"]',
    },
    {
        key: "gender-other",
        label: "Other",
        presenceSelector: '.kul-card[data-gender="other"]',
    },
    {
        key: "past-record",
        label: "Past record",
        presenceSelector: '.kul-card[data-kind="ghost"]',
    },
    {
        key: "birth",
        label: "Birth",
        presenceSelector: '.kul-edge[data-link-kind="birth"]',
    },
    {
        key: "adoption",
        label: "Adoption",
        presenceSelector: '.kul-edge[data-link-kind="adoption"]',
    },
    {
        key: "marriage",
        label: "Marriage",
        presenceSelector:
            '.kul-edge[data-link-kind="marriage"]:not([data-is-ended="true"])',
    },
    {
        key: "ended-marriage",
        label: "Ended marriage",
        presenceSelector:
            '.kul-edge[data-link-kind="marriage"][data-is-ended="true"]',
    },
];

/**
 * Filter {@link LEGEND_ROWS} to categories present in the diagram. Takes a
 * `querySelector`-like predicate so it stays DOM-free and unit testable.
 */
export function presentLegendRows(
    querySelector: (selector: string) => unknown,
): ReadonlyArray<LegendRow> {
    return LEGEND_ROWS.filter(
        (row) => querySelector(row.presenceSelector) != null,
    );
}

/**
 * Inline-SVG markup for one swatch: a miniature of the real glyph carrying the
 * production class + `data-*` attributes so the stylesheet themes it for free
 * (ADR-0022). Empty string for an unknown key.
 */
export function legendSwatchInnerSvg(key: string): string {
    switch (key) {
        case "gender-male":
        case "gender-female":
        case "gender-other": {
            const gender = key.substring("gender-".length);
            return (
                '<g class="kul-card" data-kind="canonical" data-gender="' +
                gender +
                '"><rect x="0.75" y="2" width="28.5" height="14" rx="3" ry="3"/></g>'
            );
        }
        case "past-record":
            return (
                '<g class="kul-card" data-kind="ghost">' +
                '<rect x="0.75" y="2" width="28.5" height="14" rx="3" ry="3" stroke-dasharray="3 2"/></g>'
            );
        case "birth":
            return '<path class="kul-edge" data-link-kind="birth" fill="none" d="M 0 9 L 30 9"/>';
        case "adoption":
            return '<path class="kul-edge" data-link-kind="adoption" fill="none" d="M 0 9 L 30 9" stroke-dasharray="6 4"/>';
        case "marriage":
            return '<path class="kul-edge" data-link-kind="marriage" fill="none" d="M 0 9 L 30 9"/>';
        case "ended-marriage":
            return '<path class="kul-edge" data-link-kind="marriage" data-is-ended="true" fill="none" d="M 0 9 L 30 9"/>';
        default:
            return "";
    }
}

export interface LegendController {
    /** Rebuild rows from the live SVG and reconcile visibility. */
    render(svgRoot: ParentNode): void;
    /** Clear rows and hide; preserves the user's open/closed toggle. */
    hide(): void;
    /** User-driven open/close. */
    toggle(): void;
    /** Re-apply visibility (used after content presence changes externally). */
    applyVisibility(): void;
}

/**
 * Two-step lifecycle (ADR-0022): `render(svg)` rebuilds the row list from the
 * SVG DOM on each render; `applyVisibility` reconciles `legend.hidden` from
 * the user's toggle AND content presence.
 */
export function createLegendController(
    legend: HTMLElement | null,
    queryToggle: () => HTMLElement | null,
): LegendController {
    let legendVisible = false;
    let legendHasContent = false;

    function applyVisibility(): void {
        if (!legend) {
            return;
        }
        const shouldShow = legendVisible && legendHasContent;
        legend.hidden = !shouldShow;
        const toggle = queryToggle();
        if (toggle) {
            toggle.setAttribute("aria-pressed", String(shouldShow));
            const labelText = shouldShow ? "Hide legend" : "Show legend";
            toggle.setAttribute("aria-label", labelText);
            toggle.setAttribute("title", labelText);
        }
    }

    function render(svgRoot: ParentNode): void {
        if (!legend) {
            return;
        }
        const present = LEGEND_ROWS.filter(
            (row) => svgRoot.querySelector(row.presenceSelector) !== null,
        );
        legendHasContent = present.length > 0;
        if (!legendHasContent) {
            legend.innerHTML = "";
        } else {
            legend.innerHTML = present
                .map(
                    (row) =>
                        '<div class="kul-legend-row" data-row="' +
                        row.key +
                        '">' +
                        '<svg class="kul-legend-swatch" viewBox="0 0 30 18" aria-hidden="true">' +
                        legendSwatchInnerSvg(row.key) +
                        "</svg>" +
                        '<span class="kul-legend-label">' +
                        row.label +
                        "</span>" +
                        "</div>",
                )
                .join("");
        }
        applyVisibility();
    }

    function hide(): void {
        if (legend) {
            legendHasContent = false;
            legend.innerHTML = "";
            legend.hidden = true;
        }
        applyVisibility();
    }

    function toggle(): void {
        legendVisible = !legendVisible;
        applyVisibility();
    }

    return { render, hide, toggle, applyVisibility };
}
