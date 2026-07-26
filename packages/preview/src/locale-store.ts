/**
 * Where the reader's language choice survives a preview reopen.
 *
 * The choice is **webview-local** (ADR-0033): it belongs to this panel, not to
 * the document (a `language:` field in `kul.yml` would bind every third-party
 * Kul consumer for a preview UX feature) and not to a user setting (which the
 * store can graduate to additively if per-panel turns out to be the wrong
 * grain).
 *
 * It is an interface rather than a direct `getState` / `setState` call for one
 * reason: `HostAdapter` stays untouched and the preview stays host-agnostic.
 * The in-memory implementation below is the default, and any embedding that
 * has durable state — VSCode does, via `acquireVsCodeApi().getState()` — hands
 * in its own without the chrome knowing which it got. Two real
 * implementations, so this is a seam rather than a hypothetical one.
 */
export interface LocaleStore {
    /** The persisted BCP-47 subtag, or `null` when nothing has been chosen. */
    read(): string | null;
    write(code: string): void;
}

/**
 * The default store: the choice lives as long as the mounted chrome does. A
 * host that wants it to outlive a reopen supplies its own.
 */
export function createMemoryLocaleStore(initial: string | null = null): LocaleStore {
    let held = initial;
    return {
        read: () => held,
        write: (code) => {
            held = code;
        },
    };
}
