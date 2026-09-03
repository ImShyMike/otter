<script lang="ts">
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import ArrowDown from '@lucide/svelte/icons/arrow-down';
	import ArrowUp from '@lucide/svelte/icons/arrow-up';
	import ArrowUpDown from '@lucide/svelte/icons/arrow-up-down';
	import Banknote from '@lucide/svelte/icons/banknote';
	import Clock3 from '@lucide/svelte/icons/clock-3';
	import FolderGit2 from '@lucide/svelte/icons/folder-git-2';
	import Globe from '@lucide/svelte/icons/globe';
	import Layers from '@lucide/svelte/icons/layers';
	import Users from '@lucide/svelte/icons/users';
	import BarChart from '$lib/components/BarChart.svelte';
	import Head from '$lib/components/Head.svelte';
	import PieChart from '$lib/components/PieChart.svelte';
	import ServerStatus from '$lib/components/ServerStatus.svelte';
	import TimeSeriesBarChart from '$lib/components/TimeSeriesBarChart.svelte';
	import WorldMap from '$lib/components/WorldMap.svelte';
	import { Badge } from '$lib/components/ui/badge';
	import { Input } from '$lib/components/ui/input';
	import * as Table from '$lib/components/ui/table';
	import { goBack } from '$lib/back';
	import { SvelteMap } from 'svelte/reactivity';
	import { API_BASE } from '$lib/search';
	import type { CountryStats, ProjectPeriodStats, YswsCountryStats, YswsStats } from '$lib/types';
	import { countryName, formatFloat } from '$lib/utils';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
	const stats = $derived(data.stats);

	const ALL = '__all__';

	type Metric = 'total_projects' | 'total_hours' | 'total_wp' | 'unique_shippers';
	type MetricRow = {
		total_projects: bigint;
		total_hours: number;
		total_wp: number;
		unique_shippers: bigint;
	};

	const METRICS: { key: Metric; label: string }[] = [
		{ key: 'total_projects', label: 'Projects' },
		{ key: 'total_wp', label: 'WPs' },
		{ key: 'unique_shippers', label: 'Shippers' }
	];

	function metricValue(row: MetricRow, metric: Metric): number {
		const raw = row[metric];
		return typeof raw === 'bigint' ? Number(raw) : raw;
	}

	function metricFormatter(metric: Metric): (value: number) => string {
		if (metric === 'total_hours' || metric === 'total_wp') return (n) => formatFloat(n, 1);
		return (n) => Math.round(n).toLocaleString();
	}

	function topNWithOther(
		rows: { label: string; value: number }[],
		n: number
	): { label: string; value: number; isOther?: boolean }[] {
		if (rows.length <= n) return rows;
		const top = rows.slice(0, n);
		const otherValue = rows.slice(n).reduce((sum, r) => sum + r.value, 0);
		return [...top, { label: 'Other', value: otherValue, isOther: true }];
	}

	function currency(value: number): string {
		return value.toLocaleString(undefined, { style: 'currency', currency: 'USD' });
	}

	type SortKey = 'name' | 'total_projects' | 'total_wp' | 'unique_shippers';
	type SortState = { key: SortKey; dir: 1 | -1 };

	function toggleSort(state: SortState, key: SortKey): SortState {
		if (state.key === key) return { key, dir: state.dir === 1 ? -1 : 1 };
		return { key, dir: key === 'name' ? 1 : -1 };
	}

	function sortRows<T extends MetricRow>(
		rows: T[],
		state: SortState,
		nameOf: (row: T) => string
	): T[] {
		const sorted = [...rows];
		sorted.sort((a, b) => {
			if (state.key === 'name') return nameOf(a).localeCompare(nameOf(b)) * state.dir;
			return (metricValue(a, state.key) - metricValue(b, state.key)) * state.dir;
		});
		return sorted;
	}

	type Granularity = 'day' | 'week' | 'month' | 'quarter' | 'year';
	const GRANULARITIES: { key: Granularity; label: string }[] = [
		{ key: 'day', label: 'Day' },
		{ key: 'week', label: 'Week' },
		{ key: 'month', label: 'Month' },
		{ key: 'quarter', label: 'Quarter' },
		{ key: 'year', label: 'Year' }
	];

	let yswsSort = $state<SortState>({ key: 'total_projects', dir: -1 });
	let countrySort = $state<SortState>({ key: 'total_projects', dir: -1 });
	let countryYsws = $state<string>(ALL);
	let overTimeYsws = $state<string>(ALL);
	let overTimeGranularity = $state<Granularity>('month');
	let yswsFilter = $state('');
	let countryFilter = $state('');
	let yswsMetric = $state<Metric>('total_projects');
	let countryMetric = $state<Metric>('total_projects');

	const sortedYsws = $derived(
		stats
			? [...stats.by_ysws].sort((a, b) => metricValue(b, yswsMetric) - metricValue(a, yswsMetric))
			: []
	);
	const topYswsBar = $derived(
		sortedYsws.slice(0, 15).map((r) => ({ label: r.ysws, value: metricValue(r, yswsMetric) }))
	);
	const yswsPieData = $derived(
		topNWithOther(
			sortedYsws.map((r) => ({ label: r.ysws, value: metricValue(r, yswsMetric) })),
			9
		)
	);

	const byYsws = $derived.by(() => {
		if (!stats) return [];
		const needle = yswsFilter.trim().toLowerCase();
		const filtered = needle
			? stats.by_ysws.filter((r) => r.ysws.toLowerCase().includes(needle))
			: stats.by_ysws;
		return sortRows<YswsStats>(filtered, yswsSort, (r) => r.ysws);
	});

	const sortedCountries = $derived(
		stats
			? [...stats.by_country].sort(
					(a, b) => metricValue(b, countryMetric) - metricValue(a, countryMetric)
				)
			: []
	);
	const topCountryBar = $derived(
		sortedCountries
			.slice(0, 15)
			.map((r) => ({ label: countryName(r.country_code), value: metricValue(r, countryMetric) }))
	);
	const countryPieData = $derived(
		topNWithOther(
			sortedCountries.map((r) => ({
				label: countryName(r.country_code),
				value: metricValue(r, countryMetric)
			})),
			5
		)
	);
	const worldMapData = $derived.by(() => {
		const map: Record<string, number> = {};
		for (const row of sortedCountries) map[row.country_code] = metricValue(row, countryMetric);
		return map;
	});

	const countryBreakdownCache = new SvelteMap<string, YswsCountryStats[]>();
	let countryBreakdownLoading = $state(false);

	$effect(() => {
		if (countryYsws === ALL || countryBreakdownCache.has(countryYsws)) return;
		const ysws = countryYsws;
		countryBreakdownLoading = true;
		fetch(`${API_BASE}/api/v1/stats/country-breakdown?ysws=${encodeURIComponent(ysws)}`)
			.then((res) => (res.ok ? res.json() : []))
			.then((rows: YswsCountryStats[]) => countryBreakdownCache.set(ysws, rows))
			.catch(() => countryBreakdownCache.set(ysws, []))
			.finally(() => (countryBreakdownLoading = false));
	});

	const countryPool = $derived.by((): (CountryStats | YswsCountryStats)[] => {
		if (!stats) return [];
		if (countryYsws === ALL) return stats.by_country;
		return countryBreakdownCache.get(countryYsws) ?? [];
	});
	const byCountry = $derived.by(() => {
		const needle = countryFilter.trim().toLowerCase();
		const filtered = needle
			? countryPool.filter((r) => countryName(r.country_code).toLowerCase().includes(needle))
			: countryPool;
		return sortRows(filtered, countrySort, (r) => countryName(r.country_code));
	});

	const programOptions = $derived.by(() => {
		if (!stats) return [];
		const hasUnmatchedFines = stats.fines_by_day.some((r) => r.ysws === null);
		return [...stats.by_ysws.map((r) => r.ysws), ...(hasUnmatchedFines ? ['Unmatched'] : [])];
	});

	const netFinesCents = $derived(
		stats ? stats.fines_by_day.reduce((sum, r) => sum + Number(r.amount_cents), 0) : 0
	);
	const avgHoursPerShipper = $derived(
		stats && Number(stats.overview.unique_shippers) > 0
			? stats.overview.total_hours / Number(stats.overview.unique_shippers)
			: 0
	);

	const finesByYsws = $derived.by(() => {
		if (!stats) return new SvelteMap<string, number>();
		const map = new SvelteMap<string, number>();
		for (const row of stats.fines_by_day) {
			if (row.ysws === null) continue;
			map.set(row.ysws, (map.get(row.ysws) ?? 0) + Number(row.amount_cents));
		}
		return map;
	});

	function netFinesFor(ysws: string): number {
		return (finesByYsws.get(ysws) ?? 0) / 100;
	}

	function finesPerWpFor(ysws: string, wp: number): number {
		return wp > 0 ? netFinesFor(ysws) / wp : 0;
	}

	function avgHoursPerProject(hours: number, projects: number | bigint): number {
		const n = Number(projects);
		return n > 0 ? hours / n : 0;
	}

	function avgHoursPerShipperFor(hours: number, shippers: number | bigint): number {
		const n = Number(shippers);
		return n > 0 ? hours / n : 0;
	}

	const netFinesByYswsBar = $derived(
		[...finesByYsws.entries()]
			.map(([ysws, cents]) => ({ label: ysws, value: cents / 100 }))
			.sort((a, b) => b.value - a.value)
			.slice(0, 15)
	);
	const netFinesPerWpByYswsBar = $derived.by(() => {
		if (!stats) return [];
		const wpByYsws = new SvelteMap(stats.by_ysws.map((r) => [r.ysws, r.total_wp]));
		return [...finesByYsws.entries()]
			.map(([ysws, cents]) => {
				const wp = wpByYsws.get(ysws) ?? 0;
				return { label: ysws, value: wp > 0 ? cents / 100 / wp : 0 };
			})
			.filter((r) => r.value > 0)
			.sort((a, b) => b.value - a.value)
			.slice(0, 15);
	});

	type TimeSeriesRow = {
		date: string;
		ysws: string;
		total_projects: bigint | number;
		total_hours: number;
	};

	const projectsByGranularity = new SvelteMap<Granularity, ProjectPeriodStats[]>();
	let projectsByGranularityLoading = $state(false);

	$effect(() => {
		const granularity = overTimeGranularity;
		if (granularity === 'month' || projectsByGranularity.has(granularity)) return;
		projectsByGranularityLoading = true;
		fetch(`${API_BASE}/api/v1/stats/projects-by-time?granularity=${granularity}`)
			.then((res) => (res.ok ? res.json() : []))
			.then((rows: ProjectPeriodStats[]) => projectsByGranularity.set(granularity, rows))
			.catch(() => projectsByGranularity.set(granularity, []))
			.finally(() => (projectsByGranularityLoading = false));
	});

	const timeSeriesLoading = $derived(
		overTimeGranularity !== 'month' &&
			!projectsByGranularity.has(overTimeGranularity) &&
			projectsByGranularityLoading
	);
	const timeSeriesData = $derived.by((): TimeSeriesRow[] => {
		if (!stats) return [];
		const rows =
			overTimeGranularity === 'month'
				? stats.projects_by_month
				: (projectsByGranularity.get(overTimeGranularity) ?? []);
		return rows.map((r) => ({ ...r, date: r.period }));
	});

	const hoursDistributionBar = $derived(
		stats
			? stats.hours_per_shipper_distribution.map((r) => ({
					label: r.bucket,
					value: Number(r.count)
				}))
			: []
	);
	const submissionsDistributionBar = $derived(
		stats
			? stats.submissions_per_shipper_distribution.map((r) => ({
					label: r.bucket,
					value: Number(r.count)
				}))
			: []
	);
</script>

{#snippet sortIndicator(state: SortState, key: SortKey)}
	{#if state.key !== key}
		<ArrowUpDown class="h-3 w-3" />
	{:else if state.dir === 1}
		<ArrowUp class="h-3 w-3" />
	{:else}
		<ArrowDown class="h-3 w-3" />
	{/if}
{/snippet}

{#snippet metricToggle(current: Metric, onSelect: (metric: Metric) => void)}
	<div class="flex flex-wrap gap-1">
		{#each METRICS as m (m.key)}
			<button type="button" class="cursor-pointer" onclick={() => onSelect(m.key)}>
				<Badge variant={current === m.key ? 'default' : 'secondary'} class="cursor-pointer text-xs">
					{m.label}
				</Badge>
			</button>
		{/each}
	</div>
{/snippet}

<Head
	title="Stats · Otter"
	description="Site-wide statistics for Hack Club YSWS programs: shippers, projects, hours, countries, and fines."
/>

<div class="mx-auto flex min-h-screen max-w-5xl flex-col px-4 py-6.5 sm:py-8">
	<div class="mb-4 flex items-center gap-4">
		<button
			onclick={goBack}
			class="flex cursor-pointer items-center gap-1 text-sm text-muted-foreground underline underline-offset-2 hover:text-foreground"
			data-umami-event="stats-back"
		>
			<ArrowLeft class="h-3 w-3" /> Back
		</button>
	</div>

	<h1 class="mb-6 text-2xl font-bold tracking-tight">Stats</h1>

	{#if !stats}
		<p class="py-12 text-center text-sm text-muted-foreground">Could not load statistics.</p>
	{:else}
		{@const overview = stats.overview}
		<div class="mb-10 grid grid-cols-2 gap-3 sm:grid-cols-4">
			<div class="rounded-lg border border-border bg-muted/40 px-4 py-4">
				<p class="flex items-center gap-1.5 text-xs text-muted-foreground">
					<FolderGit2 class="h-3 w-3 shrink-0" /> Projects
				</p>
				<p class="mt-1 text-xl font-semibold tracking-tight">{Number(overview.total_projects)}</p>
			</div>
			<div class="rounded-lg border border-border bg-muted/40 px-4 py-4">
				<p class="flex items-center gap-1.5 text-xs text-muted-foreground">
					<Layers class="h-3 w-3 shrink-0" /> Weighted projects
				</p>
				<p class="mt-1 text-xl font-semibold tracking-tight">{formatFloat(overview.total_wp, 1)}</p>
			</div>
			<div class="rounded-lg border border-border bg-muted/40 px-4 py-4">
				<p class="flex items-center gap-1.5 text-xs text-muted-foreground">
					<Users class="h-3 w-3 shrink-0" /> Shippers
				</p>
				<p class="mt-1 text-xl font-semibold tracking-tight">{Number(overview.unique_shippers)}</p>
			</div>
			<div class="rounded-lg border border-border bg-muted/40 px-4 py-4">
				<p class="flex items-center gap-1.5 text-xs text-muted-foreground">
					<Banknote class="h-3 w-3 shrink-0" /> Net fines
				</p>
				<p class="mt-1 text-xl font-semibold tracking-tight">{currency(netFinesCents / 100)}</p>
			</div>
			<div class="rounded-lg border border-border bg-muted/40 px-4 py-4">
				<p class="flex items-center gap-1.5 text-xs text-muted-foreground">
					<Layers class="h-3 w-3 shrink-0" /> YSWS programs
				</p>
				<p class="mt-1 text-xl font-semibold tracking-tight">{Number(overview.total_ysws)}</p>
			</div>
			<div class="rounded-lg border border-border bg-muted/40 px-4 py-4">
				<p class="flex items-center gap-1.5 text-xs text-muted-foreground">
					<Globe class="h-3 w-3 shrink-0" /> Countries
				</p>
				<p class="mt-1 text-xl font-semibold tracking-tight">{Number(overview.total_countries)}</p>
			</div>
			<div class="rounded-lg border border-border bg-muted/40 px-4 py-4">
				<p class="flex items-center gap-1.5 text-xs text-muted-foreground">
					<Clock3 class="h-3 w-3 shrink-0" /> Hours / shipper
				</p>
				<p class="mt-1 text-xl font-semibold tracking-tight">
					{formatFloat(avgHoursPerShipper, 1)}
				</p>
			</div>
			<div class="rounded-lg border border-border bg-muted/40 px-4 py-4">
				<p class="flex items-center gap-1.5 text-xs text-muted-foreground">
					<Clock3 class="h-3 w-3 shrink-0" /> Hours / project
				</p>
				<p class="mt-1 text-xl font-semibold tracking-tight">
					{formatFloat(avgHoursPerProject(overview.total_hours, overview.total_projects), 1)}
				</p>
			</div>
		</div>

		<section class="mb-10">
			<div class="mb-3 flex flex-wrap items-center justify-between gap-2">
				<h2 class="text-lg font-semibold tracking-tight">By YSWS</h2>
				{@render metricToggle(yswsMetric, (m) => (yswsMetric = m))}
			</div>

			<div class="mb-6 grid grid-cols-1 gap-4 lg:grid-cols-[2fr_1fr]">
				<div class="rounded-md border border-border p-3">
					<BarChart data={topYswsBar} formatValue={metricFormatter(yswsMetric)} />
				</div>
				<div class="rounded-md border border-border p-3">
					<PieChart data={yswsPieData} formatValue={metricFormatter(yswsMetric)} />
				</div>
			</div>

			<div class="mb-6 grid grid-cols-1 gap-4 lg:grid-cols-2">
				<div class="rounded-md border border-border p-3">
					<h3 class="mb-2 flex items-center gap-1.5 text-sm font-medium text-muted-foreground">
						<Banknote class="h-3.5 w-3.5" /> Net fines
					</h3>
					<BarChart data={netFinesByYswsBar} formatValue={currency} />
				</div>
				<div class="rounded-md border border-border p-3">
					<h3 class="mb-2 flex items-center gap-1.5 text-sm font-medium text-muted-foreground">
						<Banknote class="h-3.5 w-3.5" /> Net fines / weighted project
					</h3>
					<BarChart data={netFinesPerWpByYswsBar} formatValue={currency} />
				</div>
			</div>

			<div class="mb-3 flex flex-wrap items-center justify-between gap-2">
				<h3 class="text-sm font-medium text-muted-foreground">All programs</h3>
				<Input
					bind:value={yswsFilter}
					name="ysws-filter"
					placeholder="Filter YSWS…"
					class="h-8 w-48"
					aria-label="Filter YSWS"
				/>
			</div>
			<div class="max-h-112 overflow-y-auto rounded-md border border-border">
				<Table.Root>
					<Table.Header class="sticky top-0 z-10">
						<Table.Row>
							<Table.Head>
								<button
									class="flex cursor-pointer items-center gap-1 hover:text-foreground"
									onclick={() => (yswsSort = toggleSort(yswsSort, 'name'))}
								>
									YSWS
									{@render sortIndicator(yswsSort, 'name')}
								</button>
							</Table.Head>
							<Table.Head>
								<button
									class="flex cursor-pointer items-center gap-1 hover:text-foreground"
									onclick={() => (yswsSort = toggleSort(yswsSort, 'total_projects'))}
								>
									Projects
									{@render sortIndicator(yswsSort, 'total_projects')}
								</button>
							</Table.Head>
							<Table.Head>
								<button
									class="flex cursor-pointer items-center gap-1 hover:text-foreground"
									onclick={() => (yswsSort = toggleSort(yswsSort, 'unique_shippers'))}
								>
									Shippers
									{@render sortIndicator(yswsSort, 'unique_shippers')}
								</button>
							</Table.Head>
							<Table.Head>
								<button
									class="flex cursor-pointer items-center gap-1 hover:text-foreground"
									onclick={() => (yswsSort = toggleSort(yswsSort, 'total_wp'))}
								>
									WPs
									{@render sortIndicator(yswsSort, 'total_wp')}
								</button>
							</Table.Head>
							<Table.Head>Net fines</Table.Head>
							<Table.Head>Fines / WP</Table.Head>
							<Table.Head>Hours / project</Table.Head>
							<Table.Head>Hours / shipper</Table.Head>
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each byYsws as row (row.ysws)}
							<Table.Row>
								<Table.Cell class="font-medium">{row.ysws}</Table.Cell>
								<Table.Cell>{Number(row.total_projects)}</Table.Cell>
								<Table.Cell>{Number(row.unique_shippers)}</Table.Cell>
								<Table.Cell>{formatFloat(row.total_wp, 1)}</Table.Cell>
								<Table.Cell>{currency(netFinesFor(row.ysws))}</Table.Cell>
								<Table.Cell>{currency(finesPerWpFor(row.ysws, row.total_wp))}</Table.Cell>
								<Table.Cell
									>{formatFloat(
										avgHoursPerProject(row.total_hours, row.total_projects),
										1
									)}</Table.Cell
								>
								<Table.Cell
									>{formatFloat(
										avgHoursPerShipperFor(row.total_hours, row.unique_shippers),
										1
									)}</Table.Cell
								>
							</Table.Row>
						{:else}
							<Table.Row>
								<Table.Cell colspan={8} class="text-center text-muted-foreground"
									>No matching YSWS programs.</Table.Cell
								>
							</Table.Row>
						{/each}
					</Table.Body>
				</Table.Root>
			</div>
		</section>

		<section class="mb-10">
			<div class="mb-3 flex flex-wrap items-center justify-between gap-2">
				<h2 class="text-lg font-semibold tracking-tight">By country</h2>
				{@render metricToggle(countryMetric, (m) => (countryMetric = m))}
			</div>

			<div class="mb-4 rounded-md border border-border p-3">
				<WorldMap data={worldMapData} formatValue={metricFormatter(countryMetric)} />
			</div>

			<div class="mb-6 grid grid-cols-1 gap-4 lg:grid-cols-[2fr_1fr]">
				<div class="rounded-md border border-border p-3">
					<BarChart data={topCountryBar} formatValue={metricFormatter(countryMetric)} />
				</div>
				<div class="rounded-md border border-border p-3">
					<PieChart data={countryPieData} formatValue={metricFormatter(countryMetric)} />
				</div>
			</div>

			<div class="mb-3 flex flex-wrap items-center justify-between gap-2">
				<h3 class="text-sm font-medium text-muted-foreground">All countries</h3>
				<div class="flex items-center gap-2">
					<Input
						bind:value={countryFilter}
						name="country-filter"
						placeholder="Filter country…"
						class="h-8 w-40"
						aria-label="Filter country"
					/>
					<select
						bind:value={countryYsws}
						name="country-program-filter"
						aria-label="Filter by YSWS program"
						class="h-8 cursor-pointer rounded-lg border border-input bg-popover px-2.5 py-1 text-sm text-popover-foreground"
					>
						<option value={ALL}>All programs</option>
						{#each stats.by_ysws as program (program.ysws)}
							<option value={program.ysws}>{program.ysws}</option>
						{/each}
					</select>
				</div>
			</div>
			{#if countryBreakdownLoading}
				<p class="mb-2 text-xs text-muted-foreground">Loading country breakdown…</p>
			{/if}
			<div class="max-h-112 overflow-y-auto rounded-md border border-border">
				<Table.Root>
					<Table.Header class="sticky top-0 z-10">
						<Table.Row>
							<Table.Head>
								<button
									class="flex cursor-pointer items-center gap-1 hover:text-foreground"
									onclick={() => (countrySort = toggleSort(countrySort, 'name'))}
								>
									Country
									{@render sortIndicator(countrySort, 'name')}
								</button>
							</Table.Head>
							<Table.Head>
								<button
									class="flex cursor-pointer items-center gap-1 hover:text-foreground"
									onclick={() => (countrySort = toggleSort(countrySort, 'total_projects'))}
								>
									Projects
									{@render sortIndicator(countrySort, 'total_projects')}
								</button>
							</Table.Head>
							<Table.Head>
								<button
									class="flex cursor-pointer items-center gap-1 hover:text-foreground"
									onclick={() => (countrySort = toggleSort(countrySort, 'total_wp'))}
								>
									WPs
									{@render sortIndicator(countrySort, 'total_wp')}
								</button>
							</Table.Head>
							<Table.Head>
								<button
									class="flex cursor-pointer items-center gap-1 hover:text-foreground"
									onclick={() => (countrySort = toggleSort(countrySort, 'unique_shippers'))}
								>
									Shippers
									{@render sortIndicator(countrySort, 'unique_shippers')}
								</button>
							</Table.Head>
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each byCountry as row (`${row.country_code}-${'ysws' in row ? row.ysws : ''}`)}
							<Table.Row>
								<Table.Cell class="font-medium">
									{countryName(row.country_code)}
									<span class="text-muted-foreground">({row.country_code})</span>
								</Table.Cell>
								<Table.Cell>{Number(row.total_projects)}</Table.Cell>
								<Table.Cell>{formatFloat(row.total_wp, 1)}</Table.Cell>
								<Table.Cell>{Number(row.unique_shippers)}</Table.Cell>
							</Table.Row>
						{:else}
							<Table.Row>
								<Table.Cell colspan={5} class="text-center text-muted-foreground"
									>No matching countries.</Table.Cell
								>
							</Table.Row>
						{/each}
					</Table.Body>
				</Table.Root>
			</div>
		</section>

		<section class="mb-10">
			<div class="mb-3 flex flex-wrap items-center justify-between gap-2">
				<h2 class="text-lg font-semibold tracking-tight">Over time</h2>
				<div class="flex flex-wrap items-center gap-2">
					<div class="flex flex-wrap gap-1">
						{#each GRANULARITIES as g (g.key)}
							<button
								type="button"
								class="cursor-pointer"
								onclick={() => (overTimeGranularity = g.key)}
							>
								<Badge
									variant={overTimeGranularity === g.key ? 'default' : 'secondary'}
									class="cursor-pointer text-xs"
								>
									{g.label}
								</Badge>
							</button>
						{/each}
					</div>
					<select
						bind:value={overTimeYsws}
						name="over-time-program-filter"
						aria-label="Filter over-time charts by YSWS program"
						class="h-8 cursor-pointer rounded-lg border border-input bg-popover px-2.5 py-1 text-sm text-popover-foreground"
					>
						<option value={ALL}>All programs</option>
						{#each programOptions as program (program)}
							<option value={program}>{program}</option>
						{/each}
					</select>
				</div>
			</div>

			<div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
				<div class="rounded-md border border-border p-3">
					<h3 class="mb-2 flex items-center gap-1.5 text-sm font-medium text-muted-foreground">
						<FolderGit2 class="h-3.5 w-3.5" /> Approvals over time
					</h3>
					{#if timeSeriesLoading}
						<p class="py-12 text-center text-sm text-muted-foreground">Loading…</p>
					{:else}
						<TimeSeriesBarChart
							data={timeSeriesData}
							ysws={overTimeYsws === ALL ? null : overTimeYsws}
							granularity={overTimeGranularity}
							getDate={(r) => r.date}
							getYsws={(r) => r.ysws}
							getValue={(r) => Number(r.total_projects)}
							formatValue={(n) => Math.round(n).toLocaleString()}
							label="Projects"
						/>
					{/if}
				</div>
				<div class="rounded-md border border-border p-3">
					<h3 class="mb-2 flex items-center gap-1.5 text-sm font-medium text-muted-foreground">
						<Layers class="h-3.5 w-3.5" /> WPs logged
					</h3>
					{#if timeSeriesLoading}
						<p class="py-12 text-center text-sm text-muted-foreground">Loading…</p>
					{:else}
						<TimeSeriesBarChart
							data={timeSeriesData}
							ysws={overTimeYsws === ALL ? null : overTimeYsws}
							granularity={overTimeGranularity}
							getDate={(r) => r.date}
							getYsws={(r) => r.ysws}
							getValue={(r) => r.total_hours / 10}
							formatValue={(n) => formatFloat(n, 1)}
							label="WPs"
						/>
					{/if}
				</div>
				<div class="rounded-md border border-border p-3 lg:col-span-2">
					<h3 class="mb-2 flex items-center gap-1.5 text-sm font-medium text-muted-foreground">
						<Banknote class="h-3.5 w-3.5" /> Net fines over time
					</h3>
					<TimeSeriesBarChart
						data={stats.fines_by_day}
						ysws={overTimeYsws === ALL ? null : overTimeYsws}
						granularity={overTimeGranularity}
						getDate={(r) => r.date}
						getYsws={(r) => r.ysws}
						getValue={(r) => Number(r.amount_cents) / 100}
						formatValue={currency}
						label="Net fines"
						cumulative
					/>
				</div>
			</div>
		</section>

		<section class="mb-10">
			<h2 class="mb-3 text-lg font-semibold tracking-tight">Shippers</h2>

			<div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
				<div class="rounded-md border border-border p-3">
					<h3 class="mb-2 flex items-center gap-1.5 text-sm font-medium text-muted-foreground">
						<Clock3 class="h-3.5 w-3.5" /> Total hours per shipper
					</h3>
					<BarChart
						data={hoursDistributionBar}
						formatValue={(n) => Math.round(n).toLocaleString()}
						vertical
					/>
				</div>
				<div class="rounded-md border border-border p-3">
					<h3 class="mb-2 flex items-center gap-1.5 text-sm font-medium text-muted-foreground">
						<FolderGit2 class="h-3.5 w-3.5" /> Submissions per shipper
					</h3>
					<BarChart
						data={submissionsDistributionBar}
						formatValue={(n) => Math.round(n).toLocaleString()}
						vertical
					/>
				</div>
			</div>
		</section>
	{/if}

	<ServerStatus />
</div>
