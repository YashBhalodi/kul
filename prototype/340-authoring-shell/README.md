# Prototype 340 — authoring shell

**Throwaway.** Answers: what does authoring a vault-backed Kul project feel like in a browser window?

Two variants, switchable via `?variant=`. **B (tree stage) was discarded.**

```
just prototype-340
```

Then open <http://localhost:3400/prototype/340-authoring-shell/?variant=A>

| Key | Name | Structure |
| --- | --- | --- |
| A | Workbench | Two independently collapsible left rails (Shelf, Files); editor and kinship tree split the rest. |
| C | Gallery then studio | Shelf is its own screen (cards). Opening a Kul project drops the shelf and shows a **collapsible left Files rail** + editor/tree. No horizontal file tabs. |

The shelf includes a synthetic **Patel archive** (24 `.kul` files + `kul.yml`) so C's tab strip can be judged at archive scale. The tree picture is borrowed from `examples/09-family-across-a-century` and does not match those files.

Workbench chrome uses the same light surface / ink / muted / border tokens as the committed example `tree.svg`, so the diagram is not sitting on a dark IDE.
