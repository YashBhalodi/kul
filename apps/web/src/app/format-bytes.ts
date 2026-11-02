const UNITS = ["kilobyte", "megabyte", "gigabyte", "terabyte"] as const

/** "512 bytes", "3.1 kB", "10.7 GB": decimal units, as storage is sold. */
export function formatBytes(count: number): string {
  if (count < 1000)
    return `${Math.round(count)} ${count === 1 ? "byte" : "bytes"}`
  let value = count / 1000
  let unit = 0
  while (value >= 1000 && unit < UNITS.length - 1) {
    value /= 1000
    unit++
  }
  return new Intl.NumberFormat("en", {
    style: "unit",
    unit: UNITS[unit],
    unitDisplay: "short",
    maximumFractionDigits: value < 100 ? 1 : 0,
  }).format(value)
}
