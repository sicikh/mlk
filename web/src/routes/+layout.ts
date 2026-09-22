// The editor runs where the compiler does: in the browser.
//
// There is no server to render the pages on, and no server to run the compiler on either —
// the driver is a wasm module the page loads — so nothing is server-side rendered.
export const ssr = false;

// The pages are files, so they can be served from anywhere.
export const prerender = true;
