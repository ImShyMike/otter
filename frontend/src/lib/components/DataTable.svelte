<script lang="ts">
	import {
		type ColumnDef,
		type SortingState,
		type PaginationState,
		type Updater,
		FlexRender,
		createTable,
		renderSnippet,
		tableFeatures,
		rowSortingFeature,
		rowPaginationFeature,
		columnVisibilityFeature,
		createCoreRowModel
	} from '@tanstack/svelte-table';
	import * as Table from '$lib/components/ui/table';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Input, AutocompleteInput } from '$lib/components/ui/input';
	import { Spinner } from '$lib/components/ui/spinner';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import ArrowUpDown from '@lucide/svelte/icons/arrow-up-down';
	import ArrowUp from '@lucide/svelte/icons/arrow-up';
	import ArrowDown from '@lucide/svelte/icons/arrow-down';
	import ChevronLeft from '@lucide/svelte/icons/chevron-left';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import ChevronsLeft from '@lucide/svelte/icons/chevrons-left';
	import ChevronsRight from '@lucide/svelte/icons/chevrons-right';
	import Plus from '@lucide/svelte/icons/plus';
	import Share2 from '@lucide/svelte/icons/share-2';
	import X from '@lucide/svelte/icons/x';
	import lzString from 'lz-string';
	import { API_BASE, title as projectTitle, imageUrl } from '$lib/search';
	import type { SearchResult } from '$lib/types';
	import { formatApproved } from '$lib/utils';
	import { onMount, untrack } from 'svelte';
	import { resolve } from '$app/paths';

	const { compressToEncodedURIComponent, decompressFromEncodedURIComponent } = lzString;

	interface QueryFilter {
		field: string;
		op: string;
		value?: string | number | boolean;
	}

	interface QueryRequest {
		filters: QueryFilter[];
		order_by?: string;
		sort_direction?: string;
		limit: number;
		page: number;
	}

	interface QueryResponse {
		data: SearchResult[];
		total: number;
		page: number;
		per_page: number;
	}

	const _features = tableFeatures({
		rowSortingFeature,
		rowPaginationFeature,
		columnVisibilityFeature
	});

	type TableFeatures = typeof _features;

	let data = $state<SearchResult[]>([]);
	let total = $state(0);
	let loading = $state(false);

	let sorting: SortingState = $state([]);
	let pagination: PaginationState = $state({ pageIndex: 0, pageSize: 50 });

	const FIELDS: Record<string, { type: string; label: string }> = {
		airtable_id: { type: 'text', label: 'Airtable ID' },
		ysws: { type: 'text', label: 'YSWS' },
		country: { type: 'text', label: 'Country' },
		description: { type: 'text', label: 'Description' },
		github_username: { type: 'text', label: 'GitHub User' },
		display_name: { type: 'text', label: 'Name' },
		code_url: { type: 'text', label: 'Code URL' },
		demo_url: { type: 'text', label: 'Demo URL' },
		archived_demo: { type: 'text', label: 'Archived Demo' },
		archived_repo: { type: 'text', label: 'Archived Repo' },
		hours: { type: 'int', label: 'Hours' },
		true_hours: { type: 'float', label: 'True Hours' },
		github_stars: { type: 'int', label: 'Stars' },
		approved_at: { type: 'timestamp', label: 'Approved At' },
		has_media: { type: 'bool', label: 'Has Media' }
	};

	const OPS_BY_TYPE: Record<string, string[]> = {
		text: [
			'eq',
			'neq',
			'contains',
			'not_contains',
			'starts_with',
			'ends_with',
			'is_null',
			'is_not_null'
		],
		int: ['eq', 'neq', 'gt', 'gte', 'lt', 'lte', 'is_null', 'is_not_null'],
		float: ['eq', 'neq', 'gt', 'gte', 'lt', 'lte', 'is_null', 'is_not_null'],
		timestamp: ['eq', 'neq', 'gt', 'gte', 'lt', 'lte', 'is_null', 'is_not_null'],
		bool: ['eq', 'neq', 'is_null', 'is_not_null']
	};

	const OP_LABELS: Record<string, string> = {
		eq: 'equals',
		neq: 'not equals',
		gt: 'greater than',
		gte: 'greater or equal',
		lt: 'less than',
		lte: 'less or equal',
		contains: 'contains',
		not_contains: 'not contains',
		starts_with: 'starts with',
		ends_with: 'ends with',
		is_null: 'is empty',
		is_not_null: 'is not empty'
	};

	const NO_VALUE_OPS = new Set(['is_null', 'is_not_null']);

	interface FilterRow {
		id: number;
		field: string;
		op: string;
		value: string;
	}

	const STORAGE_KEY = 'otter-explore-state';
	const SHARE_QUERY_KEY = 'view';

	const PAGE_SIZE_OPTIONS = [10, 25, 50, 100];
	const DESCRIPTION_PREVIEW_LENGTH = 120;

	function filterValueAsString(value: unknown): string {
		if (typeof value === 'string') return value;
		if (typeof value === 'number' || typeof value === 'boolean') return String(value);
		return '';
	}

	function previewDescription(value: string): string {
		const normalized = value.replace(/\s+/g, ' ').trim();
		if (normalized.length <= DESCRIPTION_PREVIEW_LENGTH) return normalized;
		return `${normalized.slice(0, DESCRIPTION_PREVIEW_LENGTH).trimEnd()}...`;
	}

	interface SavedState {
		filters: Omit<FilterRow, 'id'>[];
		sorting: SortingState;
		pageIndex: number;
		pageSize: number;
	}

	function toSavedState(): SavedState {
		return {
			filters: filters.map(({ field, op, value }) => ({ field, op, value })),
			sorting,
			pageIndex: pagination.pageIndex,
			pageSize: pagination.pageSize
		};
	}

	function normalizeSavedState(saved: SavedState): SavedState {
		const sanitizedFilters = (saved.filters || []).map((f) => {
			const field = FIELDS[f.field] ? f.field : 'ysws';
			const ops = OPS_BY_TYPE[FIELDS[field].type] ?? [];
			const op = ops.includes(f.op) ? f.op : (ops[0] ?? 'eq');
			const value = typeof f.value === 'string' ? f.value : String(f.value ?? '');
			return { field, op, value };
		});

		return {
			filters: sanitizedFilters,
			sorting: Array.isArray(saved.sorting) ? saved.sorting.slice(0, 1) : [],
			pageIndex: Math.max(0, Number(saved.pageIndex) || 0),
			pageSize: Math.min(100, Math.max(1, Number(saved.pageSize) || 50))
		};
	}

	function decodeStateFromQuery(): SavedState | null {
		if (typeof window === 'undefined') return null;
		const encoded = new URLSearchParams(window.location.search).get(SHARE_QUERY_KEY);
		if (!encoded) return null;

		try {
			const json = decompressFromEncodedURIComponent(encoded);
			if (!json) return null;
			const parsed = JSON.parse(json) as SavedState;
			return normalizeSavedState(parsed);
		} catch {
			return null;
		}
	}

	function encodeStateForQuery(state: SavedState): string {
		return compressToEncodedURIComponent(JSON.stringify(state));
	}

	function loadState(): {
		filters: FilterRow[];
		sorting: SortingState;
		pageIndex: number;
		pageSize: number;
		counter: number;
	} {
		const sharedState = decodeStateFromQuery();
		if (sharedState) {
			let counter = 0;
			const loaded = sharedState.filters.map((f) => ({ ...f, id: counter++ }));
			return {
				filters: loaded,
				sorting: sharedState.sorting,
				pageIndex: sharedState.pageIndex,
				pageSize: sharedState.pageSize,
				counter
			};
		}

		if (typeof localStorage === 'undefined') {
			return {
				filters: [{ id: 0, field: 'approved_at', op: 'is_not_null', value: '' }],
				sorting: [],
				pageIndex: 0,
				pageSize: 50,
				counter: 1
			};
		}

		try {
			const raw = localStorage.getItem(STORAGE_KEY);
			if (raw) {
				const saved = normalizeSavedState(JSON.parse(raw) as SavedState);
				let counter = 0;
				const loaded = saved.filters.map((f) => ({ ...f, id: counter++ }));
				return {
					filters: loaded,
					sorting: saved.sorting,
					pageIndex: saved.pageIndex,
					pageSize: saved.pageSize,
					counter
				};
			}
		} catch {
			/* ignore */
		}
		let counter = 0;
		return {
			filters: [{ id: counter++, field: 'approved_at', op: 'is_not_null', value: '' }],
			sorting: [],
			pageIndex: 0,
			pageSize: 50,
			counter
		};
	}

	function saveState() {
		if (typeof localStorage === 'undefined') return;

		const state = toSavedState();
		try {
			localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
		} catch {
			/* ignore */
		}
	}

	let shareStatus = $state<'idle' | 'copied' | 'failed'>('idle');

	async function copyShareLink() {
		if (typeof window === 'undefined') return;

		const url = new URL(window.location.href);
		url.searchParams.set(SHARE_QUERY_KEY, encodeStateForQuery(toSavedState()));
		window.history.replaceState(null, '', `${url.pathname}${url.search}${url.hash}`);

		try {
			await navigator.clipboard.writeText(url.toString());
			shareStatus = 'copied';
		} catch {
			shareStatus = 'failed';
		}

		setTimeout(() => {
			shareStatus = 'idle';
		}, 2000);
	}

	function clearFilters() {
		filters = [];
		sorting = [];
		onFilterChange();
	}

	const initial = loadState();
	let filterIdCounter = initial.counter;
	let filters = $state<FilterRow[]>(initial.filters);
	sorting = initial.sorting;
	pagination = { pageIndex: initial.pageIndex, pageSize: initial.pageSize };

	function onPageSizeChange(event: Event) {
		const target = event.currentTarget as HTMLSelectElement;
		const pageSize = Math.min(100, Math.max(1, Number.parseInt(target.value, 10) || 50));
		pagination = { pageIndex: 0, pageSize };
	}

	let pageJumpValue = $state(String(initial.pageIndex + 1));

	$effect(() => {
		pageJumpValue = String(pagination.pageIndex + 1);
	});

	function jumpToPage() {
		const maxPage = Math.max(1, pageCount || 1);
		const requested = Number.parseInt(pageJumpValue, 10);
		const safePage = Number.isFinite(requested)
			? Math.min(maxPage, Math.max(1, requested))
			: pagination.pageIndex + 1;

		pageJumpValue = String(safePage);
		setPagination((prev) => ({ ...prev, pageIndex: safePage - 1 }));
	}

	function onPageJumpKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			event.preventDefault();
			jumpToPage();
		}
	}

	let yswsOptions = $state<string[]>([]);

	onMount(async () => {
		try {
			const res = await fetch(`${API_BASE}/api/ysws/list`);
			yswsOptions = await res.json();
		} catch {
			yswsOptions = [];
		}
	});

	function addFilter() {
		filters.push({ id: filterIdCounter++, field: 'ysws', op: 'eq', value: '' });
		onFilterChange();
	}

	function removeFilter(id: number) {
		filters = filters.filter((f) => f.id !== id);
		onFilterChange();
	}

	function onFieldChange(filter: FilterRow) {
		const ops = OPS_BY_TYPE[FIELDS[filter.field].type] ?? [];
		if (!ops.includes(filter.op)) filter.op = ops[0] ?? 'eq';
		filter.value = '';
		onFilterChange();
	}

	function getAvailableOps(field: string) {
		return OPS_BY_TYPE[FIELDS[field]?.type ?? 'text'] ?? [];
	}

	function setYSWSFilter(ysws: string) {
		const existing = filters.find((f) => f.field === 'ysws' && f.op === 'eq');
		if (existing) {
			existing.value = ysws;
			onFilterChange();
		} else {
			filters.push({ id: filterIdCounter++, field: 'ysws', op: 'eq', value: ysws });
			onFilterChange();
		}
	}

	const fieldMap: Record<string, string> = {
		airtable_id: 'airtable_id',
		ysws: 'ysws',
		country: 'country',
		description: 'description',
		github_username: 'github_username',
		code_url: 'code_url',
		demo_url: 'demo_url',
		archived_demo: 'archived_demo',
		archived_repo: 'archived_repo',
		github_stars: 'github_stars',
		hours: 'hours',
		true_hours: 'true_hours',
		has_media: 'has_media',
		approved_at: 'approved_at',
		display_name: 'display_name'
	};

	let fetchVersion = 0;

	async function fetchData() {
		const version = ++fetchVersion;
		loading = true;
		try {
			const queryFilters: QueryFilter[] = filters
				.filter((f) => {
					const rawValue = filterValueAsString(f.value);
					const normalizedValue = rawValue.trim();

					if (NO_VALUE_OPS.has(f.op)) return true;
					if (normalizedValue === '') return false;
					if (f.field === 'ysws' && ['eq', 'neq'].includes(f.op) && yswsOptions.length > 0) {
						return yswsOptions.includes(normalizedValue);
					}
					return true;
				})
				.map((f) => {
					const ft = FIELDS[f.field]?.type ?? 'text';
					const filter: QueryFilter = { field: f.field, op: f.op };
					const rawValue = filterValueAsString(f.value);
					const normalizedValue = rawValue.trim();
					if (!NO_VALUE_OPS.has(f.op)) {
						if (ft === 'int') {
							const parsed = Number.parseInt(normalizedValue, 10);
							filter.value = Number.isFinite(parsed) ? parsed : 0;
						} else if (ft === 'float') {
							const parsed = Number.parseFloat(normalizedValue);
							filter.value = Number.isFinite(parsed) ? parsed : 0;
						} else if (ft === 'bool') {
							filter.value = normalizedValue === 'true' || normalizedValue === '1';
						} else {
							filter.value = normalizedValue;
						}
					}
					return filter;
				});

			const body: QueryRequest = {
				filters: queryFilters,
				limit: pagination.pageSize,
				page: pagination.pageIndex + 1
			};

			if (sorting.length > 0) {
				const sort = sorting[0];
				const field = fieldMap[sort.id];
				if (field) {
					body.order_by = field;
					body.sort_direction = sort.desc ? 'desc' : 'asc';
				}
			}

			const res = await fetch(`${API_BASE}/api/query`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(body)
			});

			if (version !== fetchVersion) return;
			const result: QueryResponse = await res.json();
			data = result.data.map((d) => ({ ...d, score: 0 }));
			total = result.total;
		} catch {
			if (version !== fetchVersion) return;
			data = [];
			total = 0;
		} finally {
			if (version === fetchVersion) loading = false;
		}
	}

	function setSorting(updater: Updater<SortingState>) {
		sorting = updater instanceof Function ? updater(sorting) : updater;
		pagination = { ...pagination, pageIndex: 0 };
	}

	function setPagination(updater: Updater<PaginationState>) {
		const next = updater instanceof Function ? updater(pagination) : updater;
		pagination = {
			...next,
			pageSize: Math.min(100, Math.max(1, next.pageSize))
		};
	}

	let filterVersion = $state(0);
	let debounceTimer: ReturnType<typeof setTimeout> | undefined;

	function onFilterChange() {
		pagination = { ...pagination, pageIndex: 0 };
		filterVersion++;
	}

	$effect(() => {
		void sorting;
		void pagination;
		void filterVersion;
		untrack(() => {
			clearTimeout(debounceTimer);
			debounceTimer = setTimeout(() => {
				saveState();
				fetchData();
			}, 300);
		});

		return () => clearTimeout(debounceTimer);
	});

	const columns: ColumnDef<TableFeatures, SearchResult>[] = [
		{
			accessorKey: 'display_name',
			header: 'Name',
			cell: (info) => renderSnippet(nameSnippet, info.row.original),
			enableSorting: true
		},
		{
			accessorKey: 'github_username',
			header: 'User',
			cell: (info) => renderSnippet(usernameSnippet, info.getValue() as string | null),
			enableSorting: true
		},
		{
			accessorKey: 'ysws',
			header: 'YSWS',
			cell: (info) => renderSnippet(yswsSnippet, info.getValue() as string),
			enableSorting: true
		},
		{
			accessorKey: 'description',
			header: 'Description',
			cell: (info) => renderSnippet(descriptionSnippet, info.getValue() as string | null),
			enableSorting: true
		},
		{
			accessorKey: 'country',
			header: 'Country',
			cell: (info) => (info.getValue() as string | null) ?? '—',
			enableSorting: true
		},
		{
			accessorKey: 'approved_at',
			header: 'Approved',
			cell: (info) => formatApproved(info.getValue() as number | null),
			enableSorting: true
		},
		{
			accessorKey: 'hours',
			header: 'Hours',
			cell: (info) => {
				const value = info.getValue() as number | null;
				return value == null ? '—' : `${value}h`;
			},
			enableSorting: true
		},
		{
			accessorKey: 'true_hours',
			header: 'True Hours',
			cell: (info) => {
				const value = info.getValue() as number | null;
				return value == null ? '—' : `${value.toFixed(1)}h`;
			},
			enableSorting: true
		},
		{
			accessorKey: 'github_stars',
			header: 'Stars',
			cell: (info) => {
				const v = info.getValue() as number;
				return v > 0 ? v.toString() : '—';
			},
			enableSorting: true
		},
		{
			accessorKey: 'has_media',
			header: 'Media',
			cell: (info) => renderSnippet(mediaSnippet, info.row.original),
			enableSorting: true
		},
		{
			accessorKey: 'code_url',
			header: 'Code URL',
			cell: (info) =>
				renderSnippet(urlSnippet, {
					url: info.getValue() as string | null,
					label: 'Code'
				}),
			enableSorting: true
		},
		{
			accessorKey: 'demo_url',
			header: 'Demo URL',
			cell: (info) =>
				renderSnippet(urlSnippet, {
					url: info.getValue() as string | null,
					label: 'Demo'
				}),
			enableSorting: true
		},
		{
			accessorKey: 'archived_demo',
			header: 'Archived Demo',
			cell: (info) =>
				renderSnippet(urlSnippet, {
					url: info.getValue() as string | null,
					label: 'Archived Demo'
				}),
			enableSorting: true
		},
		{
			accessorKey: 'archived_repo',
			header: 'Archived Repo',
			cell: (info) =>
				renderSnippet(urlSnippet, {
					url: info.getValue() as string | null,
					label: 'Archived Repo'
				}),
			enableSorting: true
		},
		{
			accessorKey: 'airtable_id',
			header: 'Airtable ID',
			cell: (info) => info.getValue() as string,
			enableSorting: true
		}
	];

	const pageCount = $derived(Math.ceil(total / pagination.pageSize));

	const table = createTable({
		_features,
		_rowModels: {
			coreRowModel: createCoreRowModel()
		},
		get data() {
			return data;
		},
		columns,
		manualSorting: true,
		manualPagination: true,
		get pageCount() {
			return pageCount;
		},
		get rowCount() {
			return total;
		},
		state: {
			get sorting() {
				return sorting;
			},
			get pagination() {
				return pagination;
			}
		},
		onSortingChange: setSorting,
		onPaginationChange: setPagination
	});
</script>

{#snippet nameSnippet(r: SearchResult)}
	<a href={resolve('/project/[id]', { id: r.airtable_id })} class="hover:underline"
		>{projectTitle(r)}</a
	>
{/snippet}

{#snippet mediaSnippet(r: SearchResult)}
	{#if r.has_media}
		<a
			href={imageUrl(r.airtable_id)}
			target="_blank"
			rel="noopener external"
			class="text-muted-foreground underline underline-offset-2 hover:text-foreground">Media</a
		>
	{:else}
		—
	{/if}
{/snippet}

{#snippet usernameSnippet(username: string | null)}
	{#if username}
		<a
			href={`https://github.com/${username}`}
			target="_blank"
			rel="noopener external"
			class="text-muted-foreground underline underline-offset-2 hover:text-foreground"
		>
			@{username}
		</a>
	{:else}
		—
	{/if}
{/snippet}

{#snippet descriptionSnippet(description: string | null)}
	{#if !description}
		—
	{:else}
		<Tooltip.Root>
			<Tooltip.Trigger
				class="block max-w-xs cursor-default overflow-hidden text-left text-ellipsis whitespace-nowrap text-foreground/90"
			>
				{previewDescription(description)}
			</Tooltip.Trigger>
			<Tooltip.Portal>
				<Tooltip.Content class="max-w-96 wrap-break-word whitespace-pre-wrap">
					{description}
				</Tooltip.Content>
			</Tooltip.Portal>
		</Tooltip.Root>
	{/if}
{/snippet}

{#snippet yswsSnippet(value: string)}
	<button
		class="cursor-pointer"
		onclick={() => {
			setYSWSFilter(value);
		}}
	>
		<Badge variant="secondary" class="cursor-pointer">{value}</Badge>
	</button>
{/snippet}

{#snippet urlSnippet(link: { url: string | null; label: string })}
	{#if link.url}
		<a
			href={link.url}
			target="_blank"
			rel="noopener external"
			class="text-muted-foreground underline underline-offset-2 hover:text-foreground"
		>
			{link.label}
		</a>
	{:else}
		—
	{/if}
{/snippet}

<Tooltip.Provider>
	<div class="space-y-4">
		<div class="flex flex-col gap-2">
			{#each filters as filter (filter.id)}
				<div class="flex flex-wrap items-center gap-2">
					<select
						bind:value={filter.field}
						onchange={() => onFieldChange(filter)}
						class="h-8 cursor-pointer rounded-lg border border-input bg-transparent px-2.5 py-1 text-sm"
					>
						{#each Object.entries(FIELDS) as [key, meta] (key)}
							<option value={key}>{meta.label}</option>
						{/each}
					</select>
					<select
						bind:value={filter.op}
						onchange={onFilterChange}
						class="h-8 cursor-pointer rounded-lg border border-input bg-transparent px-2.5 py-1 pr-7 text-sm"
					>
						{#each getAvailableOps(filter.field) as op (op)}
							<option value={op}>{OP_LABELS[op]}</option>
						{/each}
					</select>
					{#if !NO_VALUE_OPS.has(filter.op)}
						{#if filter.field === 'ysws' && ['eq', 'neq'].includes(filter.op) && yswsOptions.length > 0}
							<AutocompleteInput
								bind:value={filter.value}
								options={yswsOptions}
								placeholder="Filter YSWS…"
								onselect={onFilterChange}
							/>
						{:else if FIELDS[filter.field]?.type === 'bool'}
							<select
								bind:value={filter.value}
								onchange={onFilterChange}
								class="h-8 cursor-pointer rounded-lg border border-input bg-transparent px-2.5 py-1 text-sm"
							>
								<option value="">Select…</option>
								<option value="true">true</option>
								<option value="false">false</option>
							</select>
						{:else}
							<Input
								type={FIELDS[filter.field]?.type === 'int' || FIELDS[filter.field]?.type === 'float'
									? 'number'
									: 'text'}
								placeholder={FIELDS[filter.field]?.type === 'timestamp' ? '2025-01-01' : 'value'}
								bind:value={filter.value}
								oninput={onFilterChange}
								class="h-8 w-40"
							/>
						{/if}
					{/if}
					<Button variant="ghost" size="icon-sm" onclick={() => removeFilter(filter.id)}>
						<X class="h-3.5 w-3.5" />
					</Button>
				</div>
			{/each}
			<div class="flex flex-wrap items-center gap-2">
				<Button variant="outline" size="sm" onclick={addFilter}>
					<Plus class="mr-1 h-3.5 w-3.5" />
					Add Filter
				</Button>
				<Button variant="outline" size="sm" onclick={clearFilters} disabled={filters.length === 0}>
					Clear Filters
				</Button>
				<Button variant="outline" size="sm" onclick={copyShareLink}>
					<Share2 class="mr-1 h-3.5 w-3.5" />
					Share View
				</Button>
				{#if shareStatus === 'copied'}
					<span class="text-xs text-muted-foreground">Copied link</span>
				{:else if shareStatus === 'failed'}
					<span class="text-xs text-muted-foreground">Copy failed</span>
				{/if}
			</div>
		</div>

		<div class="relative rounded border">
			{#if loading && data.length > 0}
				<div
					class="absolute inset-0 z-10 flex items-center justify-center rounded bg-background/50"
				>
					<Spinner />
				</div>
			{/if}
			<Table.Table>
				<Table.Header>
					{#each table.getHeaderGroups() as headerGroup (headerGroup.id)}
						<Table.Row>
							{#each headerGroup.headers as header (header.id)}
								<Table.Head>
									{#if header.column.getCanSort()}
										<button
											class="flex cursor-pointer items-center gap-1"
											onclick={() => header.column.toggleSorting()}
										>
											<FlexRender {header} />
											{#if header.column.getIsSorted() === 'asc'}
												<ArrowUp class="h-3 w-3" />
											{:else if header.column.getIsSorted() === 'desc'}
												<ArrowDown class="h-3 w-3" />
											{:else}
												<ArrowUpDown class="h-3 w-3 text-muted-foreground/50" />
											{/if}
										</button>
									{:else}
										<FlexRender {header} />
									{/if}
								</Table.Head>
							{/each}
						</Table.Row>
					{/each}
				</Table.Header>
				<Table.Body>
					{#if loading && data.length === 0}
						<Table.Row>
							<Table.Cell colspan={columns.length} class="h-24 text-center">
								<div class="flex items-center justify-center gap-2">
									<Spinner />
									<span class="text-sm text-muted-foreground">Loading…</span>
								</div>
							</Table.Cell>
						</Table.Row>
					{:else if !loading && data.length === 0}
						<Table.Row>
							<Table.Cell colspan={columns.length} class="h-24 text-center text-muted-foreground">
								No results.
							</Table.Cell>
						</Table.Row>
					{:else}
						{#each table.getRowModel().rows as row (row.id)}
							<Table.Row>
								{#each row.getVisibleCells() as cell (cell.id)}
									<Table.Cell>
										<FlexRender {cell} />
									</Table.Cell>
								{/each}
							</Table.Row>
						{/each}
					{/if}
				</Table.Body>
			</Table.Table>
		</div>

		<div class="flex items-center justify-between">
			<span class="text-sm text-muted-foreground">
				{total} total result{total !== 1 ? 's' : ''}
			</span>
			<div class="flex items-center gap-2">
				<label class="text-sm text-muted-foreground" for="page-size">Rows</label>
				<select
					id="page-size"
					value={pagination.pageSize}
					onchange={onPageSizeChange}
					class="h-7 cursor-pointer rounded-lg border border-input bg-transparent px-2.5 py-1 pr-8 text-sm"
				>
					{#each PAGE_SIZE_OPTIONS as size (size)}
						<option value={size}>{size}</option>
					{/each}
				</select>
				<Button
					variant="outline"
					size="icon-sm"
					disabled={!table.getCanPreviousPage()}
					onclick={() => table.firstPage()}
				>
					<ChevronsLeft class="h-4 w-4" />
				</Button>
				<Button
					variant="outline"
					size="icon-sm"
					disabled={!table.getCanPreviousPage()}
					onclick={() => table.previousPage()}
				>
					<ChevronLeft class="h-4 w-4" />
				</Button>
				<div class="flex items-center gap-1 text-sm text-muted-foreground">
					<span>Page</span>
					<Input
						id="page-jump"
						type="text"
						inputmode="numeric"
						pattern="[0-9]*"
						min="1"
						max={Math.max(1, pageCount || 1)}
						bind:value={pageJumpValue}
						onkeydown={onPageJumpKeydown}
						onblur={jumpToPage}
						class="h-7 w-12 p-1! text-center"
					/>
					<span>of {pageCount || 1}</span>
				</div>
				<Button
					variant="outline"
					size="icon-sm"
					disabled={!table.getCanNextPage()}
					onclick={() => table.nextPage()}
				>
					<ChevronRight class="h-4 w-4" />
				</Button>
				<Button
					variant="outline"
					size="icon-sm"
					disabled={!table.getCanNextPage()}
					onclick={() => table.lastPage()}
				>
					<ChevronsRight class="h-4 w-4" />
				</Button>
			</div>
		</div>
	</div>
</Tooltip.Provider>
