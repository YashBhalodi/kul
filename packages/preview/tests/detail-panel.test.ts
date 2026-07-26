// The three panel variants, at the pure seam: one `EntityDetail` in, one
// model out. Plus the provenance lint ADR-0035 asks for — the widget may not
// read the rendered SVG, so this file asserts over `detail-panel.ts`'s own
// source text that it has no way to.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

import type { EntityDetail } from "../src/engine-wire.js";
import { buildDetailPanel } from "../src/detail-panel.js";
import type { DetailPanelView } from "../src/detail-panel.js";
import { EN } from "../src/phrasing/index.js";

/** No Explore-kin list: the variants below are about the entity, not the list. */
const NO_KIN: DetailPanelView = { pack: EN, kin: null };

const GIULIA = {
    id: "giulia",
    name: "Giulia Rossi",
    family: "Rossi",
    gender: "female",
    born: { value: "1928", precision: "year", circa: true },
};
const MARCO = { id: "marco", name: "Marco Rossi", gender: "male" };
const ALDO = { id: "aldo", name: "Aldo Bruno", gender: "male" };
const LUCIA = { id: "lucia", name: "Lucia Rossi", gender: "female" };
const DALISAY = { id: "dalisay", name: "Dalisay Reyes", gender: "female" };

const BIRTH_LINK = {
    marriageId: "m1",
    childId: "lucia",
    kind: "biological" as const,
};
const ADOPTION_LINK = {
    marriageId: "m1",
    childId: "dalisay",
    kind: "adoptive" as const,
    start: { value: "1955-06-01", precision: "day", circa: false },
};

const M1 = {
    id: "m1",
    spouses: ["giulia", "marco"] as [string, string],
    start: { value: "1948-03-02", precision: "day", circa: false },
    end: { value: "1970", precision: "year", circa: false },
    endReason: "divorce",
};
const M2 = { id: "m2", spouses: ["giulia", "aldo"] as [string, string] };

describe("the person variant", () => {
    const detail: EntityDetail = {
        kind: "person",
        person: GIULIA,
        parents: [],
        marriages: [
            { marriage: M1, spouse: MARCO },
            { marriage: M2, spouse: ALDO },
        ],
        children: [
            { person: LUCIA, link: BIRTH_LINK },
            { person: DALISAY, link: ADOPTION_LINK },
        ],
    };

    it("titles on the engine's name and reveals the person", () => {
        const model = buildDetailPanel(detail, NO_KIN);
        expect(model.kicker).toBe("Person");
        expect(model.title).toBe("Giulia Rossi");
        expect(model.revealId).toBe("giulia");
    });

    it("omits absent fields rather than rendering 'not recorded'", () => {
        const model = buildDetailPanel(detail, NO_KIN);
        const labels = model.fields.map((field) => field.label);
        expect(labels).toEqual(["Gender", "Family name", "Born"]);
        // No `died`, no `given` on the wire, so neither appears at all.
        expect(labels).not.toContain("Died");
        expect(model.fields).toContainEqual({ label: "Born", value: "c. 1928" });
    });

    it("gives each marriage its own line with its span and end reason", () => {
        const model = buildDetailPanel(detail, NO_KIN);
        const marriages = model.sections.find((s) => s.title === "Marriages");
        expect(marriages?.rows).toEqual([
            {
                person: { id: "marco", name: "Marco Rossi" },
                note: "1948-03-02 – 1970 · divorce",
            },
            // The second marriage records no dates; its line simply carries none.
            { person: { id: "aldo", name: "Aldo Bruno" }, note: undefined },
        ]);
    });

    it("carries no ghost note — the ended marriage is the underlying truth", () => {
        const model = buildDetailPanel(detail, NO_KIN);
        const text = JSON.stringify(model).toLowerCase();
        expect(text).not.toContain("ghost");
        expect(text).not.toContain("past-record");
    });

    it("notes each child's link kind in the document's own vocabulary", () => {
        const model = buildDetailPanel(detail, NO_KIN);
        const children = model.sections.find((s) => s.title === "Children");
        expect(children?.rows).toEqual([
            { person: { id: "lucia", name: "Lucia Rossi" }, note: "birth" },
            {
                person: { id: "dalisay", name: "Dalisay Reyes" },
                note: "adoption · from 1955-06-01",
            },
        ]);
    });

    it("omits a section with no rows rather than emptying it", () => {
        const model = buildDetailPanel(detail, NO_KIN);
        expect(model.sections.map((s) => s.title)).toEqual([
            "Marriages",
            "Children",
        ]);
    });

    it("keeps one row per link, so a person reached twice appears twice", () => {
        const model = buildDetailPanel({
            kind: "person",
            person: LUCIA,
            parents: [
                { person: GIULIA, link: { ...BIRTH_LINK, childId: "lucia" } },
                {
                    person: GIULIA,
                    link: { ...ADOPTION_LINK, childId: "lucia", marriageId: "m2" },
                },
            ],
            marriages: [],
            children: [],
        }, NO_KIN);
        const parents = model.sections.find((s) => s.title === "Parents");
        expect(parents?.rows).toHaveLength(2);
        expect(parents?.rows.map((row) => row.note)).toEqual([
            "birth",
            "adoption · from 1955-06-01",
        ]);
    });
});

describe("the marriage variant", () => {
    const detail: EntityDetail = {
        kind: "marriage",
        marriage: M1,
        spouses: [GIULIA, MARCO],
        children: [{ person: LUCIA, link: BIRTH_LINK }],
    };

    it("titles on both spouses and reveals the marriage — the seam now works for edges", () => {
        const model = buildDetailPanel(detail, NO_KIN);
        expect(model.kicker).toBe("Marriage");
        expect(model.title).toBe("Giulia Rossi & Marco Rossi");
        expect(model.revealId).toBe("m1");
    });

    it("lists its own fields and its spouses and children as clickable rows", () => {
        const model = buildDetailPanel(detail, NO_KIN);
        expect(model.fields).toEqual([
            { label: "Started", value: "1948-03-02" },
            { label: "Ended", value: "1970" },
            { label: "End reason", value: "divorce" },
        ]);
        const spouses = model.sections.find((s) => s.title === "Spouses");
        expect(spouses?.rows.map((row) => row.person?.id)).toEqual([
            "giulia",
            "marco",
        ]);
    });

    it("omits the end fields of a marriage that has not ended", () => {
        const model = buildDetailPanel({
            kind: "marriage",
            marriage: M2,
            spouses: [GIULIA, ALDO],
            children: [],
        }, NO_KIN);
        expect(model.fields).toEqual([]);
    });
});

describe("the adoption variant", () => {
    const detail: EntityDetail = {
        kind: "adoption",
        adoption: ADOPTION_LINK,
        child: DALISAY,
        parents: [GIULIA, MARCO],
    };

    it("reveals the child, because an adoption link has no entity id", () => {
        const model = buildDetailPanel(detail, NO_KIN);
        expect(model.kicker).toBe("Adoption");
        expect(model.title).toBe("Dalisay Reyes");
        expect(model.revealId).toBe("dalisay");
    });

    it("names the child and the adopting parents as clickable rows", () => {
        const model = buildDetailPanel(detail, NO_KIN);
        expect(model.sections.map((s) => s.title)).toEqual([
            "Child",
            "Adoptive parents",
        ]);
        expect(
            model.sections.flatMap((s) => s.rows.map((row) => row.person?.id)),
        ).toEqual(["dalisay", "giulia", "marco"]);
    });
});

describe("panel content has one provenance path", () => {
    const source = readFileSync(
        join(dirname(fileURLToPath(import.meta.url)), "..", "src", "detail-panel.ts"),
        "utf8",
    )
        .replace(/\/\*[\s\S]*?\*\//g, "")
        .replace(/^[ \t]*\/\/.*$/gm, "");

    it("reads no SVG attribute vocabulary, in any form", () => {
        // ADR-0034 refused reading the panel off `data-*` on principle and
        // ADR-0035 refused the hybrid again after discovering the contract
        // could not answer everything. The fix was to widen the contract, so
        // the widget must have no way back to the picture.
        for (const forbidden of [
            "data-",
            "dataset",
            "getAttribute",
            "getElementById",
            "getElementsBy",
            "querySelector",
            "attributes",
            "closest",
        ]) {
            expect(source).not.toContain(forbidden);
        }
    });
});
