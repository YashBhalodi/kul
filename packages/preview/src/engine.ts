// The query engine, as the webview reaches it.
//
// ADR-0034 put the engine *in* the webview rather than behind an LSP request,
// so a query is a local function call. ADR-0040 decided where the module comes
// from: the host builds the `wasm-pack --target web` output as an asset, ships
// it beside the webview bundle, and hands this module two URIs. Nothing here
// imports `@kullang/wasm` — that is the point of the {@link EngineSource}
// indirection, and it is what keeps `@kullang/preview` host-agnostic.
//
// The module loads on the **first query**, never at mount. Opening a preview
// must cost no WASM at all; module load is 3.6 ms (#292), so the first query
// pays it and nothing after does.

import type {
    DetailLookupResult,
    DetailTarget,
    Manifest,
    Query,
    QueryEnvelope,
    QueryResult,
    WasmInputFile,
} from "./engine-wire.js";

/** Where the host put the engine. Both are webview-resource URIs. */
export interface EngineSource {
    /** The `wasm-pack --target web` ESM glue module. */
    moduleUri: string;
    /** The `.wasm` binary that glue module fetches and instantiates. */
    wasmUri: string;
}

/**
 * The project a query runs against. Every WASM operation is stateless — it
 * takes `files + manifest` and re-checks — so the source travels with the
 * picture it produced (ADR-0034).
 */
export interface ProjectSnapshot {
    files: WasmInputFile[];
    manifest: Manifest;
}

/**
 * The slice of the `@kullang/wasm` surface the preview calls. Structural, so
 * the loaded module satisfies it without an import; a test double satisfies it
 * without a WASM build.
 */
export interface EngineModule {
    queryDetail(
        files: WasmInputFile[],
        manifest: Manifest,
        targets: DetailTarget[],
    ): QueryEnvelope<DetailLookupResult>;
    queryKin(
        files: WasmInputFile[],
        manifest: Manifest,
        query: Query,
    ): QueryEnvelope<QueryResult>;
}

/**
 * Fetches, instantiates and returns the engine module. This is the one system
 * boundary in the query path — a network fetch plus a WASM instantiation — and
 * therefore the one thing unit tests substitute (`CODING_STANDARDS.md`:
 * mock at system boundaries, never internal collaborators).
 */
export type EngineModuleLoader = (source: EngineSource) => Promise<EngineModule>;

/** Shape of the ESM module `wasm-pack --target web` emits. */
interface WasmPackWebModule extends EngineModule {
    default(init: { module_or_path: string }): Promise<unknown>;
}

/**
 * Default loader: dynamic-`import()` the glue module, then call its `init`
 * default export with the binary's URI.
 *
 * The dynamic import is deliberately not statically analysable — esbuild must
 * leave it alone rather than try to bundle a path that only exists on the
 * host's disk. This is also why the webview CSP needs `'wasm-unsafe-eval'` in
 * `script-src` and the host origin in `connect-src` (ADR-0034): the glue does
 * `fetch(url)` and then `WebAssembly.instantiate*`.
 */
export const loadEngineModule: EngineModuleLoader = async (source) => {
    const specifier = source.moduleUri;
    const mod = (await import(/* @vite-ignore */ specifier)) as WasmPackWebModule;
    // Pass the object form: wasm-bindgen's init warns about a bare string.
    await mod.default({ module_or_path: source.wasmUri });
    return mod;
};

/**
 * A loaded-on-demand handle onto the engine. One instance per preview; the
 * module is loaded at most once, and concurrent first queries share the single
 * in-flight load rather than racing two instantiations.
 */
export interface QueryEngine {
    /**
     * Batched detail lookup (ADR-0037): a list of targets in, one answer per
     * target out, in the order asked. `null` in a position means that target
     * named no entity; the envelope's error arm means the project failed its
     * checks (ADR-0009) and there is no partial answer.
     */
    queryDetail(
        project: ProjectSnapshot,
        targets: DetailTarget[],
    ): Promise<QueryEnvelope<DetailLookupResult>>;
    /**
     * Evaluate one kin-set {@link Query} — ADR-0025's single contract artifact,
     * built by the caller and evaluated only here. The `count` and `members`
     * projections are the same value one field apart, so the Explore-kin list
     * and the paint it drives share one call site.
     */
    queryKin(
        project: ProjectSnapshot,
        query: Query,
    ): Promise<QueryEnvelope<QueryResult>>;
    /** True once the module has finished loading. Never triggers a load. */
    readonly isLoaded: boolean;
}

export function createQueryEngine(
    source: EngineSource,
    load: EngineModuleLoader = loadEngineModule,
): QueryEngine {
    let pending: Promise<EngineModule> | null = null;
    let loaded: EngineModule | null = null;

    function moduleOnce(): Promise<EngineModule> {
        if (!pending) {
            pending = load(source).then(
                (mod) => {
                    loaded = mod;
                    return mod;
                },
                (err) => {
                    // A failed load must not poison the engine forever: drop
                    // the memo so a later query can retry.
                    pending = null;
                    throw err;
                },
            );
        }
        return pending;
    }

    return {
        async queryDetail(project, targets) {
            const mod = await moduleOnce();
            return mod.queryDetail(project.files, project.manifest, targets);
        },
        async queryKin(project, query) {
            const mod = await moduleOnce();
            return mod.queryKin(project.files, project.manifest, query);
        },
        get isLoaded() {
            return loaded !== null;
        },
    };
}
