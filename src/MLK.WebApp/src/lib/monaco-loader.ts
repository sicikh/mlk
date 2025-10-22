import type monaco from 'monaco-editor'

export type Monaco = typeof monaco

class MonacoLoader {
    private monaco: Monaco
    loaded: Promise<Monaco>
    toDispose: monaco.IDisposable[] = []

    constructor() {
        this.load()
    }

    async load(): Promise<Monaco> {
        if (this.loaded) return this.loaded

        this.loaded = import('monaco-editor')
        this.monaco = await this.loaded

        self.MonacoEnvironment = {
            getWorker: async function (_, label) {
                const worker = await import('monaco-editor/esm/vs/editor/editor.worker?worker')
                return new worker.default()
            }
        }

        return this.monaco
    }
}

export const Monaco = new MonacoLoader()
