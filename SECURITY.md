# Security policy

## Reporting a vulnerability

Please **do not** open a public issue for security reports. Use GitHub's private vulnerability reporting:

→ https://github.com/YashBhalodi/kul/security/advisories/new

You'll get an acknowledgement within a few business days. Once the issue is understood and a fix is in flight, we'll coordinate disclosure timing with you and credit you in the advisory unless you'd rather stay anonymous.

## Scope

In scope:

- The `kul` CLI, `kul-lsp` language server, and the `@kullang/wasm` npm package
- Anything that lets a malicious `.kul` document or LSP message exfiltrate data, escape the validator's strict-on-errors contract, or crash the server in a way that affects unrelated documents
- The release pipeline (anything that lets an unauthorized party publish to npm or Open VSX under our names)

Out of scope:

- Bugs that are functional defects rather than security weaknesses — please file those as normal issues
- Theoretical concerns without a reproducer

## Supported versions

KulLang is pre-1.0. Only the latest published version (`v0.x.0`) gets fixes; older `v0.x.0` lines are not patched. Once `v1.0.0` ships this policy will be revisited.
