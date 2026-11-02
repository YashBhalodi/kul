import { describe, expect, it } from "vitest";

import {
    type ProjectReader,
    collectProjectSnapshot,
} from "./project-snapshot";

function reader(entries: Record<string, string | null>): ProjectReader {
    return {
        listEntries: async () => Object.keys(entries),
        readText: async (name) => entries[name] ?? null,
    };
}

describe("collectProjectSnapshot", () => {
    it("collects every .kul file in lexicographic order", async () => {
        const snapshot = await collectProjectSnapshot(
            reader({
                "zeta.kul": "z",
                "alpha.kul": "a",
            }),
        );
        expect(snapshot.files.map((f) => f.name)).toEqual([
            "alpha.kul",
            "zeta.kul",
        ]);
    });

    it("ignores non-.kul entries", async () => {
        const snapshot = await collectProjectSnapshot(
            reader({
                "tree.svg": "<svg/>",
                "notes.md": "#",
                "a.kul": "a",
            }),
        );
        expect(snapshot.files.map((f) => f.name)).toEqual(["a.kul"]);
    });

    it("skips a file it cannot read rather than failing the whole snapshot", async () => {
        const snapshot = await collectProjectSnapshot(
            reader({ "a.kul": "a", "b.kul": null }),
        );
        expect(snapshot.files.map((f) => f.name)).toEqual(["a.kul"]);
    });

    it("carries each file's text verbatim", async () => {
        const source = 'person a name:"A" gender:male\n';
        const snapshot = await collectProjectSnapshot(
            reader({ "a.kul": source }),
        );
        expect(snapshot.files).toEqual([{ name: "a.kul", source }]);
    });
});
