/**
 * The **phrasing layer** — descriptor → word, the layer the kinship engine
 * refuses to own (ADR-0025, ADR-0026, ADR-0033).
 *
 * A pure module: **no DOM imports**, no host, no state, so extracting it to
 * its own package stays mechanical if a second surface ever wants it. The rule
 * is enforced by `tests/phrasing/no-dom.test.ts`.
 *
 * The whole public surface is one function and one registry:
 *
 * ```ts
 * import { packFor, phrase } from "./phrasing/index.js";
 * phrase(descriptor, packFor("en")!); // { text, kind, hopCount }
 * ```
 */

export { phrase } from "./phrase.js";
export type { Phrase, PhraseKind } from "./phrase.js";

export { PACKS, packFor, EN } from "./packs/index.js";
export type {
    AffixRule,
    GenitivePattern,
    HopLexicon,
    LanguagePack,
    NumericBound,
    PackEntry,
} from "./pack.js";

export { NUMERIC_FACETS, PHRASING_FACETS, derivedFacetsOf, phrasingKeyOf, subPathKeyOf } from "./key.js";
export type { FacetMatch, NumericFacet, PhrasingKey } from "./key.js";

export { affixOrderKey, affixVerdict, facetsMatch, lexicalize, specificityOf, winningEntries } from "./lexicalize.js";
export type { LexicalForm } from "./lexicalize.js";

export type {
    Affinity,
    Classification,
    EdgeNature,
    Gender,
    HopEdge,
    LinealRole,
    MarriageStatus,
    PathHop,
    RelationshipDescriptor,
    Seniority,
    Sharing,
    Side,
} from "./descriptor.js";
