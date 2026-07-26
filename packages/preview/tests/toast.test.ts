// The quiet toast: one message at a time, gone on its own.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { TOAST_DURATION_MS, createToaster } from "../src/toast.js";

function region(): HTMLElement {
    const el = document.createElement("div");
    document.body.appendChild(el);
    return el;
}

beforeEach(() => {
    vi.useFakeTimers();
    document.body.innerHTML = "";
});

afterEach(() => {
    vi.useRealTimers();
    document.body.innerHTML = "";
});

describe("createToaster", () => {
    it("shows one message and takes it down on its own", () => {
        const host = region();
        createToaster(host).show("nothing is guessed");
        expect(host.querySelector(".kul-toast")?.textContent).toBe(
            "nothing is guessed",
        );
        vi.advanceTimersByTime(TOAST_DURATION_MS);
        expect(host.querySelector(".kul-toast")).toBeNull();
    });

    it("replaces rather than stacks, so two absences never read as a log", () => {
        const host = region();
        const toaster = createToaster(host);
        toaster.show("first");
        toaster.show("second");
        expect(host.querySelectorAll(".kul-toast")).toHaveLength(1);
        expect(host.querySelector(".kul-toast")?.textContent).toBe("second");
    });

    it("restarts the clock on each message", () => {
        const host = region();
        const toaster = createToaster(host);
        toaster.show("first");
        vi.advanceTimersByTime(TOAST_DURATION_MS - 1);
        toaster.show("second");
        vi.advanceTimersByTime(TOAST_DURATION_MS - 1);
        expect(host.querySelector(".kul-toast")?.textContent).toBe("second");
    });

    it("sets its text, so an author-written name can never be markup", () => {
        const host = region();
        createToaster(host).show("<script>x</script>");
        expect(host.querySelector("script")).toBeNull();
        expect(host.querySelector(".kul-toast")?.textContent).toBe(
            "<script>x</script>",
        );
    });

    it("is a working no-op without a notify region", () => {
        const toaster = createToaster(null);
        expect(() => {
            toaster.show("nothing to show it in");
            toaster.clear();
            toaster.dispose();
        }).not.toThrow();
    });
});
