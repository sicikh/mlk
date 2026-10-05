import { defineConfig } from "vitest/config";

/**
 * The tests of the pure modules of the editor.
 *
 * The editor itself is checked by `svelte-check` and by the browser check; the modules here
 * are values in and values out, so they are tested as such, without a browser and without the
 * SvelteKit plugin of `vite.config.ts`.
 */
export default defineConfig({
    test: {
        include: ["src/**/*.test.ts"],
    },
});
