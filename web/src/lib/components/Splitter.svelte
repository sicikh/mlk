<script lang="ts">
    interface Props {
        /** Which way the handle runs, and which way it is dragged: across the page, or up and down it. */
        direction: "x" | "y";

        /** Where the handle sits in the layout, which is the grid the page lays it out by. */
        style?: string;

        /** How big the panel the handle sizes is now, in pixels. */
        size: number;

        /** How small and how large that panel may be. */
        min: number;
        max: number;

        /**
         * Whether the panel sits on the far side of the handle,
         * so that dragging towards the start of the axis makes it larger.
         */
        flip?: boolean;

        /** What the handle resizes, for whoever cannot see it. */
        label: string;

        /** Called with the size the handle was dragged to. */
        onResize: (size: number) => void;
    }

    let {
        direction,
        style,
        size,
        min,
        max,
        flip = false,
        label,
        onResize,
    }: Props = $props();

    /** Whether a person is dragging the handle right now. */
    let dragging = $state(false);

    /** Where the drag began, and how big the panel was then. */
    let from = { at: 0, size: 0 };

    /** How far one arrow key moves the handle. */
    const step = 16;

    const clamp = (value: number) => Math.min(Math.max(value, min), max);

    /** The place along the axis a pointer is at. */
    const point = (event: PointerEvent) =>
        direction === "x" ? event.clientX : event.clientY;

    function start(event: PointerEvent) {
        const handle = event.currentTarget as HTMLElement;

        // Without this the drag would select the text it passes over.
        event.preventDefault();
        handle.setPointerCapture(event.pointerId);
        dragging = true;
        from = { at: point(event), size };
    }

    function move(event: PointerEvent) {
        if (!dragging) return;

        const delta = (point(event) - from.at) * (flip ? -1 : 1);

        onResize(clamp(from.size + delta));
    }

    function stop(event: PointerEvent) {
        if (!dragging) return;

        (event.currentTarget as HTMLElement).releasePointerCapture(
            event.pointerId,
        );
        dragging = false;
    }

    /** A handle is a control as well: the arrow keys nudge it, by the same step. */
    function nudge(event: KeyboardEvent) {
        const back = direction === "x" ? "ArrowLeft" : "ArrowUp";
        const forth = direction === "x" ? "ArrowRight" : "ArrowDown";
        const way = event.key === back ? -1 : event.key === forth ? 1 : 0;

        if (way === 0) return;

        event.preventDefault();
        onResize(clamp(size + way * step * (flip ? -1 : 1)));
    }
</script>

<!--
    	A separator that takes focus and holds a value is a widget, not a line of decoration:
    	the window splitter pattern. The compiler's a11y rule assumes the latter.
    	https://www.w3.org/WAI/ARIA/apg/patterns/windowsplitter/
    -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
    class="handle {direction}"
    {style}
    class:dragging
    role="separator"
    aria-orientation={direction === "x" ? "vertical" : "horizontal"}
    aria-label={label}
    aria-valuenow={size}
    aria-valuemin={min}
    aria-valuemax={max}
    tabindex={0}
    onpointerdown={start}
    onpointermove={move}
    onpointerup={stop}
    onpointercancel={stop}
    onkeydown={nudge}
></div>

<style>
    .handle {
        position: relative;
        z-index: 1;
        touch-action: none;
    }

    /*
    		The handle lies on the seam rather than in a lane of its own,
    		so that nothing is drawn behind the panels: it is a target, not a wall.
    	*/
    .handle.x {
        left: -4px;
        width: 9px;
        cursor: col-resize;
    }

    .handle.y {
        top: -4px;
        width: 100%;
        height: 9px;
        cursor: row-resize;
    }

    /* A handle is a line of nothing until it is asked for, and then it says so. */
    .handle::after {
        position: absolute;
        content: "";
        background: transparent;
    }

    .handle.x::after {
        inset: 0 4px;
    }

    .handle.y::after {
        inset: 4px 0;
    }

    .handle:hover::after {
        background: var(--border);
    }

    .handle.dragging::after,
    .handle:focus-visible::after {
        background: var(--accent);
    }

    .handle:focus-visible {
        outline: none;
    }
</style>
