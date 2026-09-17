# Prototype 340 — authoring shell

**Throwaway.** Answers: what does authoring a vault-backed Kul project feel like in a browser window?

**A (workbench) and B (tree stage) were discarded.** The remaining shell is gallery-then-studio: the shelf is its own screen; opening a Kul project drops you into a studio whose right pane is always the kinship tree.

```
just prototype-340
```

Then open <http://localhost:3400/prototype/340-authoring-shell/>

Studio: **left pane** = file explorer + editor (each collapses from a top sidebar icon). **Right pane** is the kinship tree — drag to pan, wheel to zoom, +/−/reset in the corner.

The shelf includes a synthetic **Patel archive** (24 `.kul` files + `kul.yml`) so the file list can be judged at archive scale. The tree picture is borrowed from `examples/09-family-across-a-century` and does not match those files.

Chrome uses the same light surface / ink / muted / border tokens as the committed example `tree.svg`, so the diagram is not sitting on a dark IDE.
