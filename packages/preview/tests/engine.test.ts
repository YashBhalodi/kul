// The query engine's loading behaviour. The loader is the one system boundary
// here — a network fetch plus a WebAssembly instantiation — so it is the one
// thing substituted; everything below the seam is the real code.

import { describe, expect, it, vi } from "vitest";

import type {
    DetailLookupResult,
    DetailTarget,
    Manifest,
    QueryEnvelope,
    WasmInputFile,
} from "../src/engine-wire.js";
import {
    type EngineModule,
    type ProjectSnapshot,
    createQueryEngine,
} from "../src/engine.js";

const SOURCE = {
    moduleUri: "https://host.example/media/preview/wasm/kul_wasm.js",
    wasmUri: "https://host.example/media/preview/wasm/kul_wasm_bg.wasm",
};

const PROJECT: ProjectSnapshot = {
    files: [{ name: "a.kul", source: 'person a name:"A" gender:male\n' }],
    manifest: { kul: "0.1" },
};

function fakeModule() {
    const calls: {
        files: WasmInputFile[];
        manifest: Manifest;
        targets: DetailTarget[];
    }[] = [];
    const module: EngineModule = {
        queryDetail(files, manifest, targets) {
            calls.push({ files, manifest, targets });
            return { ok: true, result: [null] } as QueryEnvelope<DetailLookupResult>;
        },
        queryKin() {
            return { ok: true, result: { kind: "count", count: 0 } };
        },
    };
    return { module, calls };
}

describe("createQueryEngine laziness", () => {
    it("loads nothing until the first query", () => {
        const load = vi.fn(async () => fakeModule().module);
        const engine = createQueryEngine(SOURCE, load);
        expect(load).not.toHaveBeenCalled();
        expect(engine.isLoaded).toBe(false);
    });

    it("loads the module exactly once across many queries", async () => {
        const load = vi.fn(async () => fakeModule().module);
        const engine = createQueryEngine(SOURCE, load);
        await engine.queryDetail(PROJECT, [{ kind: "person", id: "a" }]);
        await engine.queryDetail(PROJECT, [{ kind: "person", id: "a" }]);
        expect(load).toHaveBeenCalledTimes(1);
        expect(engine.isLoaded).toBe(true);
    });

    it("shares one in-flight load between concurrent first queries", async () => {
        let release: (m: EngineModule) => void = () => {};
        const load = vi.fn(
            () => new Promise<EngineModule>((resolve) => (release = resolve)),
        );
        const engine = createQueryEngine(SOURCE, load);
        const both = Promise.all([
            engine.queryDetail(PROJECT, []),
            engine.queryDetail(PROJECT, []),
        ]);
        release(fakeModule().module);
        await both;
        expect(load).toHaveBeenCalledTimes(1);
    });

    it("hands the loader the host-supplied URIs unchanged", async () => {
        const load = vi.fn(async () => fakeModule().module);
        const engine = createQueryEngine(SOURCE, load);
        await engine.queryDetail(PROJECT, []);
        expect(load).toHaveBeenCalledWith(SOURCE);
    });

    it("retries the load after a failed one instead of staying poisoned", async () => {
        const { module } = fakeModule();
        const load = vi
            .fn<() => Promise<EngineModule>>()
            .mockRejectedValueOnce(new Error("404"))
            .mockResolvedValue(module);
        const engine = createQueryEngine(SOURCE, load);
        await expect(engine.queryDetail(PROJECT, [])).rejects.toThrow("404");
        await expect(engine.queryDetail(PROJECT, [])).resolves.toEqual({
            ok: true,
            result: [null],
        });
        expect(load).toHaveBeenCalledTimes(2);
    });
});

describe("createQueryEngine queryDetail", () => {
    it("passes the project's files and manifest with every call (stateless surface)", async () => {
        const { module, calls } = fakeModule();
        const engine = createQueryEngine(SOURCE, async () => module);
        const targets: DetailTarget[] = [
            { kind: "person", id: "a" },
            { kind: "adoption", childId: "c", marriageId: "m" },
        ];
        await engine.queryDetail(PROJECT, targets);
        expect(calls).toEqual([
            { files: PROJECT.files, manifest: PROJECT.manifest, targets },
        ]);
    });

    it("returns the envelope the engine produced", async () => {
        const { module } = fakeModule();
        const engine = createQueryEngine(SOURCE, async () => module);
        const envelope = await engine.queryDetail(PROJECT, []);
        expect(envelope).toEqual({ ok: true, result: [null] });
    });
});
