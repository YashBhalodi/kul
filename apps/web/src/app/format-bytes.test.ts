import { describe, expect, it } from "vitest"

import { formatBytes } from "./format-bytes"

describe("formatBytes", () => {
  it("uses decimal units, with a decimal place below a hundred", () => {
    expect(formatBytes(1)).toBe("1 byte")
    expect(formatBytes(512)).toBe("512 bytes")
    expect(formatBytes(3_100)).toBe("3.1 kB")
    expect(formatBytes(42_000_000)).toBe("42 MB")
    expect(formatBytes(10_700_000_000)).toBe("10.7 GB")
    expect(formatBytes(2_500_000_000_000_000)).toBe("2,500 TB")
  })
})
