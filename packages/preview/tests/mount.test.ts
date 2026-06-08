import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// svg-pan-zoom relies on SVG measurement APIs jsdom does not implement. Stub
// it out before any module imports the dep so the chrome can mount cleanly.
vi.mock("svg-pan-zoom", () => {
    const make = () => {
        let pan = { x: 0, y: 0 };
        let zoom = 1;
        return {
            getPan: () => pan,
            getZoom: () => zoom,
            getSizes: () => ({ width: 800, height: 600, realZoom: 1 }),
            pan: (p: { x: number; y: number }) => {
                pan = p;
            },
            panBy: (p: { x: number; y: number }) => {
                pan = { x: pan.x + p.x, y: pan.y + p.y };
            },
            zoom: (z: number) => {
                zoom = z;
            },
            zoomIn: vi.fn(),
            zoomOut: vi.fn(),
            reset: vi.fn(),
            destroy: vi.fn(),
        };
    };
    const factory = vi.fn(() => make());
    return { default: factory };
});

import svgPanZoom from "svg-pan-zoom";
import type { HostAdapter, PreviewHandle, RevealTarget } from "../src/types.js";
import { mountPreview } from "../src/mount.js";

function makeAdapter(): { adapter: HostAdapter; reveals: RevealTarget[] } {
    const reveals: RevealTarget[] = [];
    return {
        adapter: {
            onRevealRequest(t) {
                reveals.push(t);
            },
        },
        reveals,
    };
}

function mount(): {
    container: HTMLElement;
    handle: PreviewHandle;
    reveals: RevealTarget[];
} {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const { adapter, reveals } = makeAdapter();
    const handle = mountPreview(container, adapter);
    return { container, handle, reveals };
}

const SAMPLE_SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
    <g class="kul-card" data-person-id="alice" data-kind="canonical" data-gender="female">
        <rect x="10" y="10" width="60" height="30"/>
        <text class="kul-label-name">Alice</text>
    </g>
    <g class="kul-card" data-person-id="alice" data-kind="ghost" data-ghost-reason="cross-ref">
        <rect x="120" y="10" width="60" height="30"/>
    </g>
    <path class="kul-edge" data-link-kind="birth" data-marriage-id="m1" data-child-id="alice" data-is-past="false" d="M 0 0 L 1 1"/>
    <path class="kul-edge" data-link-kind="marriage" data-marriage-id="m1" data-host-id="a" data-joining-id="b" d="M 0 0 L 1 1"/>
    <path class="kul-edge" data-link-kind="marriage" data-marriage-id="m2" data-host-id="c" data-joining-id="d" data-is-ended="true" d="M 0 0 L 1 1"/>
</svg>`;

beforeEach(() => {
    (svgPanZoom as unknown as ReturnType<typeof vi.fn>).mockClear();
});

afterEach(() => {
    document.body.innerHTML = "";
});

describe("mountPreview scaffold", () => {
    it("injects #root + control + popover + legend siblings into the container", () => {
        const { container } = mount();
        expect(container.querySelector("#root")).not.toBeNull();
        expect(container.querySelector("#kul-controls")).not.toBeNull();
        expect(container.querySelector("#kul-controls-group")).not.toBeNull();
        expect(container.querySelector("#kul-error-button")).not.toBeNull();
        expect(container.querySelector("#kul-error-popover")).not.toBeNull();
        expect(container.querySelector("#kul-legend")).not.toBeNull();
    });

    it("emits a vertical divider between pan/zoom and the legend toggle", () => {
        const { container } = mount();
        const html = container.innerHTML;
        const zoomOutIdx = html.indexOf('data-action="zoom-out"');
        const dividerIdx = html.indexOf('class="kul-control-divider"');
        const toggleIdx = html.indexOf('data-action="toggle-legend"');
        expect(zoomOutIdx).toBeLessThan(dividerIdx);
        expect(dividerIdx).toBeLessThan(toggleIdx);
    });

    it("legend toggle starts unpressed", () => {
        const { container } = mount();
        const toggle = container.querySelector(
            'button[data-action="toggle-legend"]',
        );
        expect(toggle?.getAttribute("aria-pressed")).toBe("false");
    });

    it("hides the controls panel and the error button on first mount", () => {
        const { container } = mount();
        const controls = container.querySelector("#kul-controls") as HTMLElement;
        const errorBtn = container.querySelector("#kul-error-button") as HTMLElement;
        expect(controls.hidden).toBe(true);
        expect(errorBtn.hidden).toBe(true);
    });
});

describe("mountPreview render lifecycle", () => {
    it("initialises svg-pan-zoom against the rendered SVG", () => {
        const { handle, container } = mount();
        handle.render(SAMPLE_SVG);
        expect(svgPanZoom).toHaveBeenCalledTimes(1);
        const svg = container.querySelector("svg");
        expect(svg).not.toBeNull();
    });

    it("passes controlIconsEnabled: false so the library's built-in icons stay off", () => {
        const { handle } = mount();
        handle.render(SAMPLE_SVG);
        const call = (svgPanZoom as unknown as ReturnType<typeof vi.fn>).mock.calls[0];
        const opts = call[1] as { controlIconsEnabled: boolean };
        expect(opts.controlIconsEnabled).toBe(false);
    });

    it("makes the pan/zoom group visible after the first render", () => {
        const { handle, container } = mount();
        const group = container.querySelector("#kul-controls-group") as HTMLElement;
        expect(group.hidden).toBe(true);
        handle.render(SAMPLE_SVG);
        expect(group.hidden).toBe(false);
    });

    it("hides the legend and skips pan-zoom when the rendered string contains no <svg>", () => {
        const { handle, container } = mount();
        handle.render("<div>no svg</div>");
        expect(svgPanZoom).not.toHaveBeenCalled();
        const legend = container.querySelector("#kul-legend") as HTMLElement;
        expect(legend.hidden).toBe(true);
    });

    it("destroys the prior pan/zoom instance before re-creating it on a new render", () => {
        const factory = svgPanZoom as unknown as ReturnType<typeof vi.fn>;
        const { handle } = mount();
        handle.render(SAMPLE_SVG);
        const first = factory.mock.results[0].value;
        handle.render(SAMPLE_SVG);
        expect(first.destroy).toHaveBeenCalled();
        expect(factory).toHaveBeenCalledTimes(2);
    });
});

describe("mountPreview click-to-source", () => {
    it("posts an entity revealRequest for a clicked person card", () => {
        const { handle, container, reveals } = mount();
        handle.render(SAMPLE_SVG);
        const card = container.querySelector(
            '.kul-card[data-person-id="alice"][data-kind="canonical"]',
        ) as Element;
        card.dispatchEvent(new MouseEvent("click", { bubbles: true }));
        expect(reveals).toEqual([{ kind: "entity", id: "alice" }]);
    });

    it("posts an entity revealRequest for a clicked marriage bar (uses the marriage id)", () => {
        const { handle, container, reveals } = mount();
        handle.render(SAMPLE_SVG);
        const marriage = container.querySelector(
            '.kul-edge[data-link-kind="marriage"][data-marriage-id="m1"]',
        ) as Element;
        marriage.dispatchEvent(new MouseEvent("click", { bubbles: true }));
        expect(reveals).toEqual([{ kind: "entity", id: "m1" }]);
    });

    it("ignores birth/adoption edges (keys on data-link-kind=marriage, not bare data-marriage-id)", () => {
        const { handle, container, reveals } = mount();
        handle.render(SAMPLE_SVG);
        const birth = container.querySelector(
            '.kul-edge[data-link-kind="birth"]',
        ) as Element;
        birth.dispatchEvent(new MouseEvent("click", { bubbles: true }));
        expect(reveals).toEqual([]);
    });
});

describe("mountPreview selection sync (highlightEntity)", () => {
    it("adds .kul-selected to the matching person card", () => {
        const { handle, container } = mount();
        handle.render(SAMPLE_SVG);
        handle.highlightEntity({ id: "alice", kind: "person" });
        const card = container.querySelector(
            '.kul-card[data-person-id="alice"][data-kind="canonical"]',
        );
        expect(card?.classList.contains("kul-selected")).toBe(true);
    });

    it("adds .kul-selected to the matching marriage bar", () => {
        const { handle, container } = mount();
        handle.render(SAMPLE_SVG);
        handle.highlightEntity({ id: "m1", kind: "marriage" });
        const bar = container.querySelector(
            '.kul-edge[data-link-kind="marriage"][data-marriage-id="m1"]',
        );
        expect(bar?.classList.contains("kul-selected")).toBe(true);
    });

    it("clears every prior .kul-selected before applying (stateless)", () => {
        const { handle, container } = mount();
        handle.render(SAMPLE_SVG);
        handle.highlightEntity({ id: "alice", kind: "person" });
        handle.highlightEntity({ id: "m1", kind: "marriage" });
        const selected = container.querySelectorAll(".kul-selected");
        expect(selected).toHaveLength(1);
    });

    it("treats null as clear-only", () => {
        const { handle, container } = mount();
        handle.render(SAMPLE_SVG);
        handle.highlightEntity({ id: "alice", kind: "person" });
        handle.highlightEntity(null);
        expect(container.querySelectorAll(".kul-selected")).toHaveLength(0);
    });
});

describe("mountPreview hover tooltip", () => {
    it("delegates hover on the rendered SVG and shows a .kul-tooltip after the hover-intent delay", async () => {
        vi.useFakeTimers();
        try {
            const { handle, container } = mount();
            handle.render(SAMPLE_SVG);
            const card = container.querySelector(
                '.kul-card[data-person-id="alice"][data-kind="canonical"]',
            ) as Element;
            card.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
            expect(document.querySelector(".kul-tooltip")).toBeNull();
            await vi.advanceTimersByTimeAsync(400);
            const tip = document.querySelector(".kul-tooltip");
            expect(tip).not.toBeNull();
            expect(tip?.querySelector(".kul-tooltip-kind")?.textContent).toBe("Person");
            expect(tip?.querySelector(".kul-tooltip-title")?.textContent).toBe("Alice");
        } finally {
            vi.useRealTimers();
        }
    });

    it("tears the tooltip down on render", async () => {
        vi.useFakeTimers();
        try {
            const { handle, container } = mount();
            handle.render(SAMPLE_SVG);
            const card = container.querySelector(
                '.kul-card[data-person-id="alice"][data-kind="canonical"]',
            ) as Element;
            card.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
            await vi.advanceTimersByTimeAsync(400);
            expect(document.querySelector(".kul-tooltip")).not.toBeNull();
            handle.render(SAMPLE_SVG);
            expect(document.querySelector(".kul-tooltip")).toBeNull();
        } finally {
            vi.useRealTimers();
        }
    });

    it("includes a field grid for non-empty person properties", async () => {
        vi.useFakeTimers();
        try {
            const { handle, container } = mount();
            handle.render(SAMPLE_SVG);
            const card = container.querySelector(
                '.kul-card[data-person-id="alice"][data-kind="canonical"]',
            ) as Element;
            card.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
            await vi.advanceTimersByTimeAsync(400);
            const labels = Array.from(
                document.querySelectorAll(".kul-tooltip-fields .kul-tooltip-label"),
            ).map((n) => n.textContent);
            expect(labels).toContain("Gender");
        } finally {
            vi.useRealTimers();
        }
    });
});

describe("mountPreview renderError last-good persistence (#203)", () => {
    it("does NOT wipe #root on showErrors (keeps the last-good SVG mounted)", () => {
        const { handle, container } = mount();
        handle.render(SAMPLE_SVG);
        handle.showErrors([{ message: "boom" }]);
        // Last-good SVG still in #root.
        expect(container.querySelector("svg")).not.toBeNull();
    });

    it("applies kul-render-stale to the last-good SVG on showErrors", () => {
        const { handle, container } = mount();
        handle.render(SAMPLE_SVG);
        handle.showErrors([{ message: "boom" }]);
        const svg = container.querySelector("svg");
        expect(svg?.classList.contains("kul-render-stale")).toBe(true);
    });

    it("clears the error state on a subsequent successful render", () => {
        const { handle, container } = mount();
        handle.render(SAMPLE_SVG);
        handle.showErrors([{ message: "boom" }]);
        handle.render(SAMPLE_SVG);
        const errorBtn = container.querySelector("#kul-error-button") as HTMLElement;
        expect(errorBtn.hidden).toBe(true);
    });
});

describe("mountPreview error button (#203)", () => {
    it("becomes visible when at least one error fires", () => {
        const { handle, container } = mount();
        handle.showErrors([{ message: "boom" }]);
        const errorBtn = container.querySelector("#kul-error-button") as HTMLElement;
        expect(errorBtn.hidden).toBe(false);
    });

    it("renders the error count badge with the current length", () => {
        const { handle, container } = mount();
        handle.showErrors([{ message: "a" }, { message: "b" }, { message: "c" }]);
        const badge = container.querySelector(".kul-error-count");
        expect(badge?.textContent).toBe("3");
    });

    it("flips the popover via the toggle-errors button click and tracks aria-pressed", () => {
        const { handle, container } = mount();
        handle.showErrors([{ message: "boom" }]);
        const errorBtn = container.querySelector("#kul-error-button") as HTMLElement;
        const popover = container.querySelector(
            "#kul-error-popover",
        ) as HTMLElement;
        expect(popover.hidden).toBe(true);
        expect(errorBtn.getAttribute("aria-pressed")).toBe("false");
        errorBtn.dispatchEvent(new MouseEvent("click", { bubbles: true }));
        expect(popover.hidden).toBe(false);
        expect(errorBtn.getAttribute("aria-pressed")).toBe("true");
        expect(errorBtn.getAttribute("aria-label")).toBe("Hide errors");
        errorBtn.dispatchEvent(new MouseEvent("click", { bubbles: true }));
        expect(popover.hidden).toBe(true);
        expect(errorBtn.getAttribute("aria-pressed")).toBe("false");
    });

    it("makes the panel visible from error presence alone (no render yet)", () => {
        const { handle, container } = mount();
        const controls = container.querySelector("#kul-controls") as HTMLElement;
        expect(controls.hidden).toBe(true);
        handle.showErrors([{ message: "boom" }]);
        expect(controls.hidden).toBe(false);
    });
});

describe("mountPreview error popover (#203)", () => {
    it("renders one row per error with a 1-based line + column", () => {
        const { handle, container } = mount();
        handle.showErrors([
            {
                message: "boom",
                code: "E001",
                uri: "file:///a.kul",
                range: {
                    start: { line: 0, character: 4 },
                    end: { line: 0, character: 5 },
                },
            },
        ]);
        const popover = container.querySelector("#kul-error-popover") as HTMLElement;
        expect(popover.innerHTML).toContain("E001");
        expect(popover.innerHTML).toContain("boom");
        expect(popover.innerHTML).toContain("Line 1, col 5");
    });

    it("posts a location revealRequest when a row is clicked", () => {
        const { handle, container, reveals } = mount();
        handle.showErrors([
            {
                message: "boom",
                uri: "file:///a.kul",
                range: {
                    start: { line: 1, character: 2 },
                    end: { line: 1, character: 3 },
                },
            },
        ]);
        handle.showErrors([
            {
                message: "boom",
                uri: "file:///a.kul",
                range: {
                    start: { line: 1, character: 2 },
                    end: { line: 1, character: 3 },
                },
            },
        ]);
        // First row should be addressable via the index attribute.
        const row = container.querySelector(
            '.kul-error-row[data-error-index="0"]',
        ) as Element;
        row.dispatchEvent(new MouseEvent("click", { bubbles: true }));
        expect(reveals).toEqual([
            {
                kind: "location",
                uri: "file:///a.kul",
                range: {
                    start: { line: 1, character: 2 },
                    end: { line: 1, character: 3 },
                },
            },
        ]);
    });

    it("escapes message / code / location HTML to keep the popover XSS-safe", () => {
        const { handle, container } = mount();
        handle.showErrors([{ message: "<script>x</script>", code: "<X>" }]);
        const popover = container.querySelector("#kul-error-popover") as HTMLElement;
        expect(popover.innerHTML).toContain("&lt;script&gt;x&lt;/script&gt;");
        expect(popover.innerHTML).toContain("&lt;X&gt;");
        expect(popover.querySelector("script")).toBeNull();
    });

    it("toggle-errors on an empty list is a no-op", () => {
        const { container } = mount();
        const errorBtn = container.querySelector("#kul-error-button") as HTMLElement;
        const popover = container.querySelector(
            "#kul-error-popover",
        ) as HTMLElement;
        errorBtn.dispatchEvent(new MouseEvent("click", { bubbles: true }));
        expect(popover.hidden).toBe(true);
    });

    it("hides the popover again whenever errors clears", () => {
        const { handle, container } = mount();
        handle.showErrors([{ message: "boom" }]);
        const errorBtn = container.querySelector("#kul-error-button") as HTMLElement;
        errorBtn.dispatchEvent(new MouseEvent("click", { bubbles: true }));
        const popover = container.querySelector(
            "#kul-error-popover",
        ) as HTMLElement;
        expect(popover.hidden).toBe(false);
        handle.showErrors([]);
        expect(popover.hidden).toBe(true);
    });
});

describe("mountPreview ghost badge", () => {
    it("appends a <g class='kul-ghost-badge'> with hit-target rect + title to each ghost card", () => {
        const { handle, container } = mount();
        handle.render(SAMPLE_SVG);
        const ghost = container.querySelector(
            '.kul-card[data-kind="ghost"]',
        ) as Element;
        const badge = ghost.querySelector("g.kul-ghost-badge");
        expect(badge).not.toBeNull();
        const title = badge?.querySelector("title");
        expect(title?.textContent).toBe("Jump to canonical card");
        const rects = badge?.querySelectorAll("rect");
        expect(rects?.length).toBe(1);
        expect(rects?.[0].getAttribute("fill")).toBe("transparent");
        expect(rects?.[0].getAttribute("pointer-events")).toBe("all");
    });

    it("badge click does not post a revealRequest (editor cursor untouched)", () => {
        const { handle, container, reveals } = mount();
        handle.render(SAMPLE_SVG);
        const badge = container.querySelector(".kul-ghost-badge") as Element;
        badge.dispatchEvent(new MouseEvent("click", { bubbles: true }));
        expect(reveals).toEqual([]);
    });
});

describe("mountPreview chrome legend overlay", () => {
    it("builds rows from the rendered SVG presence selectors", () => {
        const { handle, container } = mount();
        handle.render(SAMPLE_SVG);
        const legend = container.querySelector("#kul-legend") as HTMLElement;
        const rows = legend.querySelectorAll(".kul-legend-row");
        // Sample SVG includes female card, ghost, birth edge, un-ended marriage,
        // and ended marriage → five rows.
        const keys = Array.from(rows).map((r) => r.getAttribute("data-row"));
        expect(keys).toEqual([
            "gender-female",
            "past-record",
            "birth",
            "marriage",
            "ended-marriage",
        ]);
    });

    it("starts hidden — the ⓘ toggle is the discovery affordance", () => {
        const { handle, container } = mount();
        handle.render(SAMPLE_SVG);
        const legend = container.querySelector("#kul-legend") as HTMLElement;
        expect(legend.hidden).toBe(true);
    });

    it("flips the toggle and re-applies visibility on a toggle-legend click", () => {
        const { handle, container } = mount();
        handle.render(SAMPLE_SVG);
        const legend = container.querySelector("#kul-legend") as HTMLElement;
        const toggle = container.querySelector(
            'button[data-action="toggle-legend"]',
        ) as HTMLElement;
        toggle.dispatchEvent(new MouseEvent("click", { bubbles: true }));
        expect(legend.hidden).toBe(false);
        expect(toggle.getAttribute("aria-pressed")).toBe("true");
        expect(toggle.getAttribute("aria-label")).toBe("Hide legend");
        toggle.dispatchEvent(new MouseEvent("click", { bubbles: true }));
        expect(legend.hidden).toBe(true);
    });

    it("hides the legend on a missing <svg> render", () => {
        const { handle, container } = mount();
        handle.render(SAMPLE_SVG);
        handle.render("<div>no svg</div>");
        const legend = container.querySelector("#kul-legend") as HTMLElement;
        expect(legend.hidden).toBe(true);
    });
});
