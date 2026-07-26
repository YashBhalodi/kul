// The hover lens — relationship resolution as a zero-click affordance.
//
// While a person is selected, hovering any other card resolves the tie live
// and whispers it in a pill docked under the *target*. No click, no mode, no
// second selection state: nine prototype rounds tried shift-click, drag-thread,
// corner badges and pick-mode relating, and the lens superseded all four
// (#276, #302).
//
// This is the interaction the architecture was arranged around. A zero-click
// affordance firing on pointer movement cannot afford a webview→extension→LSP
// round-trip, which is why the engine runs in the webview (ADR-0034) and why
// phrasing is a local module (ADR-0033). What that buys is spent here.
//
// Three properties are load-bearing rather than stylistic:
//
// - **Position disambiguates direction.** The pill docks under the hovered
//   card, so "how is *this* person related to the selected one" needs no arrow,
//   no label and no second colour. That is why it is anchored and not centred.
// - **Nothing is collapsed.** `queryResolve` returns every way two people are
//   related and every one of them is rendered, because a consanguineous family
//   flattened to a "primary" tie is a lie about the record (ADR-0028).
// - **Emptiness stays honest.** `disconnected` and `noneWithinBounds` are two
//   different answers and the pill says two different things.
//
// The pill's *box* is not this module's: `docked-tag.ts` owns the whisper
// grammar — placement against a card's screen box, the clamp, the chrome — and
// this file owns only when one opens and what goes in it. #303 whispers a
// can't-say reason in the same grammar without touching anything here
// (ADR-0044).
//
// ACCESSIBILITY. Per ADR-0036 this chrome carries no `role`, no `aria-*` and no
// `:focus-visible`. The lens is mouse-driven by decision; the neighbouring
// controls, legend and error popover keep everything they have.

import { type DockedTag, openDockedTag } from "./docked-tag.js";
import type { QueryEnvelope, ResolveResult } from "./engine-wire.js";
import { isQueryOk } from "./engine-wire.js";
import type { RelationshipDescriptor } from "./phrasing/descriptor.js";
import { type ResultBinding, selectBoundNodes } from "./result-binding.js";
import { type SelectionStore, selectionAnchorPerson } from "./selection.js";

/**
 * Trailing-edge delay before a settled pointer becomes a query.
 *
 * `queryResolve` measures 32 ms at the 10,000-person ceiling — inside
 * ADR-0029's 50 ms budget for one call, and *not* free: 32 ms of main-thread
 * work per pointer move is a jank source, which is why #292 prescribed a
 * debounce and explicitly ruled out a spinner. 120 ms is above the worst
 * measured call, so a settle can never overlap the next one, and below the
 * ~200 ms at which an affordance stops reading as live.
 */
export const LENS_DEBOUNCE_MS = 120;

/** Class every node on a traced resolution path wears. Painted the reserved sky. */
export const LENS_PATH_CLASS = "kul-query-path";

/**
 * The lens's variant class on its docked tag. It carries no rules of its own —
 * the chrome is `.kul-docked-tag`'s — and exists so the lens's pill is findable
 * among whatever else docks a tag (#303 will add its own).
 */
export const LENS_CLASS = "kul-lens";

/**
 * What the pill says when the two lie in different components of the relation
 * graph. No budget could ever find a tie, so the parenthesis says *why* there
 * is none rather than how far the engine looked.
 */
export const NOT_RELATED_DISCONNECTED = "not related (no connection)";

/**
 * What the pill says when the engine looked as far as its budget allows and
 * found nothing. A bigger budget might; the wording says so. This is the form
 * #302 names, and the reason a bare "not related" is never rendered for either
 * case — the engine distinguishes them, so the surface must (ADR-0028).
 */
export const NOT_RELATED_WITHIN_BOUNDS = "not related (within bounds)";

/** Gloss on the viewpoint dot: whose reading of the tie the pill is showing. */
export const VIEWPOINT_TITLE = "as the selected person sees it";

/** Separator between the terms of a multi-tie pill — *māmā · sasro*. */
const TERM_SEPARATOR = "·";

export interface HoverLensOptions {
    /** The rendered-SVG host (`#root`). Hit-testing and path paint land inside it. */
    root: HTMLElement;
    /**
     * The float layer itself (`#kul-region-float`), **not** its edge dock —
     * ADR-0042's placement for entity-anchored chrome. Passed straight through
     * to {@link openDockedTag}, which owns everything about how the pill sits
     * in it; the lens only decides that one should be there.
     */
    layer: HTMLElement;
    /** The selection seam. The lens reads its person anchor and follows changes. */
    selection: SelectionStore;
    /** Two-anchor resolution, ego first (ADR-0028). */
    resolve(
        egoId: string,
        alterId: string,
    ): Promise<QueryEnvelope<ResolveResult> | null>;
    /**
     * Phrase one relationship into one element and **keep** it phrased —
     * `LocaleController.bind`. Taken as a function rather than as a pack so a
     * pill that is up when the reader flips language re-phrases itself, and so
     * the lens never has to know that packs exist.
     */
    bindPhrase(element: HTMLElement, descriptor: RelationshipDescriptor): () => void;
    /**
     * Publish the persons this reading names, so the ambient dim lifts off
     * them; `null` withdraws. `QuerySurface.setDimExemption` is the wiring and
     * ADR-0043 is the rule: the lens is what the reader is doing *now* and the
     * dim is what they did a moment ago, so the lens wins. With a kin set
     * painted, the persons a trace runs through are almost always outside the
     * answer — and an explanation rendered at the dim's alpha explains nothing.
     *
     * The set comes from the descriptors' own backbones, not from the picture,
     * so it owes no republish after a render (`dim.ts` obliges only sources
     * that read the DOM to decide) — and a render dismisses the lens anyway,
     * which withdraws it.
     */
    onTrace?(personIds: Iterable<string> | null): void;
    /** Trailing-edge delay. Defaults to {@link LENS_DEBOUNCE_MS}. */
    delayMs?: number;
}

export interface HoverLens {
    /**
     * The pointer moved over (or off) the rendered picture. Called on every
     * pointer move, so it must be cheap: it hit-tests, re-anchors a pill that
     * is already up, and otherwise re-arms the debounce. Only a *settle*
     * queries.
     *
     * `target` is the element under the pointer — or, for a pointer leaving the
     * canvas, the element it moved onto, so moving onto the pill to read a term
     * does not dismiss the pill under the pointer.
     */
    handleHover(target: Element | null): void;
    /** Take the lens down: pill removed, path unpainted, pending query abandoned. */
    dismiss(): void;
    dispose(): void;
}

/** The card under `target`, ghost or canonical — a tie is about a person, not a card. */
function cardAt(target: Element | null): Element | null {
    if (!target || typeof target.closest !== "function") {
        return null;
    }
    const card = target.closest("[data-person-id]");
    return card?.getAttribute("data-person-id") ? card : null;
}

/**
 * Every node the rendered picture holds for one relationship path (ADR-0034's
 * binding rules): each hop's edge, plus the persons the path *passes through*.
 *
 * An `across` hop names its marriage. A vertical hop is drawn as a child's
 * birth or adoption edge, and which person is the child depends on direction —
 * a `down` hop lands on the child, an `up` hop leaves from it. Getting that
 * backwards paints a plausible but wrong edge, which is why the walk carries
 * the person it came from rather than reading the hop alone.
 *
 * Neither endpoint's **card** is on the list: the ego already wears the
 * selection violet and the alter is the card under the pointer. What the trace
 * adds is the part the reader cannot see — who stands between them. An
 * endpoint's own birth edge *can* light, because a leading `up` hop was drawn
 * as it, and that edge is genuinely part of the answer.
 */
export function resolutionPathBindings(
    descriptor: RelationshipDescriptor,
): ResultBinding[] {
    const bindings: ResultBinding[] = [];
    let from = descriptor.egoId;
    descriptor.path.forEach((hop, index) => {
        if (hop.step === "across") {
            bindings.push({ kind: "marriage", marriageId: hop.marriage });
        } else {
            bindings.push({
                kind: "parenthood",
                childId: hop.step === "down" ? hop.to : from,
            });
        }
        if (index < descriptor.path.length - 1) {
            bindings.push({ kind: "person", personId: hop.to });
        }
        from = hop.to;
    });
    return bindings;
}

/**
 * Every person the reading names besides the ego: the intermediates a trace
 * runs through, and the alter it lands on. Deduplicated across a multi-tie,
 * because two ways of being related routinely share a person.
 *
 * Read off the backbones rather than off the painted DOM, which is what makes
 * it safe to publish as a dim exemption without owing a republish after a
 * render. The ego is absent because kin paint already refuses to dim an anchor
 * — naming it here would imply a need that does not exist.
 */
export function tracedPersonIds(
    relationships: ReadonlyArray<RelationshipDescriptor>,
): string[] {
    const ids = new Set<string>();
    for (const descriptor of relationships) {
        for (const hop of descriptor.path) {
            ids.add(hop.to);
        }
    }
    return [...ids];
}

/**
 * The whisper for an answer with no relationships in it, or `null` when there
 * are relationships to show instead.
 *
 * A missing reason cannot occur — the engine sets one iff the list is empty —
 * but it falls through to the bounded wording rather than asserting the
 * stronger claim, which is the same choice `kul query rel`'s human output makes.
 */
export function emptinessWhisper(result: ResolveResult): string | null {
    if (result.relationships.length > 0) {
        return null;
    }
    return result.emptyReason === "disconnected"
        ? NOT_RELATED_DISCONNECTED
        : NOT_RELATED_WITHIN_BOUNDS;
}

export function createHoverLens(options: HoverLensOptions): HoverLens {
    const { root, layer, selection, resolve, bindPhrase, onTrace } = options;
    const delayMs = options.delayMs ?? LENS_DEBOUNCE_MS;

    // The card the lens is currently about: pending a query, or showing one.
    // Hover is deduplicated against it, so a pointer resting on one card
    // re-arms nothing and a sweep across ten re-arms ten times but queries once.
    let armed: Element | null = null;
    let timer: ReturnType<typeof setTimeout> | null = null;
    // Every re-arm invalidates any answer still in flight, so a slow resolution
    // can never whisper about a card the pointer has already left.
    let generation = 0;
    // The open pill, as the docked-tag mechanism hands it back. The lens owns
    // *when* one is up and *what* is in it; where it sits is not its business.
    let tag: DockedTag | null = null;
    let unbindPhrases: Array<() => void> = [];

    function cancelPending(): void {
        if (timer !== null) {
            clearTimeout(timer);
            timer = null;
        }
    }

    function clearPaint(): void {
        root.querySelectorAll("." + LENS_PATH_CLASS).forEach((node) => {
            node.classList.remove(LENS_PATH_CLASS);
        });
    }

    function closePill(): void {
        for (const unbind of unbindPhrases) {
            unbind();
        }
        unbindPhrases = [];
        tag?.close();
        tag = null;
    }

    // Whether an exemption is currently published. Withdrawing walks every card
    // in the picture, so a pointer that never opened a pill must not pay for it.
    let tracing = false;

    function publishTrace(personIds: string[] | null): void {
        // An **empty** trace is a withdrawal, not a publication. An unrelated
        // pair whispers a bounded form and traces nobody, so publishing `[]`
        // would exempt no one while still walking every card in the picture —
        // twice, since the next dismiss would withdraw it again.
        if (personIds === null || personIds.length === 0) {
            if (!tracing) {
                return;
            }
            tracing = false;
            onTrace?.(null);
            return;
        }
        tracing = true;
        onTrace?.(personIds);
    }

    function clear(): void {
        closePill();
        clearPaint();
        publishTrace(null);
    }

    function paintPath(relationships: ReadonlyArray<RelationshipDescriptor>): void {
        for (const descriptor of relationships) {
            for (const binding of resolutionPathBindings(descriptor)) {
                for (const node of selectBoundNodes(root, binding)) {
                    node.classList.add(LENS_PATH_CLASS);
                }
            }
        }
    }

    /**
     * What goes *inside* the tag, plus the unbinds its phrase sites own —
     * returned together so the nodes and the teardown for them can never be
     * adopted separately.
     *
     * This is the lens's whole contribution to the pill. The box around it, and
     * where that box sits, belong to {@link openDockedTag} — which is what lets
     * #303 whisper a can't-say reason in the same grammar without touching this
     * file (ADR-0044).
     */
    function buildContent(result: ResolveResult): {
        nodes: Node[];
        unbinds: Array<() => void>;
    } {
        const nodes: Node[] = [];
        const unbinds: Array<() => void> = [];
        const empty = emptinessWhisper(result);
        if (empty !== null) {
            const note = document.createElement("span");
            note.className = "kul-lens-empty";
            note.textContent = empty;
            nodes.push(note);
            return { nodes, unbinds };
        }
        // The violet dot is the whole of the direction chrome: it says the
        // terms read from the selected person outward. Terms only otherwise —
        // it is a whisper, not a panel.
        const dot = document.createElement("span");
        dot.className = "kul-lens-viewpoint";
        dot.title = VIEWPOINT_TITLE;
        nodes.push(dot);
        result.relationships.forEach((descriptor, index) => {
            if (index > 0) {
                const separator = document.createElement("span");
                separator.className = "kul-lens-sep";
                separator.textContent = TERM_SEPARATOR;
                nodes.push(separator);
            }
            const term = document.createElement("span");
            term.className = "kul-lens-term";
            // `bindPhrase` writes the text, the element's own `lang`, the
            // phrase kind, the hop count and the Latin gloss, and keeps every
            // one of them current across a locale flip. The pill sizes from the
            // kind and the hop count and never reads the string itself
            // (ADR-0033, ADR-0039).
            unbinds.push(bindPhrase(term, descriptor));
            nodes.push(term);
        });
        return { nodes, unbinds };
    }

    function show(card: Element, result: ResolveResult): void {
        // Deliberately not `clear()`: withdrawing the exemption here and
        // republishing four lines later would walk every card twice for one
        // settle. The paint is torn down; the exemption is *replaced*.
        closePill();
        clearPaint();
        const built = buildContent(result);
        unbindPhrases = built.unbinds;
        tag = openDockedTag({
            layer,
            anchor: card,
            content: built.nodes,
            variant: LENS_CLASS,
        });
        paintPath(result.relationships);
        publishTrace(tracedPersonIds(result.relationships));
    }

    async function ask(card: Element, egoId: string, alterId: string): Promise<void> {
        const mine = ++generation;
        const envelope = await resolve(egoId, alterId);
        if (mine !== generation || armed !== card) {
            return;
        }
        if (!envelope || !isQueryOk(envelope)) {
            // A failing project already surfaced in the error popover; a
            // whisper about it would be a second, quieter error surface.
            return;
        }
        show(card, envelope.result);
    }

    function handleHover(target: Element | null): void {
        if (tag?.contains(target)) {
            // The pointer moved onto the pill to read a term. Leaving the
            // canvas is not leaving the lens.
            return;
        }
        const card = cardAt(target);
        const egoId = selectionAnchorPerson(selection.current);
        const alterId = card?.getAttribute("data-person-id") ?? null;
        // The lens reads only while a *person* is selected (an edge has no ego)
        // and never about the selected person themselves — "how is Giulia
        // related to Giulia" is not a question the reader asked.
        const live = egoId !== null && alterId !== null && alterId !== egoId ? card : null;
        if (live === armed) {
            // Same card: no new query. Re-anchor anyway, so the pill stays
            // docked while the reader drags the canvas underneath it.
            if (live) {
                tag?.reanchor();
            }
            return;
        }
        armed = live;
        cancelPending();
        // Whatever is up is about the card the pointer just left.
        clear();
        if (!live) {
            return;
        }
        const ego = egoId as string;
        const alter = alterId as string;
        timer = setTimeout(() => {
            timer = null;
            void ask(live, ego, alter);
        }, delayMs);
    }

    function dismiss(): void {
        armed = null;
        generation += 1;
        cancelPending();
        clear();
    }

    // A selection change moves the ego, so any tie on screen is about the
    // person who *was* selected. Take it down and let the next pointer move
    // re-arm it, rather than re-asking about a question nobody posed.
    const unsubscribe = selection.subscribe(() => dismiss());

    return {
        handleHover,
        dismiss,
        dispose() {
            unsubscribe();
            dismiss();
        },
    };
}
