<script lang="ts" generics="T">
	import Chart from 'chart.js/auto';
	import { SvelteMap } from 'svelte/reactivity';
	import { browser } from '$app/environment';

	// chartjs-plugin-zoom pulls in hammerjs, which touches `window` at import
	// time, so it can only be imported client-side (never during SSR).
	let zoomReady = $state(false);

	type Granularity = 'day' | 'week' | 'month' | 'quarter' | 'year';

	let {
		data,
		ysws,
		granularity,
		getDate,
		getYsws,
		getValue,
		formatValue,
		label,
		unmatchedLabel = 'Unmatched',
		cumulative = false
	}: {
		data: T[];
		ysws: string | null;
		granularity: Granularity;
		getDate: (row: T) => string;
		getYsws: (row: T) => string | null;
		getValue: (row: T) => number;
		formatValue: (value: number) => string;
		label: string;
		unmatchedLabel?: string;
		/** Show a running total instead of a per-bucket amount. */
		cumulative?: boolean;
	} = $props();

	let canvas = $state<HTMLCanvasElement | null>(null);
	let chart: Chart | null = null;
	let zoomed = $state(false);

	function resetZoom() {
		chart?.resetZoom();
		zoomed = false;
	}

	function bucketOf(dateStr: string): { key: string; label: string } {
		const d = new Date(`${dateStr}T00:00:00Z`);
		if (granularity === 'day') {
			return {
				key: dateStr,
				label: d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })
			};
		}
		if (granularity === 'week') {
			const dow = d.getUTCDay();
			const mondayOffset = dow === 0 ? -6 : 1 - dow;
			const monday = new Date(d.getTime() + mondayOffset * 86_400_000);
			return {
				key: monday.toISOString().slice(0, 10),
				label: `Week of ${monday.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })}`
			};
		}
		if (granularity === 'month') {
			return {
				key: dateStr.slice(0, 7),
				label: d.toLocaleDateString(undefined, { month: 'short', year: 'numeric' })
			};
		}
		if (granularity === 'quarter') {
			const year = d.getUTCFullYear();
			const quarter = Math.floor(d.getUTCMonth() / 3) + 1;
			return { key: `${year}-Q${quarter}`, label: `Q${quarter} ${year}` };
		}
		const year = d.getUTCFullYear();
		return { key: `${year}`, label: `${year}` };
	}

	const rows = $derived.by(() => {
		const filtered =
			ysws === null ? data : data.filter((row) => (getYsws(row) ?? unmatchedLabel) === ysws);

		const byBucket = new SvelteMap<string, { value: number; label: string }>();
		for (const row of filtered) {
			const { key, label: bucketLabel } = bucketOf(getDate(row));
			const existing = byBucket.get(key);
			byBucket.set(key, {
				value: (existing?.value ?? 0) + getValue(row),
				label: bucketLabel
			});
		}

		const sorted = [...byBucket.entries()].sort(([a], [b]) => a.localeCompare(b));

		if (!cumulative) {
			return sorted.map(([key, { value, label: bucketLabel }]) => ({
				key,
				value,
				chartLabel: bucketLabel
			}));
		}

		let running = 0;
		return sorted.map(([key, { value, label: bucketLabel }]) => {
			running += value;
			return { key, value: running, chartLabel: bucketLabel };
		});
	});

	const total = $derived(
		cumulative ? (rows.at(-1)?.value ?? 0) : rows.reduce((sum, r) => sum + r.value, 0)
	);

	function cssVar(name: string): string {
		if (typeof window === 'undefined') return '#888';
		return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
	}

	function render() {
		if (!canvas) return;
		chart?.destroy();
		zoomed = false;

		const bar = cssVar('--primary');
		const grid = cssVar('--border');
		const ink = cssVar('--muted-foreground');

		chart = new Chart(canvas, {
			type: 'bar',
			data: {
				labels: rows.map((r) => r.chartLabel),
				datasets: [
					{
						label,
						data: rows.map((r) => r.value),
						backgroundColor: bar,
						borderRadius: { topLeft: 4, topRight: 4, bottomLeft: 0, bottomRight: 0 },
						borderSkipped: false,
						maxBarThickness: 40
					}
				]
			},
			options: {
				responsive: true,
				maintainAspectRatio: false,
				animation: { duration: 150 },
				interaction: { mode: 'index', intersect: false },
				plugins: {
					legend: { display: false },
					tooltip: {
						callbacks: {
							label: (ctx) => formatValue(ctx.parsed.y ?? 0)
						}
					},
					datalabels: { display: false },
					zoom: {
						limits: { x: { min: 'original', max: 'original', minRange: 5 } },
						pan: { enabled: true, mode: 'x', onPanComplete: () => (zoomed = true) },
						zoom: {
							wheel: { enabled: true },
							pinch: { enabled: true },
							mode: 'x',
							onZoomComplete: (ctx) => (zoomed = ctx.chart.isZoomedOrPanned())
						}
					}
				},
				scales: {
					x: {
						grid: { display: false },
						ticks: { color: ink, font: { size: 11 }, maxRotation: 0, autoSkip: true }
					},
					y: {
						grid: { color: grid },
						ticks: {
							color: ink,
							font: { size: 11 },
							callback: (value) => formatValue(Number(value))
						},
						beginAtZero: true
					}
				}
			}
		});
	}

	$effect(() => {
		if (!browser || zoomReady) return;
		let cancelled = false;
		import('chartjs-plugin-zoom').then((mod) => {
			if (cancelled) return;
			Chart.register(mod.default);
			zoomReady = true;
		});
		return () => {
			cancelled = true;
		};
	});

	$effect(() => {
		void rows;
		void zoomReady;
		render();
		return () => chart?.destroy();
	});

	$effect(() => {
		if (typeof window === 'undefined') return;
		const observer = new MutationObserver(render);
		observer.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] });
		return () => observer.disconnect();
	});
</script>

<div class="flex flex-col gap-3">
	<p class="text-xs text-muted-foreground">
		{ysws ?? 'All programs'} · {formatValue(total)} total
	</p>

	{#if rows.length === 0}
		<p class="py-12 text-center text-sm text-muted-foreground">No data.</p>
	{:else}
		<div class="h-56 w-full">
			<canvas bind:this={canvas}></canvas>
		</div>

		<div class="flex items-center justify-between gap-2 text-[10px] text-muted-foreground">
			<span>Scroll to zoom, drag to pan</span>
			{#if zoomed}
				<button
					type="button"
					class="cursor-pointer underline underline-offset-2 hover:text-foreground"
					onclick={resetZoom}
				>
					Reset zoom
				</button>
			{/if}
		</div>

		<details class="text-xs text-muted-foreground">
			<summary class="cursor-pointer select-none hover:text-foreground">View as table</summary>
			<div class="mt-2 max-h-48 overflow-y-auto rounded-md border border-border">
				<table class="w-full text-left">
					<thead class="sticky top-0 bg-muted/60">
						<tr>
							<th class="px-3 py-1.5 font-medium">Period</th>
							<th class="px-3 py-1.5 font-medium">{label}</th>
						</tr>
					</thead>
					<tbody>
						{#each rows as row (row.key)}
							<tr class="border-t border-border">
								<td class="px-3 py-1.5">{row.chartLabel}</td>
								<td class="px-3 py-1.5">{formatValue(row.value)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</details>
	{/if}
</div>
