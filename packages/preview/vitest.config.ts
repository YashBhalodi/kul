import { defineConfig } from "vitest/config";

export default defineConfig({
    test: {
        environment: "jsdom",
        include: ["tests/**/*.test.ts"],
        /**
         * The pack suites walk an enumeration that **widens itself from the
         * registered packs** (ADR-0039): the facets a path does not determine
         * are enumerated over the values some pack actually keys. Gujarati is
         * the first pack to key `seniority` and `apexSeniority`, which
         * multiplies the coverage enumeration roughly tenfold (≈22k → ≈219k
         * descriptors) the moment `gu` is registered — by design, since that is
         * how a new language inherits coverage over its own facets without
         * editing a test.
         *
         * No assertion changed; only the wall clock did, past Vitest's 5 s
         * default. The ceiling is raised here rather than on the suite so the
         * pack tests stay untouched by the packs they scan, which is the
         * property the additivity promise is reviewed as (ADR-0041).
         */
        testTimeout: 30_000,
    },
});
