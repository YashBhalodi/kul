// The quiet toast — a transient-notification-region member (ADR-0038).
//
// Its one job in this slice is honest emptiness: a kin set that comes back with
// nothing says so in words, once, and goes away. Deliberately **not modal and
// not an error** — an empty kin set is an answer, so it must not borrow the
// error popover's chrome or block the reader from asking the next question
// (PRD-0006's honesty stance, #276 point 8).
//
// One toast at a time. A second message replaces the first rather than stacking:
// the reader clicked another row, and two contradictory absences on screen would
// read as a log rather than as an answer.
//
// ACCESSIBILITY. Per ADR-0036 this chrome carries no `role`, no `aria-*` and no
// `:focus-visible`; the notify region's existing sync hint is its neighbour and
// carries none either.

/** How long a message stays up before it fades itself out. */
export const TOAST_DURATION_MS = 6000;

export interface Toaster {
    /** Replace whatever is showing with `message`, and start its clock. */
    show(message: string): void;
    /** Take it down now. */
    clear(): void;
    dispose(): void;
}

/**
 * Mount a toaster into `region` — `#kul-region-notify`, which owns the corner,
 * the inset and the stacking. A `null` region (a host that mounted no notify
 * region) yields a working no-op rather than a crash: a toast is a courtesy,
 * and its absence must not take a query down with it.
 */
export function createToaster(region: HTMLElement | null): Toaster {
    let element: HTMLElement | null = null;
    let timer: ReturnType<typeof setTimeout> | null = null;

    function clear(): void {
        if (timer !== null) {
            clearTimeout(timer);
            timer = null;
        }
        if (element) {
            element.remove();
            element = null;
        }
    }

    return {
        show(message) {
            if (!region) {
                return;
            }
            clear();
            const el = document.createElement("div");
            el.className = "kul-toast";
            el.textContent = message;
            region.appendChild(el);
            element = el;
            timer = setTimeout(clear, TOAST_DURATION_MS);
        },
        clear,
        dispose: clear,
    };
}
