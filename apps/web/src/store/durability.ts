// Whether the browser keeps this origin's storage, and how much it holds.
// Without persistence a browser may clear the projects when space runs low.

export type Persistence = "persisted" | "best-effort" | "unsupported"

export interface StorageUsage {
  /** Bytes used by this origin. */
  usage: number
  /** Bytes this origin may use. */
  quota: number
}

function storage(): StorageManager | undefined {
  return typeof navigator === "undefined" ? undefined : navigator.storage
}

export async function readPersistence(): Promise<Persistence> {
  const manager = storage()
  if (!manager?.persisted) return "unsupported"
  try {
    return (await manager.persisted()) ? "persisted" : "best-effort"
  } catch {
    return "unsupported"
  }
}

/** Ask the browser to keep this origin's storage; resolves to the outcome. */
export async function requestPersistence(): Promise<Persistence> {
  const manager = storage()
  if (!manager?.persist) return "unsupported"
  try {
    return (await manager.persist()) ? "persisted" : "best-effort"
  } catch {
    return "best-effort"
  }
}

export async function readStorageUsage(): Promise<StorageUsage | undefined> {
  const manager = storage()
  if (!manager?.estimate) return undefined
  try {
    const { usage, quota } = await manager.estimate()
    if (usage === undefined || quota === undefined) return undefined
    return { usage, quota }
  } catch {
    return undefined
  }
}
