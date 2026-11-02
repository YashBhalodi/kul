# Deploying the site

The site at **https://kullang.dev** is the static build of `apps/web`, hosted on Netlify. GitHub Actions builds, tests and uploads it. Netlify only hosts it and never builds.

## How a deploy runs

The `deploy` job in [`.github/workflows/web.yml`](../.github/workflows/web.yml) runs after the `web` job has built the site and passed its gates (check, build, e2e on three browsers, the screenshot tour and Lighthouse). It downloads that exact build (the `site` artifact, `apps/web/dist/client`) and deploys it with a pinned `netlify-cli` through `npx`:

| Event | Deploy | URL |
| --- | --- | --- |
| Pull request | `netlify deploy --no-build --alias pr-<n>` | `https://pr-<n>--kullang-app.netlify.app`, posted as one PR comment that later pushes update |
| Push to `main` | `netlify deploy --no-build --prod` | `https://kullang.dev` (the job smoke-checks the deploy's own permalink) |

After every deploy, [`scripts/smoke-deployed.sh <url>`](../scripts/smoke-deployed.sh) checks the deployed site with curl, and a failing check fails the workflow. It checks that:

- `/`, `/docs`, `/llms.txt` and `/app/anything` answer `200` (`/docs` redirects to `/docs/` on the same host);
- the pages carry the Content-Security-Policy and Cache-Control headers;
- the `.wasm` named in the service worker's precache list is served as `application/wasm`, immutable;
- `/sw.js` is served with `Cache-Control: no-cache`.

The job reads two repository secrets, `NETLIFY_AUTH_TOKEN` and `NETLIFY_SITE_ID`. When either is missing (on a fork, or before setup), its first step logs `Netlify deploy skipped: NETLIFY_AUTH_TOKEN or NETLIFY_SITE_ID is not set …` as a notice, the deploy, smoke and comment steps show as skipped, and the workflow stays green.

Version tags don't touch the site. They release the CLI binaries only ([`release.yml`](../.github/workflows/release.yml)).

## The Netlify site

| | |
| --- | --- |
| Site name | `kullang-app` (the name `kullang` was taken on Netlify) |
| Site ID | `25b6d370-f9d0-4e17-8ea4-fd6b411388bb` (the value for `NETLIFY_SITE_ID`) |
| Team | Tempest Tech |
| Production host | `kullang-app.netlify.app`, which redirects to `https://kullang.dev` |
| Admin | https://app.netlify.com/projects/kullang-app |
| Builds | None. No repository is linked, so Netlify has nothing to build. |

## The rules Netlify applies

- **`_redirects`** is static, in [`apps/web/public/_redirects`](../apps/web/public/_redirects). `https://www.kullang.dev/*` and `https://kullang-app.netlify.app/*` redirect `301!` to `https://kullang.dev/:splat`. The `!` makes them apply to existing files too. Deploy previews (`pr-<n>--kullang-app.netlify.app`) and deploy permalinks (`<deploy-id>--kullang-app.netlify.app`) are different hosts, so they aren't redirected. `/app/*` is served `/_shell.html` with status `200`.
- **`_headers`** is generated after the build by [`apps/web/scripts/finalize-dist.mjs`](../apps/web/scripts/finalize-dist.mjs). Every HTML page gets a Content-Security-Policy that hashes exactly its inline scripts, plus `no-cache`. `/app` and `/app/*` get the shell's policy. Content-hashed `/assets/*` are immutable for a year, `*.wasm` is `application/wasm`, the `.md` and `.txt` variants are typed as text, and every response carries the security headers.
- [`apps/web/scripts/deploy-rules.mjs`](../apps/web/scripts/deploy-rules.mjs) checks both files on every build (through `check-dist`). Its test, `deploy-rules.test.mjs`, parses `public/_redirects` and proves that the canonical hosts redirect, that previews and permalinks don't, and that each header rule holds. If the site or the domain is renamed, change `_redirects` and the host constants in `deploy-rules.mjs` together.
- [`apps/web/scripts/serve-dist.mjs`](../apps/web/scripts/serve-dist.mjs) serves a build locally with the same semantics, and the e2e suites, Lighthouse and the local smoke check run against it. It matches a real Netlify deploy in the details as well. A directory redirect (`/docs` → `/docs/`, `/docs/index` → `/docs/`) carries none of the `_headers` headers, while a `_redirects` redirect carries them. A host rule's `:splat` keeps the request's trailing slash. serve-dist does the same.

To check a build locally:

```sh
just web-build
node apps/web/scripts/serve-dist.mjs --port 4173 &
scripts/smoke-deployed.sh http://127.0.0.1:4173
```

## Setup steps for the maintainer

The site already exists (see above), so these steps finish the setup. **The order matters: the domain must be live, with its certificate issued, before the secrets go in.** `_redirects` force-redirects (`301!`) every request to `kullang-app.netlify.app` to `https://kullang.dev`, so once the secrets exist, the first push to `main` publishes a production deploy that can only be reached through `kullang.dev`. If the domain does not answer yet, production is unreachable, and nothing in CI notices: the workflow smoke-checks the deploy's permalink (`<id>--kullang-app.netlify.app`), which no rule redirects, so it passes either way.

1. **Buy the domain.** Register `kullang.dev` at Cloudflare Registrar (at cost) or Porkbun, and optionally `kul.page`. `.dev` is on the HSTS preload list, so it only works over HTTPS, which Netlify's automatic certificates provide.
2. **Keep Netlify builds off.** The site `kullang-app` has no linked repository, so it never builds. If a repository is ever linked, go to *Project configuration → Build & deploy → Continuous deployment → Build settings* and choose **Stop builds**. Deploys come only from the CLI.
3. **Point the domain at Netlify, and wait for the certificate.** In the site's *Domain management*, add `kullang.dev` and make it the primary domain. Netlify adds `www.kullang.dev` with it, and the `www` redirect rule needs that host assigned to the site. At the registrar's DNS:
   - apex (`@`): an `ALIAS`/`ANAME` or flattened `CNAME` record to `apex-loadbalancer.netlify.com` (or, failing those, an `A` record to `75.2.60.5`);
   - `www`: a `CNAME` record to `kullang-app.netlify.app`.

   Or delegate the domain to Netlify DNS instead. Wait until Netlify shows the certificate as provisioned for both hosts. Before the first production deploy the site answers 404, but the TLS handshake must already succeed:

   ```sh
   curl -sI https://kullang.dev/ | head -1       # an HTTP status line, not a TLS or DNS error
   curl -sI https://www.kullang.dev/ | head -1
   ```
4. **Create the token and the secrets.** Only now. In Netlify, go to *User settings → Applications → Personal access tokens* and create a token (an expiry date is fine; renew the secret when it lapses). Then run:

   ```sh
   gh secret set NETLIFY_AUTH_TOKEN   # paste the token
   gh secret set NETLIFY_SITE_ID --body 25b6d370-f9d0-4e17-8ea4-fd6b411388bb
   ```

   The next pull-request run deploys a preview and comments its URL; the next push to `main` deploys production. After that first production deploy, check the domain itself (CI does not):

   ```sh
   scripts/smoke-deployed.sh https://kullang.dev
   curl -sI https://www.kullang.dev/docs/ | grep -i '^location'   # https://kullang.dev/docs/
   curl -sI https://kullang-app.netlify.app/ | grep -i '^location' # https://kullang.dev/
   ```
5. **Optionally, add `kul.page`.** Add it to the site as a domain alias with the same DNS records. Then add `https://kul.page/*  https://kullang.dev/:splat  301!` (and the `www.kul.page` form, if wanted) to `apps/web/public/_redirects`, and add the host to `ALIAS_HOSTS` in `deploy-rules.mjs`.
6. **Retire the old publishing secrets and listings.**
   1. Delete the repository secrets that no workflow reads: `NPM_TOKEN` and the two extension-registry tokens. `gh secret list` names them; run `gh secret delete <name>` for each. Keep `NETLIFY_AUTH_TOKEN` and `NETLIFY_SITE_ID`.
   2. Visual Studio Marketplace: unpublish or deprecate the extension `YashBhalodi.kul` from the publisher's manage page (https://marketplace.visualstudio.com/manage/publishers/yashbhalodi).
   3. Open VSX: deprecate or remove `YashBhalodi/kul` from the namespace settings on open-vsx.org.
   4. npm: `npm deprecate @kullang/wasm "Kul runs in the browser at https://kullang.dev; this package is no longer published"`.
