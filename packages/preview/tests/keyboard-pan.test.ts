import { afterEach, describe, expect, it, vi } from "vitest";

import { type KeyboardPanZoom, mountKeyboardPan } from "../src/controls.js";

function stubPanZoom(): KeyboardPanZoom {
    return {
        panBy: vi.fn(),
        zoomIn: vi.fn(),
        zoomOut: vi.fn(),
        reset: vi.fn(),
    };
}

let teardown: (() => void) | null = null;

function mount(): KeyboardPanZoom {
    const pz = stubPanZoom();
    teardown = mountKeyboardPan(() => pz);
    return pz;
}

/** Dispatch a bubbling keydown whose `target` is `from`, as the browser would. */
function press(from: Element, key: string): KeyboardEvent {
    const event = new KeyboardEvent("keydown", {
        key,
        bubbles: true,
        cancelable: true,
    });
    from.dispatchEvent(event);
    return event;
}

afterEach(() => {
    teardown?.();
    teardown = null;
    document.body.innerHTML = "";
});

describe("mountKeyboardPan target guard", () => {
    it("pans and zooms when the keystroke comes from the canvas", () => {
        const canvas = document.createElement("div");
        document.body.appendChild(canvas);
        const pz = mount();

        expect(press(canvas, "0").defaultPrevented).toBe(true);
        expect(press(canvas, "+").defaultPrevented).toBe(true);
        expect(press(canvas, "ArrowLeft").defaultPrevented).toBe(true);
        expect(pz.reset).toHaveBeenCalledTimes(1);
        expect(pz.zoomIn).toHaveBeenCalledTimes(1);
    });

    it("leaves the keystroke to the field when it comes from a text entry", () => {
        const input = document.createElement("input");
        const textarea = document.createElement("textarea");
        const editable = document.createElement("div");
        editable.setAttribute("contenteditable", "true");
        const nested = document.createElement("span");
        editable.appendChild(nested);
        document.body.append(input, textarea, editable);
        const pz = mount();

        for (const source of [input, textarea, editable, nested]) {
            for (const key of ["ArrowLeft", "ArrowUp", "+", "-", "0"]) {
                expect(press(source, key).defaultPrevented).toBe(false);
            }
        }
        expect(pz.panBy).not.toHaveBeenCalled();
        expect(pz.zoomIn).not.toHaveBeenCalled();
        expect(pz.zoomOut).not.toHaveBeenCalled();
        expect(pz.reset).not.toHaveBeenCalled();
    });
});
