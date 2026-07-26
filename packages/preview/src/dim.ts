// The dim — one class, one owner, union semantics.
//
// More than one surface wants to push the non-answer back: kin paint does it
// today, #303's attribute filter will, and both are ambient context rather than
// a live read. They must not each own a class of their own — two `opacity`
// values on nested nodes multiply (0.3 × 0.3 = 0.09), and two independent
// strippers mean whichever repaints last silently undoes the other.
//
// So the class has exactly one owner, this registry, and the sources publish
// **sets of person ids** rather than DOM state:
//
// - a person is dimmed iff **any** active source dims them (union), so adding a
//   source can only ever dim more, never un-dim;
// - `--kul-dim-alpha` is applied once per card and never nests;
// - a source withdraws by publishing `null`, and `apply` recomputes from
//   whatever remains — so a repaint after a render is the same call as the
//   first paint.
//
// One thing outranks the union: an **exemption**, which is the set of persons a
// *live pointer-driven read* is tracing. #302's hover lens traces the persons
// that justify a relationship, and with a kin set painted those persons are
// almost always outside the answer — so the trace would render at 30% under the
// ambient dim. The lens is the thing the reader is doing right now and the dim
// is the thing they did a moment ago, so the lens wins (ADR-0043). The
// exemption is held here rather than in kin paint so every later source
// inherits the rule instead of re-deciding it.

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
     * The persons a live read is tracing, lifted from **every** source's dim.
     * `null` withdraws the exemption. Does not touch the DOM.
     */
    exempt(personIds: Iterable<string> | null): void;
    /** Recompute the class on every card in `root` from the current union. */
    apply(root: ParentNode): void;
    /** Withdraw every source and the exemption, and strip the class. */
    reset(root: ParentNode): void;
    /** The union currently in force, exemption already subtracted. */
    dimmedPersonIds(): ReadonlySet<string>;
}

export function createDimRegistry(): DimRegistry {
    const sources = new Map<DimSource, ReadonlySet<string>>();
    let exemption: ReadonlySet<string> = new Set();

    function union(): Set<string> {
        const all = new Set<string>();
        for (const ids of sources.values()) {
            for (const id of ids) {
                if (!exemption.has(id)) {
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
        exempt(personIds) {
            exemption = personIds === null ? new Set() : new Set(personIds);
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
            exemption = new Set();
            for (const node of root.querySelectorAll("." + DIM_CLASS)) {
                node.classList.remove(DIM_CLASS);
            }
        },
        dimmedPersonIds: union,
    };
}
