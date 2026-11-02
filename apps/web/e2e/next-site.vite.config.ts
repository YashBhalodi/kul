// The site's Vite config, writing to the next site's directory (sites.ts).
// The output directory lives in a config file rather than on the command
// line because the prerender re-reads this file for its preview server.

import { mergeConfig } from "vite"

import site from "../vite.config.ts"
import { NEXT_OUT_DIR } from "./sites.ts"

export default mergeConfig(site, { build: { outDir: NEXT_OUT_DIR } })
