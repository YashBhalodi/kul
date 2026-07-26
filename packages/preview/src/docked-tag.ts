// The docked tag — a whisper anchored under one entity's card.
//
// This is the *mechanism* behind the hover lens's relation pill and nothing
// else: the float-layer placement ADR-0042 carved for entity-anchored chrome,
// the measurement against a card's screen box, the horizontal viewport clamp,
// and the pill chrome itself. It decides nothing about **when** a whisper
// appears or **what** it says — no selection, no query, no words.
//
// It is a separate module because #302 is not its only consumer. PRD-0006 says
// the attribute filter's "can't say" reason whispers *in the lens's docked-tag
// grammar* ([#303](https://github.com/YashBhalodi/kul/issues/303)) — under a
// card, with **no selection anywhere**, on a trigger of its own. Calling that a
// shared grammar and then keeping it private to `hover-lens.ts` would leave
// #303 either editing that file or copying the placement arithmetic, and the
// copy is worse: two docked-tag grammars that drift apart (ADR-0044).
//
// The content model is deliberately a list of nodes. The caller builds its own
// chrome inside the tag — the lens builds a viewpoint dot and phrased terms,
// the filter builds a can't-say reason — and neither knows about the other's.
// Each also brings its own variant class, which is what tells two tags in one
// layer apart.
//
// ACCESSIBILITY. Per ADR-0036 this chrome carries no `role`, no `aria-*` and no
// `:focus-visible`, and a consumer must not add them.

/** Class carrying the docked-tag chrome and its placement. */
export const DOCKED_TAG_CLASS = "kul-docked-tag";

export interface DockedTagOptions {
    /**
     * The float layer itself (`#kul-region-float`), **not** its edge dock — the
     * placement for chrome that positions against an entity's screen box
     * (ADR-0042). `QuerySurfaceOptions.floatLayer` is where the chrome gets it.
     */
    layer: HTMLElement;
    /**
     * The element the tag docks under. Its box is **measured**, never assumed,
     * so a ghost card anchors as well as a canonical one.
     */
    anchor: Element;
    /**
     * The tag's content, in order. The mechanism supplies the box and the
     * placement and never a word; a consumer appends whatever it has to say.
     */
    content: ReadonlyArray<Node>;
    /**
     * Extra class on the tag, so a consumer can find and style its own variant
     * without the mechanism knowing the variants exist.
     */
    variant?: string;
}

export interface DockedTag {
    /** The tag element. Live in the DOM until {@link DockedTag.close}. */
    readonly element: HTMLElement;
    /** The element it is docked under. */
    readonly anchor: Element;
    /**
     * Re-measure the anchor and re-position. Cheap enough to call at
     * pointer-move frequency: one box read, and a style write only where the
     * value changed.
     */
    reanchor(): void;
    /**
     * Is `node` inside this tag? A pointer that moved onto the tag has not left
     * the thing it is about, which is what lets a consumer keep a whisper up
     * while the reader hovers its content.
     */
    contains(node: Node | null): boolean;
    /** Remove it. Idempotent, and a closed tag stops re-anchoring. */
    close(): void;
}

/**
 * Open a tag docked under `anchor` and return the handle to it.
 *
 * Placement is **under** the anchor, horizontally centred on it, and only the
 * horizontal axis is clamped into the viewport. That asymmetry is the whole
 * reason the grammar is called *docked*: position is what says which entity the
 * whisper is about, so a tag that drifted upward to stay visible would be
 * saying it about nothing. Vertical containment is the consumer's problem to
 * not have, by not whispering about cards it has not panned to.
 */
export function openDockedTag(options: DockedTagOptions): DockedTag {
    const { layer, anchor, content, variant } = options;

    const element = document.createElement("div");
    element.className = variant
        ? DOCKED_TAG_CLASS + " " + variant
        : DOCKED_TAG_CLASS;
    for (const node of content) {
        element.appendChild(node);
    }
    layer.appendChild(element);

    // Measured once, on open. `reanchor` runs at pointer-move frequency, and a
    // second box read there would be a second forced layout per move for a
    // number only the clamp uses — and that the CSS `translateX(-50%)` does not
    // need at all, so content that changes width stays centred regardless.
    const halfWidth = (element.getBoundingClientRect?.().width ?? 0) / 2;
    let closed = false;

    function reanchor(): void {
        if (closed || typeof anchor.getBoundingClientRect !== "function") {
            return;
        }
        const box = anchor.getBoundingClientRect();
        const top = box.bottom + "px";
        if (element.style.top !== top) {
            element.style.top = top;
        }
        let x = box.left + box.width / 2;
        const min = halfWidth;
        const max = window.innerWidth - halfWidth;
        if (max > min) {
            x = Math.min(Math.max(x, min), max);
        }
        const left = x + "px";
        if (element.style.left !== left) {
            element.style.left = left;
        }
    }

    reanchor();

    return {
        element,
        anchor,
        reanchor,
        contains(node) {
            return node !== null && element.contains(node);
        },
        close() {
            closed = true;
            element.remove();
        },
    };
}
