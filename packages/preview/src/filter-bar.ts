// The filter bar — the chip sentence, in stage chrome.
//
// *Showing ⟨Everyone 10 ▾⟩ where ⟨family = Rossi ✕⟩ and ⟨born < 1950 ✕⟩ ⟨+⟩
// ⟨certain ▾⟩*, with the tally after it. Three things about that sentence are
// decisions rather than layout:
//
//   - **Home is stage chrome, not the details panel.** A filter is a
//     project-scoped question and the panel is selection-scoped, so the bar is
//     live with nothing selected and the panel grows no filter affordances
//     (#277). It joins the flow region by being appended (ADR-0038).
//   - **The `and` between chips is literal text.** Conjunction-only is visible
//     in the grammar rather than documented somewhere the reader will not
//     look. There is no OR chip, no OR affordance and no advanced mode that
//     adds one, because OR is permanently out of engine scope (ADR-0025).
//   - **Certainty is a chip inside the sentence**, not a display toggle. The
//     third truth value is part of the query, so it sits inside the query's
//     own words.
//
// Evaluation asks the engine **both** certainty questions on every change. The
// difference between the two answers is the set the conjunction could not
// judge — the engine's own verdict, read off two of its answers rather than
// re-derived here, because `kul-core`'s `filter__*.snap` suites own predicate
// correctness and a second evaluator could disagree with the paint beside it.
//
// ACCESSIBILITY. Per ADR-0036 this chrome carries no `role`, no `aria-*` and
// no `:focus-visible`. Its controls are real `<select>` and `<input>` elements
// all the same, which is what makes `mountKeyboardPan`'s target guard bind:
// arrows and `+` / `-` / `0` typed into a chip editor belong to the field and
// must not pan the canvas.

import type { DimRegistry } from "./dim.js";
import { type DockedTag, openDockedTag } from "./docked-tag.js";
import type {
    DetailLookupResult,
    DetailTarget,
    ExportedPerson,
    FilterMode,
    Query,
    QueryEnvelope,
    QueryResult,
} from "./engine-wire.js";
import { isQueryOk } from "./engine-wire.js";
import {
    EMPTY_FILTER,
    EVERYONE_SCOPE_ID,
    FILTER_FIELDS,
    type FilterCondition,
    type FilterField,
    type FilterOp,
    type FilterScope,
    type FilterState,
    type FilterTally,
    GENDER_VALUES,
    PRESENCE_DISCLOSURE,
    askedConditions,
    buildFilterQuery,
    computeTally,
    conditionLabel,
    fieldKind,
    isFilterActive,
    partitionScope,
    isPresenceOp,
    modeLabel,
    opLabel,
    opsFor,
    tallyText,
} from "./filter.js";
import { cantSayReason } from "./filter-reason.js";
import { paintFilterResults, uncertainPersonAt } from "./filter-paint.js";

/** The bar's element id, so a host can find it without knowing its class. */
export const FILTER_BAR_ID = "kul-filter-bar";

/** How the scope chip's default entry reads. */
export const EVERYONE_LABEL = "Everyone";

/**
 * This whisper's variant class on its docked tag. Like the lens's `kul-lens`
 * it carries no rules of its own — the chrome is `.kul-docked-tag`'s — and
 * exists so the two things that dock a tag into one layer stay tellable apart.
 */
export const FILTER_REASON_CLASS = "kul-filter-reason-tag";

/** Which chip an open editor belongs to. `number` indexes the conditions. */
type EditorKey = number | "scope" | "mode";

export interface FilterBarOptions {
    /** Flow-region host. The bar is appended to it (ADR-0038). */
    host: HTMLElement;
    /** The rendered-SVG host. Paint and hit-testing land inside it. */
    root: HTMLElement;
    /**
     * The float layer itself (`#kul-region-float`) — the can't-say whisper
     * docks under a card there, in the docked-tag grammar (ADR-0044).
     */
    floatLayer: HTMLElement;
    /** The shared dim. This surface publishes a set of ids and owns no class. */
    dim: DimRegistry;
    /** Evaluate one {@link Query}. `null` when there is nothing to ask. */
    run(query: Query): Promise<QueryEnvelope<QueryResult> | null>;
    /**
     * The batched detail lookup — the **only** source for a can't-say reason.
     * The reason describes a person's record, and person data has one
     * provenance path (ADR-0035); reading it off the SVG would open a second.
     */
    lookup(
        targets: DetailTarget[],
    ): Promise<QueryEnvelope<DetailLookupResult> | null>;
    /**
     * The kin set currently painted, or `null` — the scope chip's second
     * entry, and what makes "which of Giuseppe's descendants were born before
     * 1950" one question rather than two. Asked on every render of the bar, so
     * a kin set that is painted or cleared shows up without a notification.
     */
    kinScope(): FilterScope | null;
    /**
     * Register or lift this surface's editor-sync suspension. An active filter
     * suspends sync exactly as a selection does, and independently of one
     * (#276 point 9).
     */
    setSyncSuspended(suspended: boolean): void;
}

export interface FilterBar {
    /** The bar element, live in the flow region until {@link FilterBar.dispose}. */
    readonly element: HTMLElement;
    /** The sentence as it currently reads. */
    state(): FilterState;
    /**
     * The pointer moved over the canvas. An unjudgeable card under it opens
     * its reason; anything else takes the whisper down.
     */
    handleHover(target: Element | null): void;
    /**
     * Drop the filter entirely: sentence, paint, dim, can't-say exemption and
     * sync suspension. Synchronous and idempotent.
     *
     * **The bar's only exit**, and it has exactly one production call site —
     * `QuerySurface.clearFilter` — which both ways out of query mode route
     * through: Esc, and a render, which is an edit (ADR-0046). There is
     * deliberately no post-render *re-ask*: a filter never outlives the source
     * it was computed against, so re-evaluating against a new project snapshot
     * is a question nobody asked.
     *
     * Removing the last condition chip is **not** this and must not be
     * confused with it: `removeCondition` commits a state that keeps the
     * reader's scope and certainty chips, while this restores `EMPTY_FILTER`.
     * A sentence with no conditions left is still the reader's sentence; an
     * ended query mode is not.
     */
    reset(): void;
    dispose(): void;
}

/** Every person the picture draws — the `Everyone` scope's population. */
function personIdsInTree(root: ParentNode): Set<string> {
    const ids = new Set<string>();
    for (const card of root.querySelectorAll("[data-person-id]")) {
        const id = card.getAttribute("data-person-id");
        if (id) {
            ids.add(id);
        }
    }
    return ids;
}

/**
 * The person ids in a result.
 *
 * `members` and `personIds` are the same set one field apart — which of them
 * comes back is fixed by (source, projection), and this surface always asks
 * for `members`, so `count` is unreachable. It answers an empty set rather
 * than throwing, because a projection nobody asked for is not the reader's
 * problem.
 */
function idsOf(result: QueryResult): string[] {
    switch (result.kind) {
        case "personIds":
            return result.personIds;
        case "members":
            return result.members.map((member) => member.personId);
        case "count":
            return [];
    }
}

function element<K extends keyof HTMLElementTagNameMap>(
    tag: K,
    className: string,
    text?: string,
): HTMLElementTagNameMap[K] {
    const node = document.createElement(tag);
    node.className = className;
    if (text !== undefined) {
        node.textContent = text;
    }
    return node;
}

function keyword(text: string): HTMLElement {
    return element("span", "kul-filter-keyword", text);
}

function select(
    options: ReadonlyArray<{ value: string; label: string }>,
    selected: string,
): HTMLSelectElement {
    const node = document.createElement("select");
    for (const option of options) {
        const item = document.createElement("option");
        item.value = option.value;
        item.textContent = option.label;
        node.appendChild(item);
    }
    node.value = selected;
    return node;
}

export function createFilterBar(options: FilterBarOptions): FilterBar {
    const { host, root, floatLayer, dim, run, lookup, kinScope, setSyncSuspended } =
        options;

    const bar = element("div", "kul-filter-bar");
    bar.id = FILTER_BAR_ID;
    host.appendChild(bar);

    let state: FilterState = EMPTY_FILTER;
    let tally: FilterTally | null = null;
    let openEditor: EditorKey | null = null;
    // Every change to the sentence — and every render — invalidates whatever
    // is in flight, so a slow pair of answers can never paint a question the
    // reader has moved on from.
    let generation = 0;

    // The whisper, and the two things that can invalidate one: the filter it
    // belongs to, and where the pointer is now. They are guarded separately
    // because either can move without the other.
    let tag: DockedTag | null = null;
    let hoveredPersonId: string | null = null;
    // Who a record is being fetched for right now. The pointer fires many
    // times over one card, and without this each move would queue another
    // lookup for a person already being looked up.
    let pendingReasonFor: string | null = null;
    const reasons = new Map<string, string>();

    function currentScope(): FilterScope {
        const kin = kinScope();
        if (kin && kin.id === state.scopeId) {
            return kin;
        }
        return {
            id: EVERYONE_SCOPE_ID,
            label: EVERYONE_LABEL,
            source: { kind: "allPersons" },
            personIds: personIdsInTree(root),
        };
    }

    function closeTag(): void {
        if (tag) {
            tag.close();
            tag = null;
        }
    }

    function commit(next: FilterState): void {
        state = next;
        void evaluate();
    }

    // ── Evaluation ───────────────────────────────────────────────────────

    async function evaluate(): Promise<void> {
        const mine = ++generation;
        // The whisper belongs to the answer that produced it. A new question —
        // or a new project under the same question — means new records, so the
        // cache goes with it rather than outliving what it explains.
        closeTag();
        reasons.clear();
        pendingReasonFor = null;
        const scope = currentScope();
        // Suspension follows the reader's mode, not the engine's answer: the
        // filter is active the moment they finish a chip, whether or not the
        // answer has landed.
        setSyncSuspended(isFilterActive(state));
        if (!isFilterActive(state)) {
            tally = null;
            paintFilterResults(root, null, dim);
            render();
            return;
        }
        render();
        const [certain, inclusive] = await Promise.all([
            run(buildFilterQuery(state, scope, "certain")),
            run(buildFilterQuery(state, scope, "includeUncertain")),
        ]);
        if (mine !== generation) {
            return;
        }
        if (
            !certain ||
            !inclusive ||
            !isQueryOk(certain) ||
            !isQueryOk(inclusive)
        ) {
            // No engine, or a project that failed its checks — the popover
            // already carries the diagnostics (ADR-0009). The sentence stays;
            // the paint goes, because a stale verdict is worse than none.
            tally = null;
            paintFilterResults(root, null, dim);
            render();
            return;
        }
        // One partition, counted and painted — never two arithmetics that
        // could disagree about who is amber.
        const partition = partitionScope(
            scope.personIds,
            idsOf(certain.result),
            idsOf(inclusive.result),
        );
        tally = computeTally(partition, state.mode);
        paintFilterResults(root, partition, dim);
        render();
    }

    // ── The can't-say whisper ────────────────────────────────────────────

    function showReason(card: Element, reason: string): void {
        closeTag();
        tag = openDockedTag({
            layer: floatLayer,
            anchor: card,
            content: [element("span", "kul-filter-reason", reason)],
            variant: FILTER_REASON_CLASS,
        });
    }

    async function loadReason(
        personId: string,
        card: Element,
        mine: number,
    ): Promise<void> {
        pendingReasonFor = personId;
        const envelope = await lookup([{ kind: "person", id: personId }]);
        if (pendingReasonFor === personId) {
            pendingReasonFor = null;
        }
        // Two guards, for the two things that can have moved: the filter this
        // reason is about, and the pointer that asked for it.
        if (mine !== generation || hoveredPersonId !== personId) {
            return;
        }
        if (!envelope || !isQueryOk(envelope)) {
            return;
        }
        const detail = envelope.result[0];
        if (!detail || detail.kind !== "person") {
            return;
        }
        const person: ExportedPerson = detail.person;
        // The conditions the engine was *asked*, not every chip on screen: a
        // half-written one asks nothing, so naming it as a candidate would
        // explain the verdict with a predicate that never ran.
        const reason = cantSayReason(person, askedConditions(state));
        reasons.set(personId, reason);
        showReason(card, reason);
    }

    function handleHover(target: Element | null): void {
        const hit = uncertainPersonAt(target);
        if (!hit) {
            hoveredPersonId = null;
            // A pointer that moved onto the whisper has not left the card the
            // whisper is about.
            if (tag && !tag.contains(target)) {
                closeTag();
            }
            return;
        }
        if (hoveredPersonId === hit.personId && tag) {
            tag.reanchor();
            return;
        }
        hoveredPersonId = hit.personId;
        const cached = reasons.get(hit.personId);
        if (cached !== undefined) {
            showReason(hit.card, cached);
            return;
        }
        closeTag();
        if (pendingReasonFor === hit.personId) {
            // Already being fetched. The pointer fires per pixel; the lookup
            // must not.
            return;
        }
        void loadReason(hit.personId, hit.card, generation);
    }

    // ── The sentence ─────────────────────────────────────────────────────

    function chip(
        key: EditorKey | "add",
        label: string,
        extraClass?: string,
    ): HTMLElement {
        const wrapper = element(
            "span",
            extraClass ? `kul-filter-chip ${extraClass}` : "kul-filter-chip",
        );
        if (key !== "add" && openEditor === key) {
            wrapper.classList.add("kul-filter-chip-open");
        }
        const button = element("button", "kul-filter-chip-label", label);
        button.type = "button";
        button.addEventListener("click", () => {
            if (key === "add") {
                addCondition();
                return;
            }
            openEditor = openEditor === key ? null : key;
            render();
        });
        wrapper.appendChild(button);
        return wrapper;
    }

    function addCondition(): void {
        const next = [
            ...state.conditions,
            { field: "family" as FilterField, op: "eq" as FilterOp, value: "" },
        ];
        openEditor = next.length - 1;
        // A new chip asks nothing until it is finished, so this changes the
        // sentence without changing the question.
        state = { ...state, conditions: next };
        render();
    }

    function removeCondition(index: number): void {
        openEditor = null;
        commit({
            ...state,
            conditions: state.conditions.filter((_, i) => i !== index),
        });
    }

    function updateCondition(index: number, next: FilterCondition): void {
        commit({
            ...state,
            conditions: state.conditions.map((c, i) => (i === index ? next : c)),
        });
    }

    function conditionEditor(index: number): HTMLElement {
        const condition = state.conditions[index];
        const editor = element("div", "kul-filter-editor");
        const fieldSelect = select(
            FILTER_FIELDS.map((f) => ({ value: f, label: f })),
            condition.field,
        );
        fieldSelect.addEventListener("change", () => {
            const field = fieldSelect.value as FilterField;
            const ops = opsFor(field);
            // The op menu is field-aware, so an op the new field does not
            // offer cannot survive the change; the value goes with it when the
            // kind of value changes.
            const op = ops.includes(condition.op) ? condition.op : ops[0];
            const keepValue =
                fieldKind(field) === fieldKind(condition.field) &&
                op === condition.op;
            updateCondition(index, {
                field,
                op,
                value: keepValue ? condition.value : "",
            });
        });
        editor.appendChild(fieldSelect);

        const opSelect = select(
            opsFor(condition.field).map((op) => ({ value: op, label: opLabel(op) })),
            condition.op,
        );
        opSelect.addEventListener("change", () => {
            const op = opSelect.value as FilterOp;
            updateCondition(index, {
                field: condition.field,
                op,
                value: isPresenceOp(op) ? "" : condition.value,
            });
        });
        editor.appendChild(opSelect);

        if (isPresenceOp(condition.op)) {
            // Said where the predicate is offered, not in a doc nobody opens:
            // a presence predicate is decidable, which makes it read more
            // confident than it is.
            editor.appendChild(
                element("div", "kul-filter-note", PRESENCE_DISCLOSURE),
            );
        } else if (fieldKind(condition.field) === "enum") {
            const valueSelect = select(
                GENDER_VALUES.map((v) => ({ value: v, label: v })),
                condition.value || GENDER_VALUES[0],
            );
            valueSelect.addEventListener("change", () => {
                updateCondition(index, { ...condition, value: valueSelect.value });
            });
            editor.appendChild(valueSelect);
        } else {
            const input = document.createElement("input");
            input.type = "text";
            input.value = condition.value;
            input.placeholder =
                condition.op === "in"
                    ? "Rossi, Bianchi"
                    : fieldKind(condition.field) === "date"
                      ? "1950, 1950-06, ~1950"
                      : "value";
            input.addEventListener("change", () => {
                updateCondition(index, { ...condition, value: input.value });
            });
            editor.appendChild(input);
        }
        return editor;
    }

    function scopeEditor(): HTMLElement {
        const editor = element("div", "kul-filter-editor");
        const kin = kinScope();
        const entries = [{ value: EVERYONE_SCOPE_ID, label: EVERYONE_LABEL }];
        if (kin) {
            entries.push({ value: kin.id, label: kin.label });
        }
        const scopeSelect = select(entries, currentScope().id);
        scopeSelect.addEventListener("change", () => {
            openEditor = null;
            commit({ ...state, scopeId: scopeSelect.value });
        });
        editor.appendChild(scopeSelect);
        if (!kin) {
            editor.appendChild(
                element(
                    "div",
                    "kul-filter-note",
                    "Paint a kin set to filter inside it.",
                ),
            );
        }
        return editor;
    }

    function modeEditor(): HTMLElement {
        const editor = element("div", "kul-filter-editor");
        const modes: FilterMode[] = ["certain", "includeUncertain"];
        const modeSelect = select(
            modes.map((m) => ({ value: m, label: modeLabel(m) })),
            state.mode,
        );
        modeSelect.addEventListener("change", () => {
            openEditor = null;
            commit({ ...state, mode: modeSelect.value as FilterMode });
        });
        editor.appendChild(modeSelect);
        return editor;
    }

    function render(): void {
        const scope = currentScope();
        // A kin set that is no longer painted takes the scope back to everyone
        // rather than leaving the chip naming a set nobody can see.
        if (state.scopeId !== scope.id) {
            state = { ...state, scopeId: scope.id };
        }
        bar.textContent = "";
        bar.appendChild(keyword("Showing"));

        const scopeChip = chip(
            "scope",
            `${scope.label} ${scope.personIds.size} ▾`,
            "kul-filter-chip-scope",
        );
        if (openEditor === "scope") {
            scopeChip.appendChild(scopeEditor());
        }
        bar.appendChild(scopeChip);

        state.conditions.forEach((condition, index) => {
            // Literal `where` and `and`: the sentence *shows* that the engine
            // only conjoins, rather than documenting it elsewhere.
            bar.appendChild(keyword(index === 0 ? "where" : "and"));
            const conditionChip = chip(index, conditionLabel(condition));
            const remove = element("button", "kul-filter-chip-remove", "✕");
            remove.type = "button";
            remove.addEventListener("click", () => removeCondition(index));
            conditionChip.appendChild(remove);
            if (openEditor === index) {
                conditionChip.appendChild(conditionEditor(index));
            }
            bar.appendChild(conditionChip);
        });

        bar.appendChild(chip("add", "+", "kul-filter-chip-add"));

        const modeChip = chip(
            "mode",
            `${modeLabel(state.mode)} ▾`,
            "kul-filter-chip-mode",
        );
        if (openEditor === "mode") {
            modeChip.appendChild(modeEditor());
        }
        bar.appendChild(modeChip);

        if (tally) {
            bar.appendChild(
                element(
                    "span",
                    "kul-filter-tally",
                    tallyText(
                        tally,
                        state.mode,
                        scope.id === EVERYONE_SCOPE_ID ? null : scope.label,
                    ),
                ),
            );
        }
    }

    render();

    return {
        element: bar,
        state: () => state,
        handleHover,
        reset() {
            openEditor = null;
            commit(EMPTY_FILTER);
        },
        dispose() {
            generation += 1;
            closeTag();
            paintFilterResults(root, null, dim);
            setSyncSuspended(false);
            bar.remove();
        },
    };
}
