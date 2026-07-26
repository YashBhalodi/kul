// Result ↔ node binding: which nodes in the rendered SVG an engine answer is
// about (ADR-0034).
//
// The rule the ADR pins is that **a result is about a person, not about a
// card**. ADR-0019 emits a ghost per past intimacy, so one person can own a
// canonical card plus several ghosts, and every one of them binds. Ghost cards
// stay individually unaddressable — they carry `data-person-id`, `data-kind`
// and `data-ghost-reason` but no marriage id, so no selector can tell two
// past-adoption ghosts of one person apart, and no decided interaction needs
// to.

/**
 * What an engine answer names, in the vocabulary the rendered SVG carries.
 *
 * The three variants are the three ways a descriptor reaches the picture: a
 * result person, an `across` hop (which names its marriage), and a vertical
 * hop (which lands on a child and is drawn as that child's birth / adoption
 * edge).
 */
export type ResultBinding =
    | { kind: "person"; personId: string }
    | { kind: "marriage"; marriageId: string }
    | { kind: "parenthood"; childId: string };

/**
 * The attribute a binding matches on, and the fixed selector that narrows the
 * search to candidate nodes. Ids are author-written and arrive here inside
 * engine answers, so they are compared as *values* rather than spliced into a
 * selector string — an id holding a quote can then never widen the match.
 */
function candidates(binding: ResultBinding): {
    selector: string;
    attribute: string;
    value: string;
} {
    switch (binding.kind) {
        case "person":
            // Canonical card AND every ghost: `data-person-id` alone, with no
            // `data-kind` narrowing.
            return {
                selector: "[data-person-id]",
                attribute: "data-person-id",
                value: binding.personId,
            };
        case "marriage":
            // Birth and adoption edges also carry `data-marriage-id`, so the
            // marriage edge is keyed on `data-link-kind` too — the bare
            // attribute would drag in every child edge under that marriage.
            return {
                selector: '[data-link-kind="marriage"][data-marriage-id]',
                attribute: "data-marriage-id",
                value: binding.marriageId,
            };
        case "parenthood":
            // A vertical hop lands on a person; the edge that drew the hop is
            // that person's birth or adoption edge. Both kinds bind, because a
            // child reached by both has two edges and the engine does not
            // collapse them (ADR-0026).
            return {
                selector:
                    '[data-link-kind="birth"][data-child-id], [data-link-kind="adoption"][data-child-id]',
                attribute: "data-child-id",
                value: binding.childId,
            };
    }
}

/**
 * Every node in `root` the binding paints, in document order. An empty array
 * when the picture holds no node for the binding — a stale id is a plain empty
 * answer, not an error.
 */
export function selectBoundNodes(
    root: ParentNode,
    binding: ResultBinding,
): Element[] {
    const { selector, attribute, value } = candidates(binding);
    return Array.from(root.querySelectorAll(selector)).filter(
        (node) => node.getAttribute(attribute) === value,
    );
}
