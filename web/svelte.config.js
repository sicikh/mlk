import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import('@sveltejs/kit').Config} */
const config = {
    preprocess: vitePreprocess(),
    kit: {
        // The editor is a client-side application: there is no server to render on,
        // and the compiler it talks to is the wasm module the browser loads.
        // A static adapter serves it as files, with a shell page for the paths a host
        // does not have a file for; the editor itself is a single route.
        adapter: adapter({ fallback: "200.html" }),
    },
};

export default config;
