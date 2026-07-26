import { afterEach, describe, expect, it, vi } from "vitest";

// svg-pan-zoom relies on SVG measurement APIs jsdom does not implement. Stub
// it out before any module imports the dep so the chrome can mount cleanly.
vi.mock("svg-pan-zoom", () => {
    const factory = vi.fn(() => ({
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
    }));
    return { default: factory };
});

import { CHROME_LANG, previewHtml } from "../src/html.js";
import { LOCALE_TOGGLE_HTML, createLocaleController } from "../src/locale.js";
import { createMemoryLocaleStore } from "../src/locale-store.js";
import type { LocaleStore } from "../src/locale-store.js";
import { mountPreview } from "../src/mount.js";
import type { RelationshipDescriptor } from "../src/phrasing/index.js";

/** A daughter's daughter: *dohitrī* in Gujarati, "granddaughter" in English. */
const GRANDDAUGHTER: RelationshipDescriptor = {
    egoId: "ego",
    alterId: "alter",
    egoGender: "female",
    alterGender: "female",
    classification: { kind: "lineal", role: "descendant", generations: 2 },
    edgeNature: "blood",
    affinity: "blood",
    sharing: "notApplicable",
    side: "notApplicable",
    seniority: "unknown",
    apexSeniority: "notApplicable",
    path: [
        { step: "down", to: "p1", gender: "female", edge: "bio" },
        { step: "down", to: "p2", gender: "female", edge: "bio" },
    ],
};

function host(): HTMLElement {
    const element = document.createElement("div");
    element.innerHTML = LOCALE_TOGGLE_HTML;
    document.body.appendChild(element);
    return element.querySelector("#kul-locale") as HTMLElement;
}

function click(container: HTMLElement, selector: string): void {
    (container.querySelector(selector) as HTMLElement).dispatchEvent(
        new MouseEvent("click", { bubbles: true }),
    );
}

afterEach(() => {
    document.body.innerHTML = "";
});

describe("the locale toggle", () => {
    it("opens in the first registered pack and labels itself with its endonym", () => {
        const element = host();
        const locale = createLocaleController(element, createMemoryLocaleStore());
        const button = element.querySelector("#kul-locale-toggle") as HTMLButtonElement;
        expect(locale.pack().code).toBe("en");
        expect(button.textContent).toBe("English");
    });

    it("cycles the language on click, and tags the label with its own lang", () => {
        const element = host();
        const locale = createLocaleController(element, createMemoryLocaleStore());
        const button = element.querySelector("#kul-locale-toggle") as HTMLButtonElement;

        button.dispatchEvent(new MouseEvent("click", { bubbles: true }));

        expect(locale.pack().code).toBe("gu");
        expect(button.textContent).toBe("ગુજરાતી");
        // Per-element `lang`, not per-document: the chrome around it stays
        // English, so the document cannot carry this.
        expect(button.getAttribute("lang")).toBe("gu");
    });

    it("offers the transliterated language name on hover, in English chrome copy", () => {
        const element = host();
        const locale = createLocaleController(element, createMemoryLocaleStore());
        const button = element.querySelector("#kul-locale-toggle") as HTMLButtonElement;
        expect(button.title).toContain("English");
        locale.select("gu");
        expect(button.title).toContain("Gujarati");
    });

    it("carries no role, aria-* or tabindex — the stated non-goal, in the markup", () => {
        const element = host();
        createLocaleController(element, createMemoryLocaleStore());
        const button = element.querySelector("#kul-locale-toggle") as HTMLButtonElement;
        const attributes = button.getAttributeNames();
        expect(attributes.filter((name) => name.startsWith("aria-"))).toEqual([]);
        expect(attributes).not.toContain("role");
        expect(attributes).not.toContain("tabindex");
    });

    it("re-phrases every bound element when the language flips", () => {
        const element = host();
        const locale = createLocaleController(element, createMemoryLocaleStore());
        const slot = document.createElement("span");
        const other = document.createElement("span");
        locale.bind(slot, GRANDDAUGHTER);
        locale.bind(other, GRANDDAUGHTER);

        expect(slot.textContent).toBe("granddaughter");
        expect(slot.getAttribute("lang")).toBe("en");
        expect(slot.dataset.phraseKind).toBe("lexical");

        locale.next();

        // Both re-render, and the term is the one the daughter's line takes —
        // a distinction English does not make at all.
        for (const bound of [slot, other]) {
            expect(bound.textContent).toBe("દોહિત્રી");
            expect(bound.getAttribute("lang")).toBe("gu");
        }
    });

    it("hands chrome the whole phrasing result, not only its text", () => {
        const element = host();
        const locale = createLocaleController(element, createMemoryLocaleStore());
        const slot = document.createElement("span");
        locale.bind(slot, GRANDDAUGHTER);

        // English: a lexical term, no chain, and no gloss — the word is
        // already the reading.
        expect(slot.dataset.phraseKind).toBe("lexical");
        expect(slot.style.getPropertyValue("--kul-phrase-hops")).toBe("0");
        expect(slot.hasAttribute("title")).toBe(false);

        locale.next();

        // Gujarati: the same cell is lexical too, and now hovering it reads.
        expect(slot.dataset.phraseKind).toBe("lexical");
        expect(slot.title).toBe("dohitrī");
    });

    it("withdraws the gloss when the language flipped to has none", () => {
        const locale = createLocaleController(host(), createMemoryLocaleStore());
        const slot = document.createElement("span");
        locale.select("gu");
        locale.bind(slot, GRANDDAUGHTER);
        expect(slot.title).toBe("dohitrī");
        locale.select("en");
        // Removed, not emptied: an empty `title` is still a tooltip.
        expect(slot.hasAttribute("title")).toBe(false);
    });

    it("stops re-phrasing an unbound element", () => {
        const locale = createLocaleController(host(), createMemoryLocaleStore());
        const slot = document.createElement("span");
        const unbind = locale.bind(slot, GRANDDAUGHTER);
        unbind();
        locale.next();
        expect(slot.textContent).toBe("granddaughter");
    });

    it("notifies subscribers once per change, and not on a no-op select", () => {
        const locale = createLocaleController(host(), createMemoryLocaleStore());
        const seen: string[] = [];
        const stop = locale.subscribe((pack) => seen.push(pack.code));
        locale.select("en");
        locale.select("gu");
        locale.select("nope");
        stop();
        locale.select("en");
        expect(seen).toEqual(["gu"]);
    });
});

describe("the locale store", () => {
    it("restores the persisted choice, so it survives a preview reopen", () => {
        const store = createMemoryLocaleStore();
        const first = createLocaleController(host(), store);
        first.select("gu");

        // A reopen is a fresh controller over the same store.
        document.body.innerHTML = "";
        const second = createLocaleController(host(), store);
        expect(second.pack().code).toBe("gu");
    });

    it("ignores a persisted code no pack is registered for", () => {
        const store = createMemoryLocaleStore("klingon");
        expect(createLocaleController(host(), store).pack().code).toBe("en");
    });

    it("writes through on every change", () => {
        const store = createMemoryLocaleStore();
        createLocaleController(host(), store).select("gu");
        expect(store.read()).toBe("gu");
    });
});

describe("the mounted chrome", () => {
    it("puts the toggle in the flow region, not over the canvas", () => {
        const container = document.createElement("div");
        document.body.appendChild(container);
        mountPreview(container, { onRevealRequest: vi.fn() });
        expect(container.querySelector("#kul-locale")?.parentElement).toBe(
            container.querySelector(".kul-region-flow"),
        );
    });

    it("threads a host-supplied store through, and exposes the choice on the handle", () => {
        const container = document.createElement("div");
        document.body.appendChild(container);
        const written: string[] = [];
        const store: LocaleStore = {
            read: () => null,
            write: (code) => written.push(code),
        };
        const handle = mountPreview(container, { onRevealRequest: vi.fn() }, {
            localeStore: store,
        });

        click(container, "#kul-locale-toggle");

        expect(written).toEqual(["gu"]);
        expect(handle.locale.pack().code).toBe("gu");
    });
});

describe("the HTML shell", () => {
    it("keeps the document in the chrome's language", () => {
        // UI chrome stays English whatever the reader picks; the phrases are
        // what change language, and each carries its own `lang`.
        expect(CHROME_LANG).toBe("en");
        expect(
            previewHtml({
                themeStylesheetUri: "theme.css",
                applicationStylesheetUri: "app.css",
                scriptUri: "webview.js",
                cspSource: "vscode-resource:",
                nonce: "n".repeat(32),
            }),
        ).toContain('<html lang="en">');
    });
});
