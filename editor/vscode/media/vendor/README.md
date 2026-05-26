# Vendored third-party webview assets

These files are committed to source, not pulled from `package.json`. The
preview webview loads them as static assets via `webview.asWebviewUri(...)`;
esbuild does not bundle them.

## svg-pan-zoom

- **Package:** `svg-pan-zoom`
- **Version:** `3.6.2`
- **Source:** <https://github.com/bumbu/svg-pan-zoom> (npm: <https://www.npmjs.com/package/svg-pan-zoom>)
- **License:** BSD-2-Clause — see [`LICENSE`](./LICENSE).
- **File:** `dist/svg-pan-zoom.min.js` (the upstream minified build; its license
  banner comment is preserved at the top of the file).

### Upgrading

```sh
curl -sSL -o dist/svg-pan-zoom.min.js https://unpkg.com/svg-pan-zoom@<version>/dist/svg-pan-zoom.min.js
curl -sSL -o LICENSE                  https://unpkg.com/svg-pan-zoom@<version>/LICENSE
```

Then bump the version recorded above and re-run `npm test` to confirm the
preview-HTML contract still holds.
