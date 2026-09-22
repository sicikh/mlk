import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/**
 * The path the site is served from.
 *
 * A host puts the site where it likes: GitHub Pages serves this one under the name of the
 * repository, while a build on a laptop is served from the root. That is a fact about the
 * host, not about the code, so the build is told it — `BASE_PATH=/mlk pnpm build` — and a
 * build without it is the one `pnpm dev` and the checks see.
 */
const base = process.env.BASE_PATH ?? "";

/** @type {import('@sveltejs/kit').Config} */
const config = {
    preprocess: vitePreprocess(),
    kit: {
        // The editor is a client-side application: there is no server to render on,
        // and the compiler it talks to is the wasm module the browser loads.
        // A static adapter serves it as files, with a shell page for the paths a host
        // does not have a file for; the editor itself is a single route. The shell is
        // named `404.html`, which is the name the hosts that need one look for.
        adapter: adapter({ fallback: "404.html" }),
        paths: {
            base,
            // The shell page answers for a path the host has no file for, and answers
            // there: with relative paths the page would look for its own assets beside
            // itself, in a directory that does not exist, so they are written from the
            // root of wherever the site is served instead.
            relative: false,
        },
    },
};

export default config;
