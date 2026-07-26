import type { LocaleStore } from "./locale-store.js";
import { createMemoryLocaleStore } from "./locale-store.js";
import { PACKS, packFor, phrase } from "./phrasing/index.js";
import type { LanguagePack, RelationshipDescriptor } from "./phrasing/index.js";

/**
 * Transliterated language names, for the toggle's hover title.
 *
 * Chrome copy, deliberately: the button *reads* in the language it offers
 * (ગુજરાતી, not "Gujarati"), because that is the label a reader scans for,
 * and hovering gives the transliteration for a reader who does not know the
 * script yet. The words around it — "Kinship terms in…" — stay English;
 * chrome i18n is a separate question (ADR-0022's stance), so this map is a UI
 * string table and not term data. A pack with no entry falls back to its own
 * label.
 */
const ROMANIZED: Record<string, string> = {
    en: "English",
    gu: "Gujarati",
};

/**
 * The locale toggle: one button in the **flow** region (ADR-0038 — chrome
 * joins a region by being appended, and never computes an inset).
 *
 * Per ADR-0036 the query chrome carries no `role`, no `aria-*` and no
 * `:focus-visible`. That is a stated non-goal for what this epic newly
 * builds, and it is deliberately unlike the pan/zoom cluster next to it —
 * which keeps every attribute it has.
 */
export const LOCALE_TOGGLE_HTML = `<div id="kul-locale" class="kul-locale"><button type="button" id="kul-locale-toggle" class="kul-locale-btn" data-action="cycle-locale"></button></div>`;

/**
 * The reader's language choice, and everything phrased against it.
 *
 * A toggle rather than a setting because the compare gesture is the point:
 * flipping one English *brother-in-law* against the six Gujarati terms it
 * merges is genuinely useful, and it keeps phrasing to **one language at a
 * time** rather than doubling every label (ADR-0033).
 */
export interface LocaleController {
    /** The pack everything currently phrases against. */
    pack(): LanguagePack;
    /** Switch to a registered pack. Unknown codes are ignored. */
    select(code: string): void;
    /** Advance to the next registered pack, wrapping. What the button does. */
    next(): void;
    /** Called on every change, with the new pack. Returns an unsubscribe. */
    subscribe(listener: (pack: LanguagePack) => void): () => void;
    /**
     * Phrase one relationship into one element, and **keep** it phrased: the
     * element re-renders on every locale change, so flipping the toggle
     * re-phrases everything on screen without any caller re-running a query.
     *
     * The element carries its own `lang` — per-element rather than
     * per-document, because the chrome around it stays English while the
     * phrase inside it needs `lang="gu"` for the browser to pick a font that
     * can render Gujarati. It also carries the whole phrasing result in a form
     * chrome can act on without string-sniffing (ADR-0033): `data-phrase-kind`
     * separates a crisp *kākā* from a chain, the `--kul-phrase-hops` custom
     * property gives CSS the chain's length, and `title` carries the Latin
     * gloss for a pack that supplies one.
     *
     * Returns an unbind.
     */
    bind(element: HTMLElement, descriptor: RelationshipDescriptor): () => void;
}

export function createLocaleController(
    host: HTMLElement | null,
    store: LocaleStore = createMemoryLocaleStore(),
): LocaleController {
    const fallback = PACKS[0]!;
    let current = packFor(store.read() ?? "") ?? fallback;

    const listeners = new Set<(pack: LanguagePack) => void>();
    const bound = new Map<HTMLElement, RelationshipDescriptor>();
    const button = host?.querySelector("#kul-locale-toggle") as HTMLButtonElement | null;

    function romanized(pack: LanguagePack): string {
        return ROMANIZED[pack.code] ?? pack.label;
    }

    function render(element: HTMLElement, descriptor: RelationshipDescriptor): void {
        const result = phrase(descriptor, current);
        element.textContent = result.text;
        element.setAttribute("lang", current.code);
        element.dataset.phraseKind = result.kind;
        // `hopCount` reaches CSS as a custom property so a slot can *size* for
        // a five-hop chain without measuring or parsing the text — the reason
        // ADR-0033 put the number on the result at all. A data attribute would
        // only be selectable value by value.
        element.style.setProperty("--kul-phrase-hops", String(result.hopCount));
        // The fuller gloss on hover: the same phrase in Latin script, for a
        // pack that supplies one. Absent — attribute removed, not emptied —
        // when the pack does not, so English shows no tooltip at all rather
        // than one repeating the word underneath it (ADR-0044).
        if (result.translit === undefined) {
            element.removeAttribute("title");
        } else {
            element.title = result.translit;
        }
    }

    function paint(): void {
        if (button) {
            button.textContent = current.label;
            // The button is itself a phrase site: ગુજરાતી needs the same font
            // selection any kinship term does.
            button.setAttribute("lang", current.code);
            button.title = `Kinship terms in ${romanized(current)}. Click to switch.`;
        }
        for (const [element, descriptor] of bound) {
            render(element, descriptor);
        }
    }

    function apply(pack: LanguagePack): void {
        if (pack.code === current.code) {
            return;
        }
        current = pack;
        store.write(pack.code);
        paint();
        for (const listener of listeners) {
            listener(pack);
        }
    }

    function advance(): void {
        const at = PACKS.findIndex((pack) => pack.code === current.code);
        apply(PACKS[(at + 1) % PACKS.length]!);
    }

    button?.addEventListener("click", advance);

    paint();

    return {
        pack: () => current,
        select(code) {
            const pack = packFor(code);
            if (pack) {
                apply(pack);
            }
        },
        next: advance,
        subscribe(listener) {
            listeners.add(listener);
            return () => listeners.delete(listener);
        },
        bind(element, descriptor) {
            bound.set(element, descriptor);
            render(element, descriptor);
            return () => bound.delete(element);
        },
    };
}
