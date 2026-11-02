/** The canonical origin the site is published at. */
export const SITE_ORIGIN = "https://kullang.dev"

export const REPO_URL = "https://github.com/YashBhalodi/kul"

/**
 * The app's version, in lockstep with the CLI's (scripts/site-version.mjs
 * sets it at build time).
 */
export const APP_VERSION: string = import.meta.env.KUL_SITE_VERSION
