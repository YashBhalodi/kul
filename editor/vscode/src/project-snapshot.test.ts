import { describe, expect, it } from "vitest";

import {
    type ProjectReader,
    collectProjectSnapshot,
    manifestVersion,
} from "./project-snapshot";

function reader(entries: Record<string, string | null>): ProjectReader {
    return {
        listEntries: async () => Object.keys(entries),
        readText: async (name) => entries[name] ?? null,
    };
}

describe("manifestVersion", () => {
    it("reads a quoted version", () => {
        expect(manifestVersion('kul: "0.1"\n')).toBe("0.1");
        expect(manifestVersion("kul: '0.1'\n")).toBe("0.1");
    });

    it("reads an unquoted version", () => {
        expect(manifestVersion("kul: 0.1\n")).toBe("0.1");
    });

    it("ignores a trailing comment", () => {
        expect(manifestVersion("kul: 0.1  # the language version\n")).toBe("0.1");
    });

    it("finds the key when it is not the first line", () => {
        expect(manifestVersion("# a manifest\n\nkul: 0.1\n")).toBe("0.1");
    });

    it("ignores an indented (nested) key of the same name", () => {
        expect(manifestVersion("other:\n  kul: 9.9\n")).toBe("");
    });

    it("answers empty for a missing or unreadable manifest", () => {
        // Reports what was found rather than inventing a plausible version.
        expect(manifestVersion(null)).toBe("");
        expect(manifestVersion("")).toBe("");
        expect(manifestVersion("something: else\n")).toBe("");
    });
});

describe("collectProjectSnapshot", () => {
    it("collects every .kul file in lexicographic order", async () => {
        const snapshot = await collectProjectSnapshot(
            reader({
                "kul.yml": 'kul: "0.1"\n',
                "zeta.kul": "z",
                "alpha.kul": "a",
            }),
        );
        expect(snapshot.files.map((f) => f.name)).toEqual([
            "alpha.kul",
            "zeta.kul",
        ]);
        expect(snapshot.manifest).toEqual({ kul: "0.1" });
    });

    it("ignores non-.kul entries, the manifest included", async () => {
        const snapshot = await collectProjectSnapshot(
            reader({
                "kul.yml": "kul: 0.1\n",
                "tree.svg": "<svg/>",
                "notes.md": "#",
                "a.kul": "a",
            }),
        );
        expect(snapshot.files.map((f) => f.name)).toEqual(["a.kul"]);
    });

    it("skips a file it cannot read rather than failing the whole snapshot", async () => {
        const snapshot = await collectProjectSnapshot(
            reader({ "kul.yml": "kul: 0.1\n", "a.kul": "a", "b.kul": null }),
        );
        expect(snapshot.files.map((f) => f.name)).toEqual(["a.kul"]);
    });

    it("carries each file's text verbatim", async () => {
        const source = 'person a name:"A" gender:male\n';
        const snapshot = await collectProjectSnapshot(
            reader({ "kul.yml": "kul: 0.1\n", "a.kul": source }),
        );
        expect(snapshot.files).toEqual([{ name: "a.kul", source }]);
    });
});
