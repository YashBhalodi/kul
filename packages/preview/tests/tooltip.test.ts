import { describe, expect, it } from "vitest";

import { buildTooltip } from "../src/tooltip.js";

function attr(name: string, value: string): { name: string; value: string } {
    return { name, value };
}

function namer(map: Record<string, string> = {}): (id: string) => string {
    return (id) => map[id] ?? id;
}

function rowsOf(
    attrs: Array<{ name: string; value: string }>,
    resolve = namer(),
): Array<{ label: string; value: string }> {
    const model = buildTooltip(attrs, resolve);
    expect(model).not.toBeNull();
    return model!.rows;
}

describe("buildTooltip typed header", () => {
    it("titles a person card and resolves its display name", () => {
        const model = buildTooltip(
            [attr("data-person-id", "alice"), attr("data-gender", "female")],
            namer({ alice: "Alice Adeyemi" }),
        );
        expect(model?.title).toBe("Person");
        expect(model?.identity).toBe("Alice Adeyemi");
    });

    it("titles a marriage edge and joins both spouse names", () => {
        const model = buildTooltip(
            [
                attr("data-link-kind", "marriage"),
                attr("data-host-id", "a"),
                attr("data-joining-id", "b"),
                attr("data-start", "1962"),
            ],
            namer({ a: "Babatunde Adeyemi", b: "Amaka Adeyemi" }),
        );
        expect(model?.title).toBe("Marriage");
        expect(model?.identity).toBe("Babatunde Adeyemi & Amaka Adeyemi");
    });

    it("titles an adoption edge and resolves the child name", () => {
        const model = buildTooltip(
            [
                attr("data-link-kind", "adoption"),
                attr("data-child-id", "c"),
                attr("data-adoption-start", "1990"),
            ],
            namer({ c: "Bisi Adeyemi" }),
        );
        expect(model?.title).toBe("Adoption");
        expect(model?.identity).toBe("Bisi Adeyemi");
    });

    it("falls back to the id when a name can't be resolved", () => {
        const model = buildTooltip(
            [attr("data-person-id", "ghost42")],
            namer(),
        );
        expect(model?.identity).toBe("ghost42");
    });

    it("returns null for a birth edge (purely structural)", () => {
        expect(
            buildTooltip(
                [
                    attr("data-link-kind", "birth"),
                    attr("data-child-id", "c"),
                    attr("data-is-past", "true"),
                ],
                namer({ c: "Bisi Adeyemi" }),
            ),
        ).toBeNull();
    });

    it("returns null for an element that is neither a card nor a known edge", () => {
        expect(buildTooltip([attr("class", "kul-canvas")], namer())).toBeNull();
    });
});

describe("buildTooltip rows — denylist", () => {
    it("omits the structural attributes, keeping only display fields", () => {
        const rows = rowsOf([
            attr("data-person-id", "alice"),
            attr("data-kind", "canonical"),
            attr("data-gender", "female"),
            attr("data-is-alive", "true"),
            attr("data-born", "1850"),
            attr("data-generation", "0"),
        ]);
        expect(rows).toEqual([
            { label: "Gender", value: "Female" },
            { label: "Born", value: "1850" },
        ]);
    });

    it("ignores non-data-* attributes entirely", () => {
        const rows = rowsOf([
            attr("data-person-id", "p"),
            attr("class", "kul-card"),
            attr("transform", "translate(10,20)"),
            attr("data-given", "Alice"),
        ]);
        expect(rows).toEqual([{ label: "Given name", value: "Alice" }]);
    });

    it("omits empty values (no placeholder rows)", () => {
        const rows = rowsOf([
            attr("data-person-id", "p"),
            attr("data-born", "1850"),
            attr("data-died", ""),
            attr("data-family", ""),
        ]);
        expect(rows).toEqual([{ label: "Born", value: "1850" }]);
    });
});

describe("buildTooltip rows — scope and order", () => {
    it("surfaces a person's non-empty fields in DOM (emit) order", () => {
        const rows = rowsOf([
            attr("data-person-id", "p"),
            attr("data-kind", "canonical"),
            attr("data-gender", "male"),
            attr("data-is-alive", "false"),
            attr("data-born", "1820"),
            attr("data-died", "1890"),
            attr("data-family", "Curie"),
            attr("data-given", "Pierre"),
            attr("data-generation", "0"),
        ]);
        expect(rows).toEqual([
            { label: "Gender", value: "Male" },
            { label: "Born", value: "1820" },
            { label: "Died", value: "1890" },
            { label: "Family name", value: "Curie" },
            { label: "Given name", value: "Pierre" },
        ]);
    });

    it("surfaces a marriage edge's start, end, and end-reason", () => {
        const rows = rowsOf([
            attr("data-marriage-id", "m1"),
            attr("data-link-kind", "marriage"),
            attr("data-host-id", "a"),
            attr("data-joining-id", "b"),
            attr("data-start", "1870"),
            attr("data-is-ended", "true"),
            attr("data-end", "1885"),
            attr("data-end-reason", "divorce"),
        ]);
        expect(rows).toEqual([
            { label: "Start", value: "1870" },
            { label: "End", value: "1885" },
            { label: "End reason", value: "Divorce" },
        ]);
    });

    it("omits the Start row when a marriage has no data-start (#198)", () => {
        const rows = rowsOf([
            attr("data-marriage-id", "m1"),
            attr("data-link-kind", "marriage"),
            attr("data-host-id", "a"),
            attr("data-joining-id", "b"),
            attr("data-is-ended", "false"),
        ]);
        expect(rows).toEqual([]);
    });

    it("surfaces an adoption edge's adoption start/end", () => {
        const rows = rowsOf([
            attr("data-marriage-id", "m1"),
            attr("data-link-kind", "adoption"),
            attr("data-child-id", "c"),
            attr("data-is-past", "false"),
            attr("data-adoption-start", "1900"),
            attr("data-adoption-end", "1905"),
        ]);
        expect(rows).toEqual([
            { label: "Adoption start", value: "1900" },
            { label: "Adoption end", value: "1905" },
        ]);
    });
});

describe("buildTooltip rows — label humanization", () => {
    it("strips data-, turns - into space, and capitalizes", () => {
        expect(
            rowsOf([attr("data-link-kind", "marriage"), attr("data-end-reason", "x")])[0]
                .label,
        ).toBe("End reason");
        expect(
            rowsOf([
                attr("data-link-kind", "adoption"),
                attr("data-adoption-start", "x"),
            ])[0].label,
        ).toBe("Adoption start");
    });

    it("applies the family/given override map", () => {
        expect(
            rowsOf([attr("data-person-id", "p"), attr("data-family", "x")])[0].label,
        ).toBe("Family name");
        expect(
            rowsOf([attr("data-person-id", "p"), attr("data-given", "x")])[0].label,
        ).toBe("Given name");
    });
});

describe("buildTooltip rows — value capitalization", () => {
    it("capitalizes the first letter of a worded value", () => {
        expect(
            rowsOf([attr("data-person-id", "p"), attr("data-gender", "male")])[0].value,
        ).toBe("Male");
        expect(
            rowsOf([
                attr("data-link-kind", "marriage"),
                attr("data-end-reason", "divorce"),
            ])[0].value,
        ).toBe("Divorce");
    });

    it("passes dates through verbatim, preserving the ~ approximate marker", () => {
        expect(
            rowsOf([attr("data-person-id", "p"), attr("data-born", "1850")])[0].value,
        ).toBe("1850");
        expect(
            rowsOf([attr("data-person-id", "p"), attr("data-died", "~1890")])[0].value,
        ).toBe("~1890");
    });
});
