// WOFF 1.0 → OpenType. The Open Graph image is rasterised by resvg, which
// reads TrueType and OpenType only; the Inter files the build has are WOFF.
// A WOFF file is the font's tables, each zlib-compressed, behind a header
// (https://www.w3.org/TR/WOFF/), so unpacking it is a table copy.

import { inflateSync } from "node:zlib"

const WOFF_SIGNATURE = 0x774f4646 // "wOFF"
const WOFF_HEADER_BYTES = 44
const WOFF_ENTRY_BYTES = 20
const SFNT_HEADER_BYTES = 12
const SFNT_ENTRY_BYTES = 16

interface TableEntry {
  tag: number
  offset: number
  compressedLength: number
  length: number
  checksum: number
}

const padded = (length: number) => (length + 3) & ~3

/** The OpenType font inside a WOFF 1.0 file. */
export function woffToSfnt(woff: Uint8Array): Uint8Array {
  const view = new DataView(woff.buffer, woff.byteOffset, woff.byteLength)
  if (view.getUint32(0) !== WOFF_SIGNATURE) {
    throw new Error("not a WOFF 1.0 file")
  }
  const flavor = view.getUint32(4)
  const count = view.getUint16(12)
  const tables: Array<TableEntry> = []
  for (let index = 0; index < count; index += 1) {
    const at = WOFF_HEADER_BYTES + index * WOFF_ENTRY_BYTES
    tables.push({
      tag: view.getUint32(at),
      offset: view.getUint32(at + 4),
      compressedLength: view.getUint32(at + 8),
      length: view.getUint32(at + 12),
      checksum: view.getUint32(at + 16),
    })
  }

  const directoryEnd = SFNT_HEADER_BYTES + count * SFNT_ENTRY_BYTES
  const size = tables.reduce(
    (total, table) => total + padded(table.length),
    directoryEnd
  )
  const font = new Uint8Array(size)
  const out = new DataView(font.buffer)
  let searchRange = 1
  let entrySelector = 0
  while (searchRange * 2 <= count) {
    searchRange *= 2
    entrySelector += 1
  }
  out.setUint32(0, flavor)
  out.setUint16(4, count)
  out.setUint16(6, searchRange * 16)
  out.setUint16(8, entrySelector)
  out.setUint16(10, count * 16 - searchRange * 16)

  let offset = directoryEnd
  tables.forEach((table, index) => {
    const stored = woff.subarray(
      table.offset,
      table.offset + table.compressedLength
    )
    const data =
      table.compressedLength < table.length ? inflateSync(stored) : stored
    if (data.length !== table.length) {
      throw new Error("a WOFF table does not unpack to its stated length")
    }
    const at = SFNT_HEADER_BYTES + index * SFNT_ENTRY_BYTES
    out.setUint32(at, table.tag)
    out.setUint32(at + 4, table.checksum)
    out.setUint32(at + 8, offset)
    out.setUint32(at + 12, table.length)
    font.set(data, offset)
    offset += padded(table.length)
  })
  return font
}
