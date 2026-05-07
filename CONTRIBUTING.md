# Contributing

Thanks for the interest. Quick orientation on what's open right now:

## v0.x: issues only, no external PRs

KulLang is still finding its shape. During the `v0.x` line, **external pull requests are closed** — please file an issue first to describe what you'd like to change, and we'll discuss before any code is written. This keeps the maintainer (currently a single person) from becoming a review bottleneck and avoids the disappointment of a PR that doesn't fit the project's direction.

This will change once `v1.0.0` lands and the language surface stabilizes. Watch the repo for that announcement.

## What's useful right now

- **Bug reports** — wrong behavior, crashes, confusing diagnostics. The issue templates ask for a minimal `.kul` reproducer; please include one.
- **Feature requests / language proposals** — describe the use case, not just the syntax. The [language spec](spec/) and [vision](docs/vision.md) explain what the language is and isn't trying to do — proposals that align with that scope are easier to land.
- **Editor / tooling reports** — VSCode extension issues, `kul-lsp` glitches, `@kullang/wasm` integration breakage in your bundler.

## Security issues

Don't file those as public issues — see [SECURITY.md](SECURITY.md).

## Code of conduct

Be kind and assume good faith. Don't post anything you wouldn't want quoted back to you in a code review.
