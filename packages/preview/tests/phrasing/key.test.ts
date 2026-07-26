import { describe, expect, it } from "vitest";

import { phrasingKeyOf, subPathKeyOf } from "../../src/phrasing/key.js";
import { across, collateral, descriptorOf, down, lineal, self, up } from "./fixtures.js";

/**
 * The six backbone-derived facets (ADR-0033) and the sub-path derivation.
 * Both are pure functions of the hop sequence — no engine change, no new wire
 * field — so this suite is where their definitions are pinned.
 */
describe("the derived facets", () => {
    it("counts `across` hops and reports where they sit", () => {
        const spousesSiblingsSpouse = descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            path: [across("female"), up("male"), down("female"), across("male")],
        });
        expect(phrasingKeyOf(spousesSiblingsSpouse)).toMatchObject({
            acrossCount: 2,
            acrossAtStart: true,
            acrossAtEnd: true,
            spouseGender: "female",
            linkGender: "female",
            endedMarriage: false,
        });
    });

    it("reports `spouseGender` only for a path-initial `across`", () => {
        const siblingsHusband = descriptorOf({
            classification: collateral(1, 1),
            affinity: "inLaw",
            path: [up("female"), down("female"), across("male")],
        });
        expect(phrasingKeyOf(siblingsHusband)).toMatchObject({
            acrossAtStart: false,
            acrossAtEnd: true,
            spouseGender: "notApplicable",
            linkGender: "female",
        });
    });

    it("takes `linkGender` from the first `down` hop, never the endpoint", () => {
        // A daughter's son: the linking child is female, the alter male.
        const grandson = descriptorOf({
            classification: lineal("descendant", 2),
            path: [down("female"), down("male")],
        });
        expect(phrasingKeyOf(grandson)).toMatchObject({
            linkGender: "female",
            alterGender: "male",
        });
    });

    it("flags an ended marriage anywhere on the path", () => {
        const formerSpouse = descriptorOf({
            classification: self,
            affinity: "inLaw",
            path: [across("male", "ended")],
        });
        expect(phrasingKeyOf(formerSpouse).endedMarriage).toBe(true);
    });

    it("reports `notApplicable` — a fact about the shape — not `unknown`", () => {
        const mother = descriptorOf({
            classification: lineal("ancestor", 1),
            path: [up("female")],
        });
        expect(phrasingKeyOf(mother)).toMatchObject({
            spouseGender: "notApplicable",
            linkGender: "notApplicable",
            up: "notApplicable",
            cousinDegree: "notApplicable",
        });
    });

    it("takes the normalized dimensions from the descriptor, not from the hops", () => {
        // `sharing` and `side` are graph facts the engine supplies; a whole-path
        // key passes them through verbatim.
        const sibling = descriptorOf({
            classification: collateral(1, 1),
            sharing: "half",
            side: "paternal",
            seniority: "elder",
            apexSeniority: "elder",
            path: [up("male"), down("male")],
        });
        expect(phrasingKeyOf(sibling)).toMatchObject({
            sharing: "half",
            side: "paternal",
            seniority: "elder",
            apexSeniority: "elder",
            cousinDegree: 0,
            removed: 0,
        });
    });
});

describe("sub-path keys", () => {
    const uncle = descriptorOf({
        classification: collateral(2, 1),
        side: "paternal",
        path: [up("male"), up("female"), down("male")],
    });

    it("re-derives classification, affinity and the endpoint from the prefix", () => {
        expect(subPathKeyOf(uncle, 2)).toMatchObject({
            classification: "lineal",
            role: "ancestor",
            generations: 2,
            affinity: "blood",
            alterGender: "female",
            egoGender: uncle.egoGender,
        });
    });

    it("keeps the routing `side` when the prefix cannot be a couple apex", () => {
        expect(subPathKeyOf(uncle, 2).side).toBe("paternal");
    });

    it("marks the graph-dependent facets unknown", () => {
        const sibling = descriptorOf({
            classification: collateral(1, 2),
            sharing: "full",
            side: "both",
            path: [up("male"), down("male"), down("male")],
        });
        expect(subPathKeyOf(sibling, 2)).toMatchObject({
            sharing: "unknown",
            apexSeniority: "unknown",
            seniority: "unknown",
            side: "unknown",
        });
    });

    it("re-derives `affinity` from the prefix, which can differ from the whole path", () => {
        // Whole path `up·across·down` is `step`; its `up` prefix is plain blood.
        const stepSibling = descriptorOf({
            classification: collateral(1, 1),
            affinity: "step",
            side: "maternal",
            path: [up("female"), across("male"), down("male")],
        });
        expect(subPathKeyOf(stepSibling, 1).affinity).toBe("blood");
        expect(subPathKeyOf(stepSibling, 2).affinity).toBe("step");
    });
});
