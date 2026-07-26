import { afterEach, describe, expect, it, vi } from "vitest";

import { installVscodeInboundBridge } from "../src/adapter-vscode.js";
import type { ProjectSnapshot } from "../src/engine.js";
import { createLocaleController } from "../src/locale.js";
import type { EntityRef, ErrorRow, PreviewHandle } from "../src/types.js";

function fakeHandle() {
    const render = vi.fn<(svg: string, project?: ProjectSnapshot | null) => void>();
    const showErrors = vi.fn<(errors: ErrorRow[]) => void>();
    const highlightEntity = vi.fn<(ref: EntityRef | null) => void>();
    const queryDetail = vi.fn(async () => null);
    const queryKin = vi.fn(async () => null);
    const runQuery = vi.fn(async () => null);
    const queryResolve = vi.fn(async () => null);
    const dispose = vi.fn<() => void>();
    const handle: PreviewHandle = {
        render,
        showErrors,
        highlightEntity,
        queryDetail,
        queryKin,
        runQuery,
        queryResolve,
        // The inbound bridge never touches the locale; a detached controller
        // satisfies the handle without pretending to be one of its channels.
        locale: createLocaleController(null),
        dispose,
    };
    return { handle, render, showErrors, highlightEntity, dispose };
}

const PROJECT: ProjectSnapshot = {
    files: [{ name: "a.kul", source: 'person a name:"A" gender:male\n' }],
    manifest: { kul: "0.1" },
};

// Drive the bridge the way VSCode does: a `window.message` event whose `data`
// is the wire payload.
function post(data: unknown): void {
    window.dispatchEvent(new MessageEvent("message", { data }));
}

let teardown: (() => void) | undefined;

afterEach(() => {
    teardown?.();
    teardown = undefined;
});

describe("installVscodeInboundBridge — render", () => {
    it("renders a valid svg string", () => {
        const { handle, render } = fakeHandle();
        teardown = installVscodeInboundBridge(handle);
        post({ type: "render", svg: "<svg/>" });
        expect(render).toHaveBeenCalledWith("<svg/>", null);
    });

    it("ignores a render message whose svg is not a string", () => {
        const { handle, render } = fakeHandle();
        teardown = installVscodeInboundBridge(handle);
        post({ type: "render", svg: 42 });
        post({ type: "render" });
        expect(render).not.toHaveBeenCalled();
    });

    it("carries a well-formed project snapshot through to the handle", () => {
        const { handle, render } = fakeHandle();
        teardown = installVscodeInboundBridge(handle);
        post({ type: "render", svg: "<svg/>", project: PROJECT });
        expect(render).toHaveBeenCalledWith("<svg/>", PROJECT);
    });

    it("drops a malformed project snapshot without dropping the render", () => {
        const { handle, render } = fakeHandle();
        teardown = installVscodeInboundBridge(handle);
        post({
            type: "render",
            svg: "<svg/>",
            project: { files: [{ name: "a.kul" }], manifest: { kul: "0.1" } },
        });
        post({ type: "render", svg: "<svg/>", project: { files: [] } });
        expect(render).toHaveBeenNthCalledWith(1, "<svg/>", null);
        expect(render).toHaveBeenNthCalledWith(2, "<svg/>", null);
    });
});

describe("installVscodeInboundBridge — renderError", () => {
    it("forwards an errors array", () => {
        const { handle, showErrors } = fakeHandle();
        teardown = installVscodeInboundBridge(handle);
        const errors: ErrorRow[] = [{ message: "boom" }];
        post({ type: "renderError", errors });
        expect(showErrors).toHaveBeenCalledWith(errors);
    });

    it("degrades a non-array errors field to an empty list", () => {
        const { handle, showErrors } = fakeHandle();
        teardown = installVscodeInboundBridge(handle);
        post({ type: "renderError", errors: "nope" });
        expect(showErrors).toHaveBeenCalledWith([]);
    });
});

describe("installVscodeInboundBridge — highlightEntity", () => {
    it("highlights a valid id + kind", () => {
        const { handle, highlightEntity } = fakeHandle();
        teardown = installVscodeInboundBridge(handle);
        post({ type: "highlightEntity", id: "p1", kind: "person" });
        expect(highlightEntity).toHaveBeenCalledWith({ id: "p1", kind: "person" });
    });

    it("clears the highlight when id is null", () => {
        const { handle, highlightEntity } = fakeHandle();
        teardown = installVscodeInboundBridge(handle);
        post({ type: "highlightEntity", id: null });
        expect(highlightEntity).toHaveBeenCalledWith(null);
    });

    it("fails safe (clears) when kind has drifted off the union", () => {
        const { handle, highlightEntity } = fakeHandle();
        teardown = installVscodeInboundBridge(handle);
        post({ type: "highlightEntity", id: "p1", kind: "spouse" });
        expect(highlightEntity).toHaveBeenCalledWith(null);
        expect(highlightEntity).not.toHaveBeenCalledWith(
            expect.objectContaining({ kind: "spouse" }),
        );
    });

    it("fails safe (clears) when kind is missing", () => {
        const { handle, highlightEntity } = fakeHandle();
        teardown = installVscodeInboundBridge(handle);
        post({ type: "highlightEntity", id: "p1" });
        expect(highlightEntity).toHaveBeenCalledWith(null);
    });
});

describe("installVscodeInboundBridge — junk", () => {
    it("ignores null / non-object / unknown-type payloads", () => {
        const { handle, render, showErrors, highlightEntity } = fakeHandle();
        teardown = installVscodeInboundBridge(handle);
        post(null);
        post("render");
        post(42);
        post({ type: "unknown" });
        expect(render).not.toHaveBeenCalled();
        expect(showErrors).not.toHaveBeenCalled();
        expect(highlightEntity).not.toHaveBeenCalled();
    });

    it("stops dispatching after teardown", () => {
        const { handle, render } = fakeHandle();
        const stop = installVscodeInboundBridge(handle);
        stop();
        post({ type: "render", svg: "<svg/>" });
        expect(render).not.toHaveBeenCalled();
    });
});
