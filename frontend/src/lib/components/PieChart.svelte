<script lang="ts">
	import Chart from 'chart.js/auto';
	import ChartDataLabels from 'chartjs-plugin-datalabels';

	Chart.register(ChartDataLabels);

	const CATEGORICAL: { light: string; dark: string }[] = [
		{ light: '#2a78d6', dark: '#3987e5' }, // blue
		{ light: '#eb6834', dark: '#d95926' }, // orange
		{ light: '#1baf7a', dark: '#199e70' }, // aqua
		{ light: '#eda100', dark: '#c98500' }, // yellow
		{ light: '#e87ba4', dark: '#d55181' }, // magenta
		{ light: '#008300', dark: '#008300' }, // green
		{ light: '#8c6bb1', dark: '#7a4fa0' }, // purple
		{ light: '#00b0b0', dark: '#00a0a0' }, // teal
		{ light: '#d9d9d9', dark: '#666666' } // gray
	];
	const OTHER = '#898781';

	let {
		data,
		formatValue
	}: {
		data: { label: string; value: number; isOther?: boolean }[];
		formatValue: (value: number) => string;
	} = $props();

	let canvas = $state<HTMLCanvasElement | null>(null);
	let chart: Chart | null = null;

	const total = $derived(data.reduce((sum, r) => sum + r.value, 0));

	function isDark(): boolean {
		return typeof document !== 'undefined' && document.documentElement.classList.contains('dark');
	}

	function cssVar(name: string): string {
		if (typeof window === 'undefined') return '#888';
		return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
	}

	function render() {
		if (!canvas) return;
		chart?.destroy();

		const dark = isDark();
		let colorIndex = 0;
		const colors = data.map((row) =>
			row.isOther
				? OTHER
				: (CATEGORICAL[colorIndex++] ?? CATEGORICAL.at(-1))![dark ? 'dark' : 'light']
		);
		const surface = cssVar('--card') || (dark ? '#1a1a19' : '#fcfcfb');
		const secondaryInk = cssVar('--muted-foreground');

		chart = new Chart(canvas, {
			type: 'pie',
			data: {
				labels: data.map((r) => r.label),
				datasets: [
					{
						data: data.map((r) => r.value),
						backgroundColor: colors,
						borderColor: surface,
						borderWidth: 2
					}
				]
			},
			options: {
				responsive: true,
				maintainAspectRatio: false,
				plugins: {
					legend: {
						position: 'right',
						labels: { color: secondaryInk, boxWidth: 12, font: { size: 11 } }
					},
					tooltip: {
						callbacks: {
							label: (ctx) => `${ctx.label}: ${formatValue(ctx.parsed)}`
						}
					},
					datalabels: {
						color: (ctx) => {
							const value = (ctx.dataset.data[ctx.dataIndex] as number) ?? 0;
							return value / total < 0.06 ? 'transparent' : '#fff';
						},
						font: { size: 11, weight: 'bold' },
						formatter: (value: number) => `${Math.round((value / total) * 100)}%`
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
	<div class="h-64 w-full">
		<canvas bind:this={canvas}></canvas>
	</div>
{/if}
