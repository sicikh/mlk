import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite";

export default defineConfig({
    plugins: [sveltekit()],
    server: {
        fs: {
            // The wasm package is a sibling in the workspace, so in dev its module
            // is served over `/@fs/`, which Vite refuses unless the tree is allowed.
            allow: ["../packages/wasm"],
        },
    },
});
