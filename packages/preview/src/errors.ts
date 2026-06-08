import type { ErrorRow, HostAdapter } from "./types.js";

function escapeHtml(s: string): string {
    return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

export interface ErrorsController {
    set(next: ErrorRow[]): void;
    /** User-driven open/close. No-op on empty. */
    toggle(): void;
    applyVisibility(): void;
    get length(): number;
}

/**
 * Error popover (#203). Last-good SVG persists on renderError; this controller
 * owns the popover content + visibility + the count badge. Wires row-click
 * delegation to {@link HostAdapter.onRevealRequest} with a `location` target.
 */
export function createErrorsController(args: {
    errorButton: HTMLElement | null;
    errorPopover: HTMLElement | null;
    adapter: HostAdapter;
    onReconcile(): void;
}): ErrorsController {
    const { errorButton, errorPopover, adapter, onReconcile } = args;
    let errors: ErrorRow[] = [];
    let errorsVisible = false;

    function renderPopover(): void {
        if (!errorPopover) {
            return;
        }
        if (errors.length === 0) {
            errorPopover.innerHTML = "";
            return;
        }
        const html = errors
            .map((err, i) => {
                const message = String(err.message == null ? "" : err.message);
                const code = err.code ? String(err.code) : "";
                const hasLocation =
                    typeof err.uri === "string" &&
                    !!err.range &&
                    !!err.range.start &&
                    !!err.range.end;
                const line = hasLocation ? err.range!.start.line + 1 : null;
                const col = hasLocation ? err.range!.start.character + 1 : null;
                const locText = hasLocation ? "Line " + line + ", col " + col : "";
                const escMessage = escapeHtml(message);
                const escCode = escapeHtml(code);
                const escLoc = escapeHtml(locText);
                const role = hasLocation ? "button" : "group";
                const tabindex = hasLocation ? "0" : "-1";
                return (
                    '<div class="kul-error-row" role="' +
                    role +
                    '" tabindex="' +
                    tabindex +
                    '" data-error-index="' +
                    i +
                    '"' +
                    (hasLocation ? "" : ' aria-disabled="true"') +
                    ">" +
                    (escCode ? '<span class="kul-error-code">' + escCode + "</span>" : "") +
                    '<span class="kul-error-message">' +
                    escMessage +
                    "</span>" +
                    (escLoc ? '<span class="kul-error-location">' + escLoc + "</span>" : "") +
                    "</div>"
                );
            })
            .join("");
        errorPopover.innerHTML = html;
    }

    function applyVisibility(): void {
        const shouldShow = errorsVisible && errors.length > 0;
        if (errorPopover) {
            errorPopover.hidden = !shouldShow;
        }
        if (errorButton) {
            errorButton.setAttribute("aria-pressed", String(shouldShow));
            const labelText = shouldShow ? "Hide errors" : "Show errors";
            errorButton.setAttribute("aria-label", labelText);
            errorButton.setAttribute("title", labelText);
        }
    }

    function set(next: ErrorRow[]): void {
        errors = Array.isArray(next) ? next.slice() : [];
        if (errorButton) {
            const badge = errorButton.querySelector(".kul-error-count");
            if (badge) {
                badge.textContent = String(errors.length);
            }
        }
        if (errors.length === 0) {
            errorsVisible = false;
        }
        renderPopover();
        onReconcile();
        applyVisibility();
    }

    function toggle(): void {
        if (errors.length === 0) {
            return;
        }
        errorsVisible = !errorsVisible;
        applyVisibility();
    }

    if (errorPopover) {
        errorPopover.addEventListener("click", (event) => {
            const row = (event.target as Element | null)?.closest?.(
                ".kul-error-row[data-error-index]",
            );
            if (!row) {
                return;
            }
            const i = parseInt(row.getAttribute("data-error-index") ?? "", 10);
            const err = errors[i];
            if (!err) {
                return;
            }
            if (
                typeof err.uri === "string" &&
                err.range &&
                err.range.start &&
                err.range.end
            ) {
                adapter.onRevealRequest({
                    kind: "location",
                    uri: err.uri,
                    range: err.range,
                });
            }
        });
    }

    return {
        set,
        toggle,
        applyVisibility,
        get length() {
            return errors.length;
        },
    };
}

/**
 * Mark / clear the stale-overlay on the persisted SVG. A subsequent good render
 * swaps in a fresh SVG so the class never needs explicit clearing there — this
 * helper exists only so a transient error can re-apply it.
 */
export function setStaleSvg(root: HTMLElement | null, isStale: boolean): void {
    const svg = root?.querySelector?.("svg");
    if (!svg) {
        return;
    }
    if (isStale) {
        svg.classList.add("kul-render-stale");
    } else {
        svg.classList.remove("kul-render-stale");
    }
}
