import { describe, expect, it } from "vitest";

import { LEGEND_ROWS, legendSwatchInnerSvg, presentLegendRows } from "../src/legend.js";

describe("LEGEND_ROWS normative table", () => {
    it("lists the canonical eight rows in normative order", () => {
        expect(LEGEND_ROWS.map((r) => r.key)).toEqual([
            "gender-male",
            "gender-female",
            "gender-other",
            "past-record",
            "birth",
            "adoption",
            "marriage",
            "ended-marriage",
        ]);
    });

    it("uses the normative English label strings", () => {
        expect(LEGEND_ROWS.map((r) => r.label)).toEqual([
            "Male",
            "Female",
            "Other",
            "Past record",
            "Birth",
            "Adoption",
            "Marriage",
            "Ended marriage",
        ]);
    });

    it("keys each row on the production data-* attribute (the seam, not a new vocabulary)", () => {
        const map = Object.fromEntries(
            LEGEND_ROWS.map((r) => [r.key, r.presenceSelector]),
        );
        expect(map["gender-male"]).toContain('data-gender="male"');
        expect(map["gender-female"]).toContain('data-gender="female"');
        expect(map["gender-other"]).toContain('data-gender="other"');
        expect(map["past-record"]).toContain('data-kind="ghost"');
        expect(map["birth"]).toContain('data-link-kind="birth"');
        expect(map["adoption"]).toContain('data-link-kind="adoption"');
        expect(map["marriage"]).toContain('data-link-kind="marriage"');
        expect(map["marriage"]).toContain(':not([data-is-ended="true"])');
        expect(map["ended-marriage"]).toContain('data-link-kind="marriage"');
        expect(map["ended-marriage"]).toContain('data-is-ended="true"');
    });
});

describe("presentLegendRows dynamic presence", () => {
    function fakeHas(present: ReadonlyArray<string>): (selector: string) => unknown {
        const set = new Set(present);
        return (selector) => (set.has(selector) ? {} : null);
    }

    it("returns every row when every category is present, in canonical order", () => {
        const allSelectors = LEGEND_ROWS.map((r) => r.presenceSelector);
        const rows = presentLegendRows(fakeHas(allSelectors));
        expect(rows.map((r) => r.key)).toEqual(LEGEND_ROWS.map((r) => r.key));
    });

    it("returns the empty list when no category is present", () => {
        expect(presentLegendRows(fakeHas([])).length).toBe(0);
    });

    it("filters to only the present categories (no adoption → no Adoption row)", () => {
        const present = [
            '.kul-card[data-gender="male"]',
            '.kul-card[data-gender="female"]',
            '.kul-edge[data-link-kind="birth"]',
            '.kul-edge[data-link-kind="marriage"]:not([data-is-ended="true"])',
        ];
        const rows = presentLegendRows(fakeHas(present));
        expect(rows.map((r) => r.key)).toEqual([
            "gender-male",
            "gender-female",
            "birth",
            "marriage",
        ]);
    });

    it("shows only Ended marriage when the only marriage in the diagram is ended", () => {
        const present = [
            '.kul-card[data-gender="male"]',
            '.kul-card[data-gender="female"]',
            '.kul-edge[data-link-kind="marriage"][data-is-ended="true"]',
        ];
        const rows = presentLegendRows(fakeHas(present));
        expect(rows.map((r) => r.key)).toEqual([
            "gender-male",
            "gender-female",
            "ended-marriage",
        ]);
    });
});

describe("legendSwatchInnerSvg", () => {
    it("emits a card swatch reusing the production class + data-* per gender", () => {
        expect(legendSwatchInnerSvg("gender-male")).toContain(
            'class="kul-card" data-kind="canonical" data-gender="male"',
        );
        expect(legendSwatchInnerSvg("gender-female")).toContain(
            'data-gender="female"',
        );
        expect(legendSwatchInnerSvg("gender-other")).toContain(
            'data-gender="other"',
        );
    });

    it("emits a ghost swatch with the inline structural dashed border (mirrors production)", () => {
        const svg = legendSwatchInnerSvg("past-record");
        expect(svg).toContain('class="kul-card" data-kind="ghost"');
        expect(svg).toContain('stroke-dasharray="3 2"');
    });

    it("emits edge swatches reusing the production class + data-link-kind", () => {
        expect(legendSwatchInnerSvg("birth")).toContain(
            'class="kul-edge" data-link-kind="birth"',
        );
        const adoption = legendSwatchInnerSvg("adoption");
        expect(adoption).toContain('data-link-kind="adoption"');
        expect(adoption).toContain('stroke-dasharray="6 4"');
        expect(legendSwatchInnerSvg("marriage")).toContain(
            'class="kul-edge" data-link-kind="marriage"',
        );
        const ended = legendSwatchInnerSvg("ended-marriage");
        expect(ended).toContain('data-link-kind="marriage"');
        expect(ended).toContain('data-is-ended="true"');
    });

    it("never bakes a colour into a swatch (no fill/stroke= attributes beyond fill=none)", () => {
        for (const row of LEGEND_ROWS) {
            const svg = legendSwatchInnerSvg(row.key);
            const stripped = svg.replace(/fill="none"/g, "");
            expect(stripped).not.toContain(' fill="');
            expect(stripped).not.toContain(' stroke="');
        }
    });

    it("returns the empty string for an unknown key (defensive)", () => {
        expect(legendSwatchInnerSvg("not-a-real-row")).toBe("");
    });
});
