// The query transport, end to end inside the webview (ADR-0034):
// extension message → chrome → engine → envelope, with the engine's module
// loader substituted at its system boundary.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("svg-pan-zoom", () => {
    const make = () => ({
        getPan: () => ({ x: 0, y: 0 }),
        getZoom: () => 1,
        getSizes: () => ({ width: 800, height: 600, realZoom: 1 }),
        pan: vi.fn(),
        panBy: vi.fn(),
        zoom: vi.fn(),
        zoomIn: vi.fn(),
        zoomOut: vi.fn(),
        reset: vi.fn(),
        destroy: vi.fn(),
    });
    return { default: vi.fn(() => make()) };
});

import { installVscodeInboundBridge } from "../src/adapter-vscode.js";
import type {
    DetailLookupResult,
    DetailTarget,
    Manifest,
    Query,
    QueryEnvelope,
    WasmInputFile,
} from "../src/engine-wire.js";
import { isQueryOk } from "../src/engine-wire.js";
import {
    type EngineModule,
    type ProjectSnapshot,
    createQueryEngine,
} from "../src/engine.js";
import { mountPreview } from "../src/mount.js";
import type { HostAdapter } from "../src/types.js";

const SOURCE = {
    moduleUri: "https://host.example/media/preview/wasm/kul_wasm.js",
    wasmUri: "https://host.example/media/preview/wasm/kul_wasm_bg.wasm",
};

const PROJECT: ProjectSnapshot = {
    files: [
        {
            name: "family.kul",
            source: 'person giuseppe name:"Giuseppe" gender:male\n',
        },
    ],
    manifest: { kul: "0.1" },
};

const SVG = `<svg xmlns="http://www.w3.org/2000/svg">
  <g class="kul-card" data-person-id="giuseppe" data-kind="canonical" data-gender="male"></g>
</svg>`;

const PERSON_DETAIL = {
    kind: "person" as const,
    person: { id: "giuseppe", name: "Giuseppe", gender: "male" },
    parents: [],
    marriages: [],
    children: [],
};

/** A module that answers every target with one person detail. */
function okModule(): {
    module: EngineModule;
    calls: {
        files: WasmInputFile[];
        manifest: Manifest;
        targets: DetailTarget[];
    }[];
    resolveCalls: {
        files: WasmInputFile[];
        manifest: Manifest;
        anchors: [string, string];
    }[];
    filterCalls: {
        files: WasmInputFile[];
        manifest: Manifest;
        query: Query;
    }[];
} {
    const calls: {
        files: WasmInputFile[];
        manifest: Manifest;
        targets: DetailTarget[];
    }[] = [];
    const resolveCalls: {
        files: WasmInputFile[];
        manifest: Manifest;
        anchors: [string, string];
    }[] = [];
    const filterCalls: {
        files: WasmInputFile[];
        manifest: Manifest;
        query: Query;
    }[] = [];
    return {
        module: {
            queryDetail(files, manifest, targets) {
                calls.push({ files, manifest, targets });
                return {
                    ok: true,
                    result: targets.map(() => PERSON_DETAIL),
                } as QueryEnvelope<DetailLookupResult>;
            },
            queryKin() {
                return { ok: true, result: { kind: "count", count: 0 } };
            },
            runQuery(files, manifest, query) {
                filterCalls.push({ files, manifest, query });
                return { ok: true, result: { kind: "personIds", personIds: [] } };
            },
            queryResolve(files, manifest, xId, yId) {
                resolveCalls.push({ files, manifest, anchors: [xId, yId] });
                return { ok: true, result: { relationships: [] } };
            },
        },
        calls,
        resolveCalls,
        filterCalls,
    };
}

/** A module standing in for a project that fails its checks (ADR-0009). */
function failingModule(): EngineModule {
    return {
        queryKin() {
            return failure();
        },
        runQuery() {
            return failure();
        },
        queryDetail() {
            return failure();
        },
        queryResolve() {
            return failure();
        },
    };

    function failure() {
        return {
            ok: false,
            diagnostics: [
                {
                    code: "KUL-R03",
                    severity: "error",
                    message: "person `giuseppe` is missing required field `gender`",
                    related: [],
                },
                {
                    code: "KUL-W01",
                    severity: "warning",
                    message: "unused something",
                    related: [],
                },
            ],
        };
    }
}

const adapter: HostAdapter = { onRevealRequest: () => {} };

function mount(load: () => Promise<EngineModule>) {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const handle = mountPreview(container, adapter, {
        engine: createQueryEngine(SOURCE, load),
    });
    return { container, handle };
}

beforeEach(() => {
    document.body.innerHTML = "";
});

afterEach(() => {
    document.body.innerHTML = "";
});

describe("a query runs end to end against the rendered project", () => {
    it("posts the render's own files + manifest to the engine and returns the envelope", async () => {
        const { module, calls } = okModule();
        const { container, handle } = mount(async () => module);
        installVscodeInboundBridge(handle);

        // Drive it the way the extension does.
        window.dispatchEvent(
            new MessageEvent("message", {
                data: { type: "render", svg: SVG, project: PROJECT },
            }),
        );
        expect(container.querySelector("svg")).not.toBeNull();

        const envelope = await handle.queryDetail([
            { kind: "person", id: "giuseppe" },
        ]);
        expect(envelope).not.toBeNull();
        expect(isQueryOk(envelope!)).toBe(true);
        expect(calls).toEqual([
            {
                files: PROJECT.files,
                manifest: PROJECT.manifest,
                targets: [{ kind: "person", id: "giuseppe" }],
            },
        ]);
    });

    it("queries the project of the latest render, not the first", async () => {
        const { module, calls } = okModule();
        const { handle } = mount(async () => module);
        const second: ProjectSnapshot = {
            files: [{ name: "family.kul", source: "# edited\n" }],
            manifest: { kul: "0.1" },
        };
        handle.render(SVG, PROJECT);
        handle.render(SVG, second);
        await handle.queryDetail([{ kind: "person", id: "giuseppe" }]);
        expect(calls[0].files).toEqual(second.files);
    });

    it("resolves two anchors against the same project, ego first", async () => {
        const { module, resolveCalls } = okModule();
        const { handle } = mount(async () => module);
        handle.render(SVG, PROJECT);
        const envelope = await handle.queryResolve("giuseppe", "maria");
        expect(envelope).not.toBeNull();
        expect(isQueryOk(envelope!)).toBe(true);
        // Ego first: the descriptors come back relative to the first anchor,
        // which is what makes the lens read "as the selected person sees it".
        expect(resolveCalls).toEqual([
            {
                files: PROJECT.files,
                manifest: PROJECT.manifest,
                anchors: ["giuseppe", "maria"],
            },
        ]);
    });

    it("evaluates a filter's Query against the same project, through the same policy", async () => {
        const { module, filterCalls } = okModule();
        const { handle } = mount(async () => module);
        handle.render(SVG, PROJECT);
        const query: Query = {
            source: { kind: "allPersons" },
            where: [{ op: "eq", field: "family", value: "Rossi" }],
            mode: "certain",
            projection: "members",
        };
        const envelope = await handle.runQuery(query);
        expect(envelope).not.toBeNull();
        expect(isQueryOk(envelope!)).toBe(true);
        expect(filterCalls).toEqual([
            { files: PROJECT.files, manifest: PROJECT.manifest, query },
        ]);
    });

    it("answers null — never a stale answer — when no project has been rendered", async () => {
        const load = vi.fn(async () => okModule().module);
        const { handle } = mount(load);
        expect(await handle.queryDetail([{ kind: "person", id: "x" }])).toBeNull();
        expect(load).not.toHaveBeenCalled();
    });
});

describe("opening a preview loads no WASM", () => {
    it("does not load the module on mount, on render, or on a highlight", () => {
        const load = vi.fn(async () => okModule().module);
        const { handle } = mount(load);
        installVscodeInboundBridge(handle);
        window.dispatchEvent(
            new MessageEvent("message", {
                data: { type: "render", svg: SVG, project: PROJECT },
            }),
        );
        window.dispatchEvent(
            new MessageEvent("message", {
                data: { type: "highlightEntity", id: "giuseppe", kind: "person" },
            }),
        );
        expect(load).not.toHaveBeenCalled();
    });

    it("loads the module on the first query and not again on the second", async () => {
        const load = vi.fn(async () => okModule().module);
        const { handle } = mount(load);
        handle.render(SVG, PROJECT);
        expect(load).not.toHaveBeenCalled();
        await handle.queryDetail([{ kind: "person", id: "giuseppe" }]);
        expect(load).toHaveBeenCalledTimes(1);
        await handle.queryDetail([{ kind: "person", id: "giuseppe" }]);
        expect(load).toHaveBeenCalledTimes(1);
    });
});

describe("a project with diagnostics answers with the error arm", () => {
    it("surfaces the arm's error rows in the existing popover", async () => {
        const { container, handle } = mount(async () => failingModule());
        handle.render(SVG, PROJECT);
        await handle.queryDetail([{ kind: "person", id: "giuseppe" }]);

        const button = container.querySelector("#kul-error-button") as HTMLElement;
        expect(button.hidden).toBe(false);
        expect(container.querySelector(".kul-error-count")?.textContent).toBe("1");
        const popover = container.querySelector("#kul-error-popover") as HTMLElement;
        expect(popover.innerHTML).toContain("KUL-R03");
        expect(popover.innerHTML).toContain("missing required field");
        // The popover is the error surface; warnings are not errors.
        expect(popover.innerHTML).not.toContain("KUL-W01");
    });

    it("returns the error arm rather than a partial answer", async () => {
        const { handle } = mount(async () => failingModule());
        handle.render(SVG, PROJECT);
        const envelope = await handle.queryDetail([
            { kind: "person", id: "giuseppe" },
        ]);
        expect(envelope).not.toBeNull();
        expect(isQueryOk(envelope!)).toBe(false);
    });

    it("marks the picture stale so no answer reads as current", async () => {
        const { container, handle } = mount(async () => failingModule());
        handle.render(SVG, PROJECT);
        await handle.queryDetail([{ kind: "person", id: "giuseppe" }]);
        expect(
            container.querySelector("svg")?.classList.contains("kul-render-stale"),
        ).toBe(true);
    });
});

describe("a failed module load", () => {
    it("surfaces as a transport row in the popover and answers null", async () => {
        const { container, handle } = mount(async () => {
            throw new Error("404 fetching kul_wasm.js");
        });
        handle.render(SVG, PROJECT);
        const envelope = await handle.queryDetail([
            { kind: "person", id: "giuseppe" },
        ]);
        expect(envelope).toBeNull();
        const popover = container.querySelector("#kul-error-popover") as HTMLElement;
        expect(popover.innerHTML).toContain("Kul query failed");
        expect(popover.innerHTML).toContain("404 fetching kul_wasm.js");
    });
});

describe("a preview mounted without an engine", () => {
    it("answers null instead of throwing", async () => {
        const container = document.createElement("div");
        document.body.appendChild(container);
        const handle = mountPreview(container, adapter);
        handle.render(SVG, PROJECT);
        expect(await handle.queryDetail([{ kind: "person", id: "x" }])).toBeNull();
    });
});
