# Security policy

## Reporting a vulnerability

Please **do not** open a public issue for security reports. Use GitHub's private vulnerability reporting:

→ https://github.com/YashBhalodi/kul/security/advisories/new

You'll get an acknowledgement within a few business days. Once the issue is understood and a fix is in flight, we'll coordinate disclosure timing with you and credit you in the advisory unless you'd rather stay anonymous.

## Scope

In scope:

- The web app at https://kullang.dev, its service worker, and the site's headers (`_headers`, `_redirects`)
- Anything that makes the app send a request off its own origin, read or write another project or another site's data, or run script from a `.kul` file, an imported folder or `.zip`, a downloaded SVG, or a docs page
- The `kul` CLI: anything that lets a malicious `.kul` project read or write outside its directory, or escape the strict-on-errors contract of `export` and `query`
- The build and release pipeline: anything that lets an unauthorized party publish a GitHub Release, deploy the site, or change what the deploy serves

Out of scope:

- Bugs that are functional defects rather than security weaknesses — please file those as normal issues
- Theoretical concerns without a reproducer

## Supported versions

KulLang is pre-1.0. Only the latest published version (`v0.x.0`) gets fixes; older `v0.x.0` lines are not patched. Once `v1.0.0` ships this policy will be revisited.
