import type { ProjectSnapshot } from "@kullang/preview";

/**
 * The project directory, as the snapshot collector reads it. One interface, so
 * the collector's rules can be tested without a workspace: the extension backs
 * it with `vscode.workspace.fs` plus the open editor buffers.
 */
export interface ProjectReader {
    /** Bare names of the entries directly inside the project directory. */
    listEntries(): Promise<string[]>;
    /** UTF-8 text of `name`, or `null` when it cannot be read. */
    readText(name: string): Promise<string | null>;
}

const MANIFEST_NAME = "kul.yml";

/**
 * The `kul:` version the manifest declares, or `""` when it declares none.
 *
 * `kul.yml` is a one-field manifest (ADR-0013) and the WASM bridge takes a
 * *typed* manifest rather than YAML bytes, so this reads the one scalar it
 * needs instead of pulling a YAML parser into the extension bundle. The bridge
 * does not re-validate the version it is handed (`KUL-M0x` is raised by the
 * YAML path the CLI and LSP take), so the field is inert for a query — which
 * is exactly why an unreadable manifest reports `""` rather than a plausible
 * default: reporting what was found beats inventing a version on the author's
 * behalf, and costs the query nothing either way.
 */
export function manifestVersion(yaml: string | null): string {
    if (yaml === null) {
        return "";
    }
    // A leading `kul:` at column zero, its value optionally quoted, comments
    // and indented (i.e. nested) keys ignored.
    const match = yaml.match(/^kul\s*:\s*(?:"([^"]*)"|'([^']*)'|([^\s#]+))/m);
    if (!match) {
        return "";
    }
    return match[1] ?? match[2] ?? match[3] ?? "";
}

/**
 * Read the project the preview is showing, in the shape the WASM query surface
 * takes. Every query re-checks the project from these bytes (ADR-0034), so the
 * snapshot must be collected at render time and travel with the picture.
 *
 * Discovery mirrors `kul-loader`: flat directory, `*.kul` only, subdirectories
 * ignored, lexicographic order. It is the *lenient* posture — an unreadable
 * file is skipped rather than fatal — because a preview that is mid-edit
 * should still draw and still answer.
 */
export async function collectProjectSnapshot(
    reader: ProjectReader,
): Promise<ProjectSnapshot> {
    const entries = await reader.listEntries();
    const names = entries.filter((n) => n.endsWith(".kul")).sort();
    const files: ProjectSnapshot["files"] = [];
    for (const name of names) {
        const source = await reader.readText(name);
        if (source !== null) {
            files.push({ name, source });
        }
    }
    return {
        files,
        manifest: { kul: manifestVersion(await reader.readText(MANIFEST_NAME)) },
    };
}
