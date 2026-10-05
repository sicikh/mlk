import adapter from "@sveltejs/adapter-static";
import { sveltekit } from "@sveltejs/kit/vite";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";

/**
 * The path the site is served from.
 *
 * A host puts the site where it likes: GitHub Pages serves this one under the name of the
 * repository, while a build on a laptop is served from the root. That is a fact about the
 * host, not about the code, so the build is told it — `BASE_PATH=/mlk pnpm build` — and a
 * build without it is the one `pnpm dev` and the checks see.
 *
 * `paths.base` is typed as `"" | \`/${string}\``, hence the cleaning: the variable is the
 * host's word, not the config's shape.
 */
function siteBase(): "" | `/${string}` {
    const base = process.env.BASE_PATH ?? "";
    return base === "" ? "" : `/${base.replace(/^\/+/, "")}`;
}

export default defineConfig({
    plugins: [
        sveltekit({
            preprocess: vitePreprocess(),
            // The editor is a client-side application: there is no server to render on,
            // and the compiler it talks to is the wasm module the browser loads.
            // A static adapter serves it as files, with a shell page for the paths a host
            // does not have a file for; the editor itself is a single route. The shell is
            // named `404.html`, which is the name the hosts that need one look for.
            adapter: adapter({ fallback: "404.html" }),
            paths: {
                base: siteBase(),
                // The shell page answers for a path the host has no file for, and answers
                // there: with relative paths the page would look for its own assets beside
                // itself, in a directory that does not exist, so they are written from the
                // root of wherever the site is served instead.
                relative: false,
            },
        }),
    ],
    server: {
        fs: {
            // The wasm package is a sibling in the workspace, so in dev its module
            // is served over `/@fs/`, which Vite refuses unless the tree is allowed.
            allow: ["../packages/wasm"],
        },
    },
});
