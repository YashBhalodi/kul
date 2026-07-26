// The dim — one class, one owner, union semantics.
//
// More than one surface pushes the non-answer back: kin paint and the
// attribute filter, both ambient context rather than a live read. They must not
// each own a class of their own — two `opacity` values on nested nodes multiply
// (0.3 × 0.3 = 0.09), and two independent strippers mean whichever repaints
// last silently undoes the other.
//
// So the class has exactly one owner, this registry, and the sources publish
// **sets of person ids** rather than DOM state:
//
// - a person is dimmed iff **any** active source dims them (union), so adding a
//   source can only ever dim more, never un-dim;
// - `--kul-dim-alpha` is applied once per card and never nests;
// - a source withdraws by publishing `null`, and `apply` recomputes from
//   whatever remains.
//
// **A render obliges each source to republish, not merely to `apply`.** The
// registry holds ids, not nodes, so `apply` alone is enough for a source whose
// set is DOM-independent — but kin paint derives its set by walking the cards
// that are *not* in the answer, so a swapped-out SVG can change it. That is why
// `paintKinResults` recomputes rather than calling `apply`, and any later source
// that reads the picture to decide who it dims inherits the same obligation.
//
// One thing outranks the union: an **exemption** — persons a source lifts out
// of every source's dim. Exemptions are **keyed and unioned**, exactly as dims
// are, because two of them exist and neither may clobber the other:
//
// - the **hover lens** publishes the persons its trace runs through. With a kin
//   set painted those are almost always outside the answer, so the sky trace
//   would render at 30% under the ambient dim. The lens is what the reader is
//   doing now and the dim is what they did a moment ago, so the lens wins
//   (ADR-0043). It is a live read: published on settle, withdrawn on dismiss.
// - the **filter's can't-say set** publishes standing, for as long as the
//   filter stands. An unjudgeable person is a third paint state, not a
//   non-match, and an amber dashed ring at 30% is the silent drop the whole
//   disclosure exists to prevent (ADR-0045).
//
// The two coexist — a filter running inside a painted kin set with the pointer
// on a card is all three at once — which is why ADR-0043's single slot became a
// key. Holding this here rather than in any one paint is what lets a later
// source inherit the rule instead of re-deciding it.

/** Class a dimmed card wears. One themed alpha, no colour. */
export const DIM_CLASS = "kul-query-dim";

/** Who is dimming. A stable string per paint source. */
export type DimSource = string;

export interface DimRegistry {
    /**
     * Publish one source's dimmed person ids, or withdraw it with `null`.
     * Does not touch the DOM — call {@link DimRegistry.apply} for that.
     */
    set(source: DimSource, personIds: Iterable<string> | null): void;
    /**
     * Publish one source's exempt person ids — lifted from **every** source's
     * dim, this one's included — or withdraw it with `null`. Does not touch the
     * DOM.
     *
     * Keyed like {@link DimRegistry.set}, and for the same reason: a person is
     * exempt iff **any** source exempts them, so a second holder can only ever
     * exempt more, and neither can clobber the other by publishing.
     */
    exempt(source: DimSource, personIds: Iterable<string> | null): void;
    /** Recompute the class on every card in `root` from the current union. */
    apply(root: ParentNode): void;
    /** Withdraw every source and every exemption, and strip the class. */
    reset(root: ParentNode): void;
    /** The union currently in force, exemptions already subtracted. */
    dimmedPersonIds(): ReadonlySet<string>;
}

export function createDimRegistry(): DimRegistry {
    const sources = new Map<DimSource, ReadonlySet<string>>();
    const exemptions = new Map<DimSource, ReadonlySet<string>>();

    function isExempt(id: string): boolean {
        for (const ids of exemptions.values()) {
            if (ids.has(id)) {
                return true;
            }
        }
        return false;
    }

    function union(): Set<string> {
        const all = new Set<string>();
        for (const ids of sources.values()) {
            for (const id of ids) {
                if (!isExempt(id)) {
                    all.add(id);
                }
            }
        }
        return all;
    }

    return {
        set(source, personIds) {
            if (personIds === null) {
                sources.delete(source);
            } else {
                sources.set(source, new Set(personIds));
            }
        },
        exempt(source, personIds) {
            if (personIds === null) {
                exemptions.delete(source);
            } else {
                exemptions.set(source, new Set(personIds));
            }
        },
        apply(root) {
            const dimmed = union();
            for (const card of root.querySelectorAll("[data-person-id]")) {
                const id = card.getAttribute("data-person-id") ?? "";
                card.classList.toggle(DIM_CLASS, dimmed.has(id));
            }
        },
        reset(root) {
            sources.clear();
            exemptions.clear();
            for (const node of root.querySelectorAll("." + DIM_CLASS)) {
                node.classList.remove(DIM_CLASS);
            }
        },
        dimmedPersonIds: union,
    };
}
