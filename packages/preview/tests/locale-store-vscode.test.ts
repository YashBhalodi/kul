import { afterEach, describe, expect, it, vi } from "vitest";

/**
 * The VSCode-backed locale store (ADR-0033). `acquireVsCodeApi` may be called
 * once per webview load, so the adapter caches the handle at module scope —
 * which means each case here needs a fresh module instance around a fresh
 * fake.
 */
async function freshAdapter(api: unknown | null) {
    vi.resetModules();
    if (api === null) {
        delete (window as unknown as Record<string, unknown>).acquireVsCodeApi;
    } else {
        (window as unknown as Record<string, unknown>).acquireVsCodeApi = () => api;
    }
    return import("../src/adapter-vscode.js");
}

function fakeApi(initial: unknown = undefined) {
    let state = initial;
    return {
        postMessage: vi.fn(),
        getState: vi.fn(() => state),
        setState: vi.fn((next: unknown) => {
            state = next;
        }),
        acquisitions: 0,
    };
}

afterEach(() => {
    delete (window as unknown as Record<string, unknown>).acquireVsCodeApi;
});

describe("createVscodeLocaleStore", () => {
    it("round-trips the choice through webview-local state", async () => {
        const api = fakeApi();
        const { createVscodeLocaleStore } = await freshAdapter(api);
        const store = createVscodeLocaleStore();

        expect(store.read()).toBeNull();
        store.write("gu");
        expect(store.read()).toBe("gu");
    });

    it("merges rather than replaces, so other webview state survives", async () => {
        const api = fakeApi({ scrollTop: 42 });
        const { createVscodeLocaleStore } = await freshAdapter(api);
        createVscodeLocaleStore().write("gu");
        expect(api.setState).toHaveBeenCalledWith({ scrollTop: 42, locale: "gu" });
    });

    it("reads null from state that holds no locale, or holds a non-string", async () => {
        const api = fakeApi({ locale: 7 });
        const { createVscodeLocaleStore } = await freshAdapter(api);
        expect(createVscodeLocaleStore().read()).toBeNull();
    });

    it("degrades to a no-op outside a webview", async () => {
        const { createVscodeLocaleStore } = await freshAdapter(null);
        const store = createVscodeLocaleStore();
        expect(store.read()).toBeNull();
        expect(() => store.write("gu")).not.toThrow();
    });

    it("acquires the webview API once, however many consumers ask", async () => {
        // A second `acquireVsCodeApi()` throws in a real webview, and there are
        // two consumers now: the reveal channel and the store.
        const api = fakeApi();
        vi.resetModules();
        const acquire = vi.fn(() => api);
        (window as unknown as Record<string, unknown>).acquireVsCodeApi = acquire;
        const { createVscodeAdapter, createVscodeLocaleStore } = await import(
            "../src/adapter-vscode.js"
        );

        createVscodeAdapter().onRevealRequest({ kind: "entity", id: "p1" });
        createVscodeLocaleStore().write("gu");
        createVscodeLocaleStore().read();

        expect(acquire).toHaveBeenCalledTimes(1);
        expect(api.postMessage).toHaveBeenCalledTimes(1);
    });
});
