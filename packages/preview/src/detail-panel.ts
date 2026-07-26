// The details panel — one variant per entity kind, fed by one batched lookup.
//
// PROVENANCE. Every word this module renders comes from an {@link EntityDetail}
// the batched `queryDetail` operation returned (ADR-0035, ADR-0037). Nothing
// here reads the rendered SVG: not a `data-*` attribute, not a label's text.
// Reading the panel off the SVG is free, exhaustive and was rejected twice —
// by ADR-0034 on principle and by ADR-0035 again in its hybrid form — because
// it gives person data two provenance paths. `tests/detail-panel.test.ts`
// holds that as a lint over this file's own source text, and drives the panel
// with names that deliberately disagree with the picture.
//
// The model is separated from the DOM so the provenance question is answerable
// at a pure seam: `buildDetailPanel` takes a wire value and returns a value.
//
// ACCESSIBILITY. Per ADR-0036 this chrome carries no `role`, no `aria-*` and no
// `:focus-visible`. That is a decision, not an oversight; the neighbouring
// controls / legend / error popover keep everything they have.

import type {
    EntityDetail,
    ExportedDate,
    ExportedMarriage,
    ExportedParenthoodLink,
    ExportedPerson,
    LinkedPerson,
} from "./engine-wire.js";
import type { ScreenBox } from "./highlight.js";
import { buildKinList } from "./kin-list.js";
import type { KinListModel, KinListState } from "./kin-list.js";
import type { LanguagePack } from "./phrasing/index.js";

/** One `label: value` line of an entity's own fields. */
export interface PanelField {
    label: string;
    value: string;
}

/**
 * One line of a section. A row that names a person carries {@link PanelRow.person}
 * and is clickable — clicking moves the selection and centres that person's
 * canonical card. `note` is the row's qualifier: a span, a link kind, an end
 * reason.
 */
export interface PanelRow {
    person?: { id: string; name: string };
    /** Stands in for a person the answer could not name. */
    text?: string;
    note?: string;
}

/** A titled group of rows. Sections with no rows are omitted, never emptied. */
export interface PanelSection {
    title: string;
    rows: PanelRow[];
}

/**
 * What the panel renders, as a value. One variant of {@link EntityDetail} maps
 * onto one of these, 1:1.
 */
export interface DetailPanelModel {
    kind: EntityDetail["kind"];
    /** Entity-kind kicker: "Person" / "Marriage" / "Adoption". */
    kicker: string;
    title: string;
    /**
     * Entity the header's reveal control addresses. An adoption link has no
     * entity id, so an adoption reveals its **child** (ADR-0035).
     */
    revealId: string;
    fields: PanelField[];
    sections: PanelSection[];
    /**
     * The Explore-kin list, on a person panel that has one. `null` on the
     * marriage and adoption variants: a kin set needs a person anchor, and an
     * edge is a waypoint (ADR-0035, ADR-0042).
     */
    kin: KinListModel | null;
}

/**
 * Everything the panel renders that is **not** the engine's answer about the
 * entity: the language its phrased words are in, and the kin list's state.
 *
 * It is a second parameter rather than a second builder because the panel is
 * one widget with one model — and it is a *value* rather than a controller so
 * `buildDetailPanel` stays pure and the provenance lint over this file keeps
 * its plain reading (ADR-0042).
 *
 * `pack` is threaded from `createQuerySurface` through `createDetailPanel` to
 * here, which is the work ADR-0042 recorded as owed. It arrives already
 * resolved: the DOM layer reads the current pack at render time, so
 * `QuerySurface.refresh()` — which the locale toggle already drives — is all
 * a language flip needs.
 */
export interface DetailPanelView {
    pack: LanguagePack;
    kin: KinListState | null;
}

/**
 * A date as the reader sees it. The wire's `value` is already the authored
 * precision (`1980`, `1980-03`, `1980-03-15`); `circa` is the only decoration.
 */
function formatDate(date: ExportedDate): string {
    return date.circa ? "c. " + date.value : date.value;
}

/**
 * A span, omitting whichever half is absent — the wire uses
 * `skip_serializing_if` on both, and the panel shows what the engine sends.
 */
function formatSpan(
    start: ExportedDate | undefined,
    end: ExportedDate | undefined,
): string {
    if (start && end) {
        return formatDate(start) + " – " + formatDate(end);
    }
    if (start) {
        return "from " + formatDate(start);
    }
    if (end) {
        return "until " + formatDate(end);
    }
    return "";
}

/** Join a row's qualifiers, dropping the absent ones. */
function note(...parts: Array<string | undefined>): string | undefined {
    const kept = parts.filter((part): part is string => !!part);
    return kept.length ? kept.join(" · ") : undefined;
}

/**
 * The document's own word for a parenthood link, matching the legend the reader
 * already has on screen rather than the wire's internal spelling.
 *
 * English, like every *label* this module emits. Panel labels are **chrome**,
 * and #276 point 6 keeps chrome labels English while the locale setting governs
 * kinship *phrasing*; ADR-0022 pins the same line for the legend, whose rows
 * these words deliberately mirror. The panel's one phrased slot is the kin
 * list's term gloss, and `QuerySurface.refresh()` is what redraws it when the
 * locale flips.
 */
function linkKindLabel(link: ExportedParenthoodLink): string {
    return link.kind === "adoptive" ? "adoption" : "birth";
}

function linkNote(link: ExportedParenthoodLink): string | undefined {
    return note(linkKindLabel(link), formatSpan(link.start, link.end));
}

function personRow(person: ExportedPerson, rowNote?: string): PanelRow {
    return { person: { id: person.id, name: person.name }, note: rowNote };
}

function linkedRow(linked: LinkedPerson): PanelRow {
    return personRow(linked.person, linkNote(linked.link));
}

function personFields(person: ExportedPerson): PanelField[] {
    const fields: PanelField[] = [{ label: "Gender", value: person.gender }];
    if (person.family) {
        fields.push({ label: "Family name", value: person.family });
    }
    if (person.given) {
        fields.push({ label: "Given name", value: person.given });
    }
    if (person.born) {
        fields.push({ label: "Born", value: formatDate(person.born) });
    }
    if (person.died) {
        fields.push({ label: "Died", value: formatDate(person.died) });
    }
    return fields;
}

function marriageFields(marriage: ExportedMarriage): PanelField[] {
    const fields: PanelField[] = [];
    if (marriage.start) {
        fields.push({ label: "Started", value: formatDate(marriage.start) });
    }
    if (marriage.end) {
        fields.push({ label: "Ended", value: formatDate(marriage.end) });
    }
    if (marriage.endReason) {
        fields.push({ label: "End reason", value: marriage.endReason });
    }
    return fields;
}

/** An adoption link records a span and nothing else; `end` marks one that ended. */
function adoptionFields(link: ExportedParenthoodLink): PanelField[] {
    const fields: PanelField[] = [];
    if (link.start) {
        fields.push({ label: "Started", value: formatDate(link.start) });
    }
    if (link.end) {
        fields.push({ label: "Ended", value: formatDate(link.end) });
    }
    return fields;
}

function section(title: string, rows: PanelRow[]): PanelSection[] {
    return rows.length ? [{ title, rows }] : [];
}

/**
 * Project one detail answer onto the panel.
 *
 * Two rules are load-bearing rather than cosmetic:
 *
 * - **Absent fields are omitted**, never rendered as "not recorded". The wire
 *   omits them, so the panel does; the panel's height varies as the selection
 *   walks, and that is intended (ADR-0035).
 * - **Each marriage occupies its own line, with its span and end reason.** A
 *   comma-joined spouse list would lose the explanation for why a person
 *   appears on more than one card. There is deliberately **no ghost note** —
 *   the ended marriage on the line is the underlying truth, and sourcing a
 *   ghost note would re-derive ADR-0019 inside the query layer.
 */
export function buildDetailPanel(
    detail: EntityDetail,
    view: DetailPanelView,
): DetailPanelModel {
    switch (detail.kind) {
        case "person":
            return {
                kind: "person",
                kicker: "Person",
                title: detail.person.name,
                revealId: detail.person.id,
                kin: view.kin ? buildKinList(view.kin, view.pack) : null,
                fields: personFields(detail.person),
                sections: [
                    ...section("Parents", detail.parents.map(linkedRow)),
                    ...section(
                        "Marriages",
                        detail.marriages.map((tie) => {
                            const rowNote = note(
                                formatSpan(tie.marriage.start, tie.marriage.end),
                                tie.marriage.endReason,
                            );
                            return tie.spouse
                                ? personRow(tie.spouse, rowNote)
                                : { text: tie.marriage.id, note: rowNote };
                        }),
                    ),
                    ...section("Children", detail.children.map(linkedRow)),
                ],
            };
        case "marriage": {
            const names = detail.spouses.map((spouse) => spouse.name);
            return {
                kind: "marriage",
                kicker: "Marriage",
                title: names.length ? names.join(" & ") : detail.marriage.id,
                revealId: detail.marriage.id,
                kin: null,
                fields: marriageFields(detail.marriage),
                sections: [
                    ...section("Spouses", detail.spouses.map((s) => personRow(s))),
                    ...section("Children", detail.children.map(linkedRow)),
                ],
            };
        }
        case "adoption":
            return {
                kind: "adoption",
                kicker: "Adoption",
                title: detail.child.name,
                // An adoption link has no entity id, so reveal targets the child.
                revealId: detail.child.id,
                kin: null,
                fields: adoptionFields(detail.adoption),
                sections: [
                    ...section("Child", [personRow(detail.child)]),
                    ...section(
                        "Adoptive parents",
                        detail.parents.map((parent) => personRow(parent)),
                    ),
                ],
            };
    }
}

/** Label on the header control that reveals the entity in the source editor. */
export const REVEAL_LABEL = "Reveal in editor";

/**
 * The mounted panel. Open means "a detail answer is on screen"; a selection
 * whose answer has not arrived (or whose host ships no engine) leaves it shut.
 */
export interface DetailPanel {
    show(detail: EntityDetail, kin: KinListState | null): void;
    close(): void;
    /** The panel's viewport box while open, else `null`. */
    box(): ScreenBox | null;
    readonly isOpen: boolean;
}

/**
 * Mount a panel into `host` — the float layer's **edge dock**
 * (`#kul-region-float-dock`), which is the placement for chrome that hugs a
 * stage edge (ADR-0038, split in two by ADR-0042). Deliberately *not* the
 * layer's other placement: entity-anchored floats position themselves against
 * a card's screen box, and this panel follows the selection rather than any
 * card. The dock owns the edge, the inset and the stacking; the panel declares
 * none of them, which is what makes a collision with other chrome
 * unrepresentable.
 *
 * Rows carry their person id in a closure rather than on a `data-*` attribute:
 * the widget must have no attribute vocabulary of its own to read back, and the
 * text is set with `textContent`, so an author-written name can never be markup.
 */
export function createDetailPanel(args: {
    host: HTMLElement;
    /**
     * The pack the panel's phrased words render in, read **at render time**
     * rather than captured once. That is what makes the locale toggle's
     * existing `refresh()` wire sufficient: a flip changes what this returns,
     * and the next draw is in the new language (ADR-0041, ADR-0043). #302's
     * lens takes the same shape of seam.
     */
    pack(): LanguagePack;
    onSelectPerson(id: string): void;
    onReveal(id: string): void;
    /** The reader clicked the Explore-kin header. */
    onToggleKin(): void;
    /** The reader clicked a kin-set row. */
    onSelectKinSet(setId: string): void;
}): DetailPanel {
    const { host, pack, onSelectPerson, onReveal, onToggleKin, onSelectKinSet } = args;
    let panel: HTMLElement | null = null;

    function close(): void {
        if (panel) {
            panel.remove();
            panel = null;
        }
    }

    function show(detail: EntityDetail, kin: KinListState | null): void {
        const model = buildDetailPanel(detail, { pack: pack(), kin });
        close();
        const el = document.createElement("div");
        el.className = "kul-query-panel";

        const header = document.createElement("div");
        header.className = "kul-query-panel-header";
        const heading = document.createElement("div");
        heading.className = "kul-query-panel-heading";
        const kicker = document.createElement("span");
        kicker.className = "kul-query-panel-kicker";
        kicker.textContent = model.kicker;
        const title = document.createElement("span");
        title.className = "kul-query-panel-title";
        title.textContent = model.title;
        heading.appendChild(kicker);
        heading.appendChild(title);
        const reveal = document.createElement("button");
        reveal.type = "button";
        reveal.className = "kul-query-panel-reveal";
        reveal.textContent = REVEAL_LABEL;
        reveal.addEventListener("click", () => onReveal(model.revealId));
        header.appendChild(heading);
        header.appendChild(reveal);
        el.appendChild(header);

        if (model.fields.length) {
            const fields = document.createElement("div");
            fields.className = "kul-query-panel-fields";
            for (const field of model.fields) {
                const label = document.createElement("span");
                label.className = "kul-query-panel-label";
                label.textContent = field.label;
                const value = document.createElement("span");
                value.className = "kul-query-panel-value";
                value.textContent = field.value;
                fields.appendChild(label);
                fields.appendChild(value);
            }
            el.appendChild(fields);
        }

        for (const modelSection of model.sections) {
            const group = document.createElement("div");
            group.className = "kul-query-panel-section";
            const heading = document.createElement("div");
            heading.className = "kul-query-panel-section-title";
            heading.textContent = modelSection.title;
            group.appendChild(heading);
            for (const row of modelSection.rows) {
                const line = document.createElement("div");
                line.className = "kul-query-panel-row";
                if (row.person) {
                    const person = row.person;
                    const link = document.createElement("button");
                    link.type = "button";
                    link.className = "kul-query-panel-person";
                    link.textContent = person.name;
                    link.addEventListener("click", () => onSelectPerson(person.id));
                    line.appendChild(link);
                } else if (row.text) {
                    const text = document.createElement("span");
                    text.className = "kul-query-panel-row-text";
                    text.textContent = row.text;
                    line.appendChild(text);
                }
                if (row.note) {
                    const rowNote = document.createElement("span");
                    rowNote.className = "kul-query-panel-note";
                    rowNote.textContent = row.note;
                    line.appendChild(rowNote);
                }
                group.appendChild(line);
            }
            el.appendChild(group);
        }

        if (model.kin) {
            el.appendChild(kinListElement(model.kin));
        }

        host.appendChild(el);
        panel = el;
    }

    /**
     * The Explore-kin list. The header opens and closes it; every row is a
     * button that paints its answer on the tree.
     *
     * Row identity travels in a closure, exactly as a person row's does — the
     * widget stamps no `data-*` of its own to read back (ADR-0042).
     */
    function kinListElement(model: KinListModel): HTMLElement {
        const group = document.createElement("div");
        group.className = "kul-query-panel-section kul-kin";

        const header = document.createElement("button");
        header.type = "button";
        header.className = "kul-kin-header";
        const title = document.createElement("span");
        title.textContent = model.title;
        const arrow = document.createElement("span");
        arrow.className = "kul-kin-arrow";
        arrow.textContent = model.open ? "▾" : "▸";
        header.appendChild(title);
        header.appendChild(arrow);
        header.addEventListener("click", onToggleKin);
        group.appendChild(header);

        if (!model.open) {
            return group;
        }

        const list = document.createElement("div");
        list.className = "kul-kin-list";
        for (const row of model.rows) {
            const button = document.createElement("button");
            button.type = "button";
            button.className = row.active ? "kul-kin-row kul-kin-row-active" : "kul-kin-row";
            const label = document.createElement("span");
            label.className = "kul-kin-label";
            label.textContent = row.label;
            button.appendChild(label);
            if (row.terms.length) {
                const gloss = document.createElement("span");
                gloss.className = "kul-kin-terms";
                gloss.textContent = row.terms.join(" · ");
                // Per-element `lang`, like every other phrased slot: the chrome
                // around it stays English while the terms inside need the
                // browser to pick a font that can render the script (ADR-0033).
                gloss.setAttribute("lang", pack().code);
                button.appendChild(gloss);
            }
            const count = document.createElement("span");
            count.className =
                row.count === 0 ? "kul-kin-count kul-kin-count-zero" : "kul-kin-count";
            // An unanswered count is a placeholder, never a `0`: a zero is an
            // answer and the surface must not show one it has not been given.
            count.textContent = row.count === null ? "·" : String(row.count);
            button.appendChild(count);
            const setId = row.id;
            button.addEventListener("click", () => onSelectKinSet(setId));
            list.appendChild(button);
        }
        group.appendChild(list);
        return group;
    }

    return {
        show,
        close,
        box() {
            if (!panel || typeof panel.getBoundingClientRect !== "function") {
                return null;
            }
            const rect = panel.getBoundingClientRect();
            return {
                left: rect.left,
                top: rect.top,
                right: rect.right,
                bottom: rect.bottom,
            };
        },
        get isOpen() {
            return panel !== null;
        },
    };
}
