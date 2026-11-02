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

/**
 * Read the project the preview is showing, in the shape the WASM query surface
 * takes. Every query re-checks the project from these bytes (ADR-0034), so the
 * snapshot must be collected at render time and travel with the picture.
 *
 * Discovery mirrors the `kul` CLI: flat directory, `*.kul` only,
 * subdirectories ignored, lexicographic order. It is the *lenient* posture — an unreadable
 * file is skipped rather than fatal — because a preview that is mid-edit
 * should still draw and still answer.
 */
export async function collectProjectSnapshot(
    reader: ProjectReader,
): Promise<ProjectSnapshot> {
    const entries = await reader.listEntries();
    // `.length > 4`, not a bare `endsWith`: the CLI selects on
    // `path.extension() == "kul"`, which rejects a file named exactly `.kul`
    // (Rust reads that as a dotfile with no extension). Diverging here would
    // let the webview query a file the renderer never read.
    const names = entries.filter((n) => n.endsWith(".kul") && n.length > 4).sort();
    const files: ProjectSnapshot["files"] = [];
    for (const name of names) {
        const source = await reader.readText(name);
        if (source !== null) {
            files.push({ name, source });
        }
    }
    return { files };
}
