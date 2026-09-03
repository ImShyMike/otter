<script lang="ts">
	import { geoNaturalEarth1, geoPath } from 'd3-geo';
	import { feature } from 'topojson-client';
	import isoCountries from 'i18n-iso-countries';
	import worldTopo from 'world-atlas/countries-110m.json' with { type: 'json' };
	import { countryName } from '$lib/utils';

	const { numericToAlpha2 } = isoCountries;

	let {
		data,
		formatValue
	}: {
		data: Record<string, number>;
		formatValue: (value: number) => string;
	} = $props();

	const WIDTH = 960;
	const HEIGHT = 500;
	const BUCKETS = 6;
	const NO_DATA_LIGHT = '#e1e0d9';
	const NO_DATA_DARK = '#2c2c2a';

	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	const topology = worldTopo as any;
	const world = feature(topology, topology.objects.countries) as unknown as {
		features: { id?: string | number; properties?: { name?: string }; geometry: unknown }[];
	};

	const projection = geoNaturalEarth1().fitSize(
		[WIDTH, HEIGHT],
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		world as any
	);
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	const pathGenerator = geoPath(projection as any);

	function alpha2For(id: string | number | undefined): string | null {
		if (id === undefined) return null;
		const numeric = id.toString().padStart(3, '0');
		return numericToAlpha2(numeric) ?? null;
	}

	function lerpHex(a: string, b: string, t: number): string {
		const pa = [1, 3, 5].map((i) => parseInt(a.slice(i, i + 2), 16));
		const pb = [1, 3, 5].map((i) => parseInt(b.slice(i, i + 2), 16));
		const c = pa.map((v, i) => Math.round(v + (pb[i] - v) * t));
		return `#${c.map((v) => v.toString(16).padStart(2, '0')).join('')}`;
	}

	function rampColor(bucket: number, dark: boolean): string {
		const t = BUCKETS <= 1 ? 1 : bucket / (BUCKETS - 1);
		return dark ? lerpHex('#1f2b3a', '#5aa6ff', t) : lerpHex('#eaf1fb', '#0d366b', t);
	}

	function isDark(): boolean {
		return typeof document !== 'undefined' && document.documentElement.classList.contains('dark');
	}

	let dark = $state(false);
	$effect(() => {
		dark = isDark();
		if (typeof window === 'undefined') return;
		const observer = new MutationObserver(() => (dark = isDark()));
		observer.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] });
		return () => observer.disconnect();
	});

	const breaks = $derived.by(() => {
		const values = Object.values(data)
			.filter((v) => v > 0)
			.sort((a, b) => a - b);
		if (values.length === 0) return [] as number[];
		const cuts: number[] = [];
		for (let i = 1; i < BUCKETS; i++) {
			const idx = Math.min(Math.floor((values.length * i) / BUCKETS), values.length - 1);
			cuts.push(values[idx]);
		}
		return cuts;
	});

	function bucketFor(value: number): number {
		let bucket = 0;
		for (const cut of breaks) {
			if (value > cut) bucket++;
		}
		return Math.min(bucket, BUCKETS - 1);
	}

	const features = $derived.by(() =>
		world.features.map((f, i) => {
			const code = alpha2For(f.id);
			const value = code ? (data[code] ?? 0) : 0;
			const fill =
				value > 0 ? rampColor(bucketFor(value), dark) : dark ? NO_DATA_DARK : NO_DATA_LIGHT;
			return {
				key: f.id ?? i,
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				d: pathGenerator(f as any) ?? '',
				fill,
				tooltip: code
					? `${countryName(code)}: ${formatValue(value)}`
					: (f.properties?.name ?? 'Unknown')
			};
		})
	);

	const legendStops = $derived(Array.from({ length: BUCKETS }, (_, i) => rampColor(i, dark)));

	// Pan/zoom state: a transform on the <g> wrapping the paths, viewBox stays fixed.
	let svgEl = $state<SVGSVGElement | null>(null);
	let scale = $state(1);
	let tx = $state(0);
	let ty = $state(0);
	let panning = $state(false);
	let panStart = { x: 0, y: 0, tx: 0, ty: 0 };

	const MIN_SCALE = 1;
	const MAX_SCALE = 8;

	function clampPan(nextScale: number, nextTx: number, nextTy: number) {
		// At scale 1 the map exactly fills the viewport, so no pan is allowed at all —
		// the allowed offset must shrink to 0 as scale returns to 1.
		const maxOffsetX = (WIDTH * (nextScale - 1)) / 2;
		const maxOffsetY = (HEIGHT * (nextScale - 1)) / 2;
		return {
			tx: Math.min(maxOffsetX, Math.max(-maxOffsetX, nextTx)),
			ty: Math.min(maxOffsetY, Math.max(-maxOffsetY, nextTy))
		};
	}

	function svgPoint(clientX: number, clientY: number): { x: number; y: number } {
		if (!svgEl) return { x: 0, y: 0 };
		const rect = svgEl.getBoundingClientRect();
		return {
			x: ((clientX - rect.left) / rect.width) * WIDTH,
			y: ((clientY - rect.top) / rect.height) * HEIGHT
		};
	}

	function onWheel(e: WheelEvent) {
		e.preventDefault();
		const point = svgPoint(e.clientX, e.clientY);
		const factor = e.deltaY < 0 ? 1.25 : 0.8;
		const nextScale = Math.min(MAX_SCALE, Math.max(MIN_SCALE, scale * factor));
		if (nextScale === scale) return;
		// Keep the point under the cursor fixed while zooming.
		const worldX = (point.x - tx - WIDTH / 2) / scale;
		const worldY = (point.y - ty - HEIGHT / 2) / scale;
		const nextTx = point.x - WIDTH / 2 - worldX * nextScale;
		const nextTy = point.y - HEIGHT / 2 - worldY * nextScale;
		const clamped = clampPan(nextScale, nextTx, nextTy);
		scale = nextScale;
		tx = clamped.tx;
		ty = clamped.ty;
	}

	function onPointerDown(e: PointerEvent) {
		if (scale <= MIN_SCALE) return;
		panning = true;
		panStart = { x: e.clientX, y: e.clientY, tx, ty };
		(e.currentTarget as SVGSVGElement).setPointerCapture(e.pointerId);
	}

	function onPointerMove(e: PointerEvent) {
		if (!panning) return;
		const dxClient = e.clientX - panStart.x;
		const dyClient = e.clientY - panStart.y;
		const rect = svgEl?.getBoundingClientRect();
		const scaleFactor = rect ? WIDTH / rect.width : 1;
		const clamped = clampPan(
			scale,
			panStart.tx + dxClient * scaleFactor,
			panStart.ty + dyClient * scaleFactor
		);
		tx = clamped.tx;
		ty = clamped.ty;
	}

	function onPointerUp(e: PointerEvent) {
		panning = false;
		(e.currentTarget as SVGSVGElement).releasePointerCapture(e.pointerId);
	}

	function resetView() {
		scale = 1;
		tx = 0;
		ty = 0;
	}

	let hoveredLabel = $state<string | null>(null);
	let tooltip = $state({ x: 0, y: 0, visible: false });

	function onPathEnter(text: string, e: PointerEvent) {
		hoveredLabel = text;
		tooltip = { x: e.clientX, y: e.clientY, visible: true };
	}

	function onPathLeave() {
		hoveredLabel = null;
		tooltip = { ...tooltip, visible: false };
	}
</script>

<div class="relative flex flex-col gap-2">
	<svg
		bind:this={svgEl}
		viewBox="0 0 {WIDTH} {HEIGHT}"
		class="w-full touch-none select-none"
		class:cursor-grab={scale > MIN_SCALE && !panning}
		class:cursor-grabbing={panning}
		role="img"
		aria-label="World map"
		onwheel={onWheel}
		onpointerdown={onPointerDown}
		onpointermove={onPointerMove}
		onpointerup={onPointerUp}
		onpointerleave={onPathLeave}
	>
		<g
			transform="translate({tx}, {ty}) translate({WIDTH / 2}, {HEIGHT /
				2}) scale({scale}) translate({-WIDTH / 2}, {-HEIGHT / 2})"
		>
			{#each features as f (f.key)}
				<path
					d={f.d}
					fill={f.fill}
					stroke="var(--card)"
					stroke-width={0.5 / scale}
					role="presentation"
					onpointerenter={(e) => onPathEnter(f.tooltip, e)}
					onpointermove={(e) => onPathEnter(f.tooltip, e)}
					onpointerleave={onPathLeave}
				/>
			{/each}
		</g>
	</svg>

	{#if tooltip.visible && hoveredLabel}
		<div
			class="pointer-events-none fixed z-50 -translate-x-1/2 -translate-y-full rounded-md border border-border bg-popover px-2 py-1 text-xs whitespace-nowrap text-popover-foreground shadow-md"
			style="left: {tooltip.x}px; top: {tooltip.y - 8}px"
		>
			{hoveredLabel}
		</div>
	{/if}

	<div class="flex items-center justify-between gap-2">
		<div class="flex items-center gap-1 text-[10px] text-muted-foreground">
			<span>Less</span>
			{#each legendStops as color, i (i)}
				<span class="h-3 w-5" style="background-color: {color}"></span>
			{/each}
			<span>More</span>
		</div>
		<p class="text-center text-[10px] text-muted-foreground">Scroll to zoom, drag to pan</p>
		{#if scale > MIN_SCALE}
			<button
				type="button"
				class="cursor-pointer text-[10px] text-muted-foreground underline underline-offset-2 hover:text-foreground"
				onclick={resetView}
			>
				Reset zoom
			</button>
		{:else}
			<div class="h-3 w-16"></div>
		{/if}
	</div>
</div>
