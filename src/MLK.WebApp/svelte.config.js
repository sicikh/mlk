import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { resolve } from 'path';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// Consult https://svelte.dev/docs/kit/integrations
	// for more information about preprocessors
	preprocess: vitePreprocess(),

	kit: {
		adapter: adapter({
            pages: 'build',
            assets: 'build',
            fallback: '200.html',
            precompress: false,
            strict: true,
        }),
        alias: {
            $src: resolve('./src/'),
            $components: resolve('./src/components/'),
            $stores: resolve('./src/stores/'),
        }
	}
};

export default config;
