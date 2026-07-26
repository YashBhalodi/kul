import { describe, expect, it } from "vitest";

import { isEntityKind, isProjectSnapshot, isRevealTarget } from "../src/wire-guards.js";

describe("isEntityKind", () => {
    it("accepts the two union members", () => {
        expect(isEntityKind("person")).toBe(true);
        expect(isEntityKind("marriage")).toBe(true);
    });

    it("rejects any value outside the union", () => {
        for (const bad of ["Person", "PERSON", "birth", "", 0, null, undefined, {}]) {
            expect(isEntityKind(bad)).toBe(false);
        }
    });
});

describe("isRevealTarget", () => {
    it("accepts a well-formed entity target", () => {
        expect(isRevealTarget({ kind: "entity", id: "p1" })).toBe(true);
    });

    it("accepts a well-formed location target", () => {
        expect(
            isRevealTarget({
                kind: "location",
                uri: "file:///a.kul",
                range: {
                    start: { line: 0, character: 0 },
                    end: { line: 1, character: 4 },
                },
            }),
        ).toBe(true);
    });

    it("rejects non-object payloads", () => {
        for (const bad of [null, undefined, "entity", 42, []]) {
            expect(isRevealTarget(bad)).toBe(false);
        }
    });

    it("rejects an unknown discriminant", () => {
        expect(isRevealTarget({ kind: "marriage", id: "m1" })).toBe(false);
    });

    it("rejects an entity target with a non-string id", () => {
        expect(isRevealTarget({ kind: "entity", id: 7 })).toBe(false);
        expect(isRevealTarget({ kind: "entity" })).toBe(false);
    });

    it("rejects a location target with a malformed range", () => {
        expect(
            isRevealTarget({ kind: "location", uri: "file:///a.kul" }),
        ).toBe(false);
        expect(
            isRevealTarget({
                kind: "location",
                uri: "file:///a.kul",
                range: { start: { line: 0 }, end: { line: 1, character: 4 } },
            }),
        ).toBe(false);
        expect(
            isRevealTarget({
                kind: "location",
                uri: "file:///a.kul",
                range: { start: "0,0", end: "1,4" },
            }),
        ).toBe(false);
    });

    it("rejects a location target with a non-string uri", () => {
        expect(
            isRevealTarget({
                kind: "location",
                uri: 5,
                range: {
                    start: { line: 0, character: 0 },
                    end: { line: 1, character: 4 },
                },
            }),
        ).toBe(false);
    });
});

describe("isProjectSnapshot", () => {
    it("accepts a well-formed snapshot", () => {
        expect(
            isProjectSnapshot({
                files: [{ name: "a.kul", source: "person a\n" }],
                manifest: { kul: "0.1" },
            }),
        ).toBe(true);
    });

    it("accepts an empty file list (an empty project is a real project)", () => {
        expect(isProjectSnapshot({ files: [], manifest: { kul: "0.1" } })).toBe(true);
    });

    it("rejects the whole snapshot when one file is malformed", () => {
        // A partly-valid list would query a different project from the one the
        // picture shows, so it is rejected whole rather than filtered.
        expect(
            isProjectSnapshot({
                files: [
                    { name: "a.kul", source: "person a\n" },
                    { name: "b.kul" },
                ],
                manifest: { kul: "0.1" },
            }),
        ).toBe(false);
    });

    it("rejects a missing or non-string manifest version", () => {
        expect(isProjectSnapshot({ files: [] })).toBe(false);
        expect(isProjectSnapshot({ files: [], manifest: {} })).toBe(false);
        expect(isProjectSnapshot({ files: [], manifest: { kul: 0.1 } })).toBe(false);
    });

    it("rejects non-object payloads", () => {
        for (const bad of [null, undefined, 42, "project", []]) {
            expect(isProjectSnapshot(bad)).toBe(false);
        }
    });
});
