<script lang="ts">
	import Chart from 'chart.js/auto';

	let {
		data,
		formatValue,
		vertical = false
	}: {
		data: { label: string; value: number }[];
		formatValue: (value: number) => string;
		vertical?: boolean;
	} = $props();

	let canvas = $state<HTMLCanvasElement | null>(null);
	let chart: Chart | null = null;

	const height = $derived(vertical ? 240 : Math.max(200, data.length * 28));

	function cssVar(name: string): string {
		if (typeof window === 'undefined') return '#888';
		return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
	}

	function render() {
		if (!canvas) return;
		chart?.destroy();

		const bar = cssVar('--primary');
		const grid = cssVar('--border');
		const ink = cssVar('--muted-foreground');

		chart = new Chart(canvas, {
			type: 'bar',
			data: {
				labels: data.map((r) => r.label),
				datasets: [
					{
						data: data.map((r) => r.value),
						backgroundColor: bar,
						borderRadius: vertical
							? { topLeft: 4, topRight: 4, bottomLeft: 0, bottomRight: 0 }
							: { topLeft: 0, bottomLeft: 0, topRight: 4, bottomRight: 4 },
						borderSkipped: false,
						barThickness: vertical ? undefined : 16,
						maxBarThickness: vertical ? 48 : 20
					}
				]
			},
			options: {
				indexAxis: vertical ? 'x' : 'y',
				responsive: true,
				maintainAspectRatio: false,
				plugins: {
					legend: { display: false },
					tooltip: {
						callbacks: {
							label: (ctx) => formatValue((vertical ? ctx.parsed.y : ctx.parsed.x) ?? 0)
						}
					},
					datalabels: { display: false }
				},
				scales: vertical
					? {
							x: {
								grid: { display: false },
								ticks: { color: ink, font: { size: 11 } }
							},
							y: {
								beginAtZero: true,
								grid: { color: grid },
								ticks: {
									color: ink,
									font: { size: 11 },
									callback: (value) => formatValue(Number(value))
								}
							}
						}
					: {
							x: {
								beginAtZero: true,
								grid: { color: grid },
								ticks: {
									color: ink,
									font: { size: 11 },
									callback: (value) => formatValue(Number(value))
								}
							},
							y: {
								grid: { display: false },
								ticks: { color: ink, font: { size: 11 } }
							}
						}
			}
		});
	}

	$effect(() => {
		void data;
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

{#if data.length === 0}
	<p class="py-12 text-center text-sm text-muted-foreground">No data.</p>
{:else}
	<div style="height: {height}px" class="w-full">
		<canvas bind:this={canvas}></canvas>
	</div>
{/if}
