// What the details panel says about the selected entity: built from the
// language service's `details` answer alone, so the panel never reads the
// drawing. The panel renders this model; every related person or marriage
// in it is a link that selects that entity.

import type {
  EntityDetail,
  ExportedDate,
  ExportedMarriage,
  ExportedParenthoodLink,
  ExportedPerson,
  LinkedPerson,
} from "kul-wasm"

import type { EntityRef } from "./selection"

export interface DetailsFact {
  label: string
  value: string
}

export interface DetailsLink {
  entity: EntityRef
  label: string
}

export interface DetailsRow {
  link: DetailsLink
  /** Dates and the kind of tie, beside the link. */
  note?: string
  /** A second entity the row is about (a spouse's marriage). */
  also?: DetailsLink
}

export interface DetailsGroup {
  title: string
  rows: Array<DetailsRow>
}

export interface DetailsModel {
  entity: EntityRef
  /** What the entity is: "Person" or "Marriage". */
  kind: string
  title: string
  facts: Array<DetailsFact>
  groups: Array<DetailsGroup>
}

/** A date as the source writes it: `1950`, `1930-03`, `~1925`. */
export function formatDate(date: ExportedDate): string {
  return `${date.circa ? "~" : ""}${date.value}`
}

/** A word from the language (`female`, `divorce`) as an English label. */
export function sentenceCase(word: string): string {
  const spaced = word.replace(/_/g, " ")
  return spaced.charAt(0).toUpperCase() + spaced.slice(1)
}

function personLink(person: ExportedPerson): DetailsLink {
  return { entity: { kind: "person", id: person.id }, label: person.name }
}

function marriageLink(marriage: ExportedMarriage): DetailsLink {
  return { entity: { kind: "marriage", id: marriage.id }, label: "Marriage" }
}

function facts(
  entries: Array<[string, string | undefined]>
): Array<DetailsFact> {
  return entries
    .filter((entry): entry is [string, string] => entry[1] !== undefined)
    .map(([label, value]) => ({ label, value }))
}

function optionalDate(date: ExportedDate | undefined): string | undefined {
  return date && formatDate(date)
}

/** "from 1980 until 1990", "from 1980", or nothing, for an adoption. */
function adoptionDates(link: ExportedParenthoodLink): string | undefined {
  const parts = [
    link.start && `from ${formatDate(link.start)}`,
    link.end && `until ${formatDate(link.end)}`,
  ].filter((part): part is string => !!part)
  return parts.length > 0 ? parts.join(" ") : undefined
}

/** "married 1948", "married 1995, ended 2003 (divorce)". */
function marriageDates(marriage: ExportedMarriage): string | undefined {
  const parts = [
    marriage.start && `married ${formatDate(marriage.start)}`,
    marriage.end &&
      `ended ${formatDate(marriage.end)}${
        marriage.endReason ? ` (${marriage.endReason.replace(/_/g, " ")})` : ""
      }`,
  ].filter((part): part is string => !!part)
  return parts.length > 0 ? parts.join(", ") : undefined
}

function linkedRow(linked: LinkedPerson): DetailsRow {
  const adopted = linked.link.kind === "adoptive"
  const dates = adopted ? adoptionDates(linked.link) : undefined
  return {
    link: personLink(linked.person),
    note: adopted ? ["adopted", dates].filter(Boolean).join(" ") : undefined,
  }
}

function group(title: string, rows: Array<DetailsRow>): Array<DetailsGroup> {
  return rows.length > 0 ? [{ title, rows }] : []
}

function personModel(
  detail: Extract<EntityDetail, { kind: "person" }>
): DetailsModel {
  const { person } = detail
  const birthParents = detail.parents.filter(
    (parent) => parent.link.kind !== "adoptive"
  )
  const adoptiveParents = detail.parents.filter(
    (parent) => parent.link.kind === "adoptive"
  )
  return {
    entity: { kind: "person", id: person.id },
    kind: "Person",
    title: person.name,
    facts: facts([
      ["Gender", sentenceCase(person.gender)],
      ["Born", optionalDate(person.born)],
      ["Died", optionalDate(person.died)],
      ["Family name", person.family],
      ["Given name", person.given],
    ]),
    groups: [
      ...group(
        "Parents",
        birthParents.map((parent) => ({ link: personLink(parent.person) }))
      ),
      ...group(
        "Adopted by",
        adoptiveParents.map((parent) => ({
          link: personLink(parent.person),
          note: adoptionDates(parent.link),
        }))
      ),
      ...group(
        "Marriages",
        detail.marriages.map((tie) => ({
          link: tie.spouse
            ? personLink(tie.spouse)
            : marriageLink(tie.marriage),
          note: marriageDates(tie.marriage),
          also: tie.spouse ? marriageLink(tie.marriage) : undefined,
        }))
      ),
      ...group("Children", detail.children.map(linkedRow)),
    ],
  }
}

function marriageModel(
  detail: Extract<EntityDetail, { kind: "marriage" }>
): DetailsModel {
  const { marriage } = detail
  return {
    entity: { kind: "marriage", id: marriage.id },
    kind: "Marriage",
    title: detail.spouses.map((spouse) => spouse.name).join(" & "),
    facts: facts([
      ["Started", optionalDate(marriage.start)],
      ["Ended", optionalDate(marriage.end)],
      ["End reason", marriage.endReason && sentenceCase(marriage.endReason)],
    ]),
    groups: [
      ...group(
        "Spouses",
        detail.spouses.map((spouse) => ({ link: personLink(spouse) }))
      ),
      ...group("Children", detail.children.map(linkedRow)),
    ],
  }
}

/** The panel's model for an answer from `details`. */
export function detailsModel(detail: EntityDetail): DetailsModel {
  switch (detail.kind) {
    case "person":
      return personModel(detail)
    case "marriage":
      return marriageModel(detail)
  }
}
