export interface TooltipModel {
    /** Entity-kind kicker: "Person" / "Marriage" / "Adoption". */
    title: string;
    /** Display name (person), "A & B" (marriage), or child name (adoption);
     * empty when no name could be resolved. */
    identity: string;
    rows: Array<{ label: string; value: string }>;
}

/**
 * Single source of truth for tooltip content. Used by both the runtime hover
 * state machine in {@link mountHoverTooltip} and direct Vitest assertions.
 *
 * ADR-0021 / #156: every Person/Marriage/Adoption property rides the SVG as a
 * `data-*` attribute, so the tooltip reads them directly — no LSP round-trip.
 * Rows use a denylist of structural attrs so new display fields surface
 * automatically. Returns `null` for birth edges (purely structural) and
 * anything that is not a person card or marriage/adoption edge.
 */
export function buildTooltip(
    attrs: ReadonlyArray<{ name: string; value: string }>,
    resolveName: (id: string) => string,
): TooltipModel | null {
    const DENYLIST = new Set([
        "data-person-id",
        "data-marriage-id",
        "data-host-id",
        "data-joining-id",
        "data-child-id",
        "data-kind",
        "data-link-kind",
        "data-generation",
        "data-ghost-reason",
        "data-is-alive",
        "data-is-ended",
        "data-is-past",
    ]);
    const LABEL_OVERRIDES: Record<string, string> = {
        family: "Family name",
        given: "Given name",
    };
    function get(name: string): string {
        for (const attr of attrs) {
            if (attr.name === name) {
                return attr.value;
            }
        }
        return "";
    }
    function has(name: string): boolean {
        for (const attr of attrs) {
            if (attr.name === name) {
                return true;
            }
        }
        return false;
    }
    function displayValue(raw: string): string {
        for (let i = 0; i < raw.length; i++) {
            const ch = raw[i];
            if (ch.toLowerCase() !== ch.toUpperCase()) {
                return raw.slice(0, i) + ch.toUpperCase() + raw.slice(i + 1);
            }
        }
        return raw;
    }

    let title: string;
    let identity: string;
    if (has("data-person-id")) {
        title = "Person";
        identity = resolveName(get("data-person-id"));
    } else {
        const linkKind = get("data-link-kind");
        if (linkKind === "marriage") {
            title = "Marriage";
            const host = resolveName(get("data-host-id"));
            const joining = resolveName(get("data-joining-id"));
            identity = host && joining ? host + " & " + joining : host || joining;
        } else if (linkKind === "adoption") {
            title = "Adoption";
            identity = resolveName(get("data-child-id"));
        } else {
            return null;
        }
    }

    const rows: Array<{ label: string; value: string }> = [];
    for (const attr of attrs) {
        const name = attr.name;
        const value = attr.value;
        if (name.indexOf("data-") !== 0) {
            continue;
        }
        if (DENYLIST.has(name) || value === "") {
            continue;
        }
        const key = name.slice("data-".length);
        let label = LABEL_OVERRIDES[key];
        if (!label) {
            const spaced = key.replace(/-/g, " ");
            label = spaced.charAt(0).toUpperCase() + spaced.slice(1);
        }
        rows.push({ label, value: displayValue(value) });
    }
    return { title, identity, rows };
}

/** Minimal svg-pan-zoom surface the tooltip needs to scale itself. */
export interface TooltipPanZoomReader {
    getSizes(): { realZoom: number };
}

/** Hover-intent delay tracks VSCode's editor-hover feel. */
const HOVER_DELAY_MS = 350;
const MAX_TOOLTIP_SCALE = 1.5;

/**
 * Wire the floating-tooltip state machine onto `root`. Caller supplies a
 * `getPanZoom()` accessor so the tooltip tracks the live zoom even after
 * pan/zoom destroy/recreate cycles. Returns a manual close handle the caller
 * invokes on render / pan / zoom to ensure no stale tooltip survives.
 */
export function mountHoverTooltip(
    root: HTMLElement,
    getPanZoom: () => TooltipPanZoomReader | null,
): { close(): void } {
    let hoverTarget: Element | null = null;
    let hoverTimer: ReturnType<typeof setTimeout> | null = null;
    let tooltipEl: HTMLElement | null = null;

    function close(): void {
        if (hoverTimer !== null) {
            clearTimeout(hoverTimer);
            hoverTimer = null;
        }
        if (tooltipEl) {
            tooltipEl.remove();
            tooltipEl = null;
        }
        hoverTarget = null;
    }

    function tooltipScale(): number {
        const pz = getPanZoom();
        const sizes = pz && pz.getSizes ? pz.getSizes() : null;
        const z = sizes && sizes.realZoom > 0 ? sizes.realZoom : 1;
        return z < MAX_TOOLTIP_SCALE ? z : MAX_TOOLTIP_SCALE;
    }

    function positionTooltip(target: Element): void {
        if (!tooltipEl || typeof (target as HTMLElement).getBoundingClientRect !== "function") {
            return;
        }
        const anchor = (target as HTMLElement).getBoundingClientRect();
        const tip = tooltipEl.getBoundingClientRect();
        const GAP = 8 * tooltipScale();
        const MARGIN = 4;
        let left = anchor.left;
        let top = anchor.bottom + GAP;
        if (left + tip.width > window.innerWidth - MARGIN) {
            left = window.innerWidth - MARGIN - tip.width;
        }
        if (left < MARGIN) {
            left = MARGIN;
        }
        if (top + tip.height > window.innerHeight - MARGIN) {
            top = anchor.top - GAP - tip.height;
        }
        if (top < MARGIN) {
            top = MARGIN;
        }
        tooltipEl.style.left = left + "px";
        tooltipEl.style.top = top + "px";
    }

    function resolveName(id: string): string {
        if (!id) {
            return "";
        }
        const card = root.querySelector('[data-person-id="' + id + '"]');
        if (!card) {
            return id;
        }
        const label = card.querySelector(".kul-label-name");
        return label && label.textContent ? label.textContent : id;
    }

    function showTooltip(target: Element): void {
        const attrs: Array<{ name: string; value: string }> = [];
        for (let i = 0; i < target.attributes.length; i++) {
            const a = target.attributes[i];
            attrs.push({ name: a.name, value: a.value });
        }
        const model = buildTooltip(attrs, resolveName);
        if (!model) {
            return;
        }
        const el = document.createElement("div");
        el.className = "kul-tooltip";
        el.setAttribute("role", "tooltip");
        el.style.transformOrigin = "top left";
        el.style.transform = "scale(" + tooltipScale() + ")";
        const header = document.createElement("div");
        header.className = "kul-tooltip-header";
        const kind = document.createElement("span");
        kind.className = "kul-tooltip-kind";
        kind.textContent = model.title;
        header.appendChild(kind);
        if (model.identity) {
            const name = document.createElement("span");
            name.className = "kul-tooltip-title";
            name.textContent = model.identity;
            header.appendChild(name);
        }
        el.appendChild(header);
        if (model.rows.length) {
            const fields = document.createElement("div");
            fields.className = "kul-tooltip-fields";
            model.rows.forEach((row) => {
                const label = document.createElement("span");
                label.className = "kul-tooltip-label";
                label.textContent = row.label;
                const value = document.createElement("span");
                value.className = "kul-tooltip-value";
                value.textContent = row.value;
                fields.appendChild(label);
                fields.appendChild(value);
            });
            el.appendChild(fields);
        }
        document.body.appendChild(el);
        tooltipEl = el;
        positionTooltip(target);
    }

    root.addEventListener("mouseover", (event) => {
        const entity = (event.target as Element | null)?.closest?.(
            ".kul-card, .kul-edge",
        );
        if (entity === hoverTarget) {
            return;
        }
        close();
        if (entity) {
            hoverTarget = entity;
            hoverTimer = setTimeout(() => {
                hoverTimer = null;
                showTooltip(entity);
            }, HOVER_DELAY_MS);
        }
    });
    root.addEventListener("mouseout", (event) => {
        const to = event.relatedTarget as Node | null;
        if (hoverTarget && (!to || !hoverTarget.contains(to))) {
            close();
        }
    });

    return { close };
}
