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
	import { Input } from '$lib/components/ui/input';
	import { Spinner } from '$lib/components/ui/spinner';
	import Code from '@lucide/svelte/icons/code';
	import Globe from '@lucide/svelte/icons/globe';
	import ArrowUpDown from '@lucide/svelte/icons/arrow-up-down';
	import ArrowUp from '@lucide/svelte/icons/arrow-up';
	import ArrowDown from '@lucide/svelte/icons/arrow-down';
	import ChevronLeft from '@lucide/svelte/icons/chevron-left';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import ChevronsLeft from '@lucide/svelte/icons/chevrons-left';
	import ChevronsRight from '@lucide/svelte/icons/chevrons-right';
	import { API_BASE, title as projectTitle } from '$lib/search';
	import type { SearchResult } from '$lib/types';
	import { formatHours, formatApproved } from '$lib/utils';
	import { onMount } from 'svelte';

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

	let filterYsws = $state('');
	let filterCountry = $state('');
	let filterUser = $state('');

	let yswsOptions = $state<string[]>([]);
	let showYswsDropdown = $state(false);

	const filteredYswsOptions = $derived(
		filterYsws.trim()
			? yswsOptions.filter((o) => o.toLowerCase().includes(filterYsws.trim().toLowerCase()))
			: yswsOptions
	);

	onMount(async () => {
		try {
			const res = await fetch(`${API_BASE}/api/ysws/list`);
			yswsOptions = await res.json();
		} catch {
			yswsOptions = [];
		}
	});

	const fieldMap: Record<string, string> = {
		ysws: 'ysws',
		country: 'country',
		github_username: 'github_username',
		github_stars: 'github_stars',
		hours: 'hours',
		approved_at: 'approved_at',
		display_name: 'display_name'
	};

	let fetchVersion = 0;

	async function fetchData() {
		const version = ++fetchVersion;
		loading = true;
		try {
			const filters: QueryFilter[] = [];

			if (filterYsws.trim()) {
				filters.push({ field: 'ysws', op: 'eq', value: filterYsws.trim() });
			}
			if (filterCountry.trim()) {
				filters.push({ field: 'country', op: 'contains', value: filterCountry.trim() });
			}
			if (filterUser.trim()) {
				filters.push({ field: 'github_username', op: 'contains', value: filterUser.trim() });
			}

			const body: QueryRequest = {
				filters,
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
		pagination = updater instanceof Function ? updater(pagination) : updater;
	}

	let filterVersion = $state(0);

	function onFilterChange() {
		pagination = { ...pagination, pageIndex: 0 };
		filterVersion++;
	}

	function selectYsws(value: string) {
		filterYsws = value;
		showYswsDropdown = false;
		onFilterChange();
	}

	$effect(() => {
		void sorting;
		void pagination;
		void filterVersion;
		fetchData();
	});

	const columns: ColumnDef<TableFeatures, SearchResult>[] = [
		{
			accessorKey: 'display_name',
			header: 'Name',
			cell: (info) => projectTitle(info.row.original),
			enableSorting: true
		},
		{
			accessorKey: 'ysws',
			header: 'YSWS',
			cell: (info) => renderSnippet(yswsSnippet, info.getValue() as string),
			enableSorting: true
		},
		{
			accessorKey: 'github_username',
			header: 'User',
			cell: (info) => {
				const v = info.getValue() as string | null;
				return v ? `@${v}` : '—';
			},
			enableSorting: true
		},
		{
			accessorKey: 'country',
			header: 'Country',
			cell: (info) => (info.getValue() as string | null) ?? '—',
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
			accessorKey: 'hours',
			header: 'Hours',
			cell: (info) => formatHours(info.row.original),
			enableSorting: true
		},
		{
			accessorKey: 'approved_at',
			header: 'Approved',
			cell: (info) => formatApproved(info.getValue() as number | null),
			enableSorting: true
		},
		{
			id: 'links',
			header: 'Links',
			cell: (info) => renderSnippet(linksSnippet, info.row.original),
			enableSorting: false
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

{#snippet yswsSnippet(value: string)}
	<Badge variant="secondary">{value}</Badge>
{/snippet}

{#snippet linksSnippet(r: SearchResult)}
	<div class="flex justify-start gap-2">
		{#if r.demo_url}
			<a href={r.demo_url} target="_blank" rel="noopener external">
				<Globe class="h-4 w-4 text-muted-foreground hover:text-foreground" />
			</a>
		{/if}
		{#if r.code_url}
			<a href={r.code_url} target="_blank" rel="noopener external">
				<Code class="h-4 w-4 text-muted-foreground hover:text-foreground" />
			</a>
		{/if}
		{#if r.archived_demo}
			<a href={r.archived_demo} target="_blank" rel="noopener external" title="Archived Demo">
				<Globe class="h-4 w-4 text-muted-foreground hover:text-foreground" />
			</a>
		{/if}
		{#if r.archived_repo}
			<a href={r.archived_repo} target="_blank" rel="noopener external" title="Archived Repo">
				<Code class="h-4 w-4 text-muted-foreground hover:text-foreground" />
			</a>
		{/if}
	</div>
{/snippet}

<div class="space-y-4">
	<div class="flex flex-wrap gap-2">
		<div
			class="relative"
			onfocusin={() => (showYswsDropdown = true)}
			onfocusout={(e) => {
				if (!e.currentTarget.contains(e.relatedTarget as Node)) showYswsDropdown = false;
			}}
		>
			<Input
				type="text"
				placeholder="Filter YSWS…"
				bind:value={filterYsws}
				oninput={onFilterChange}
				class="h-8 w-44"
				autocomplete="off"
			/>
			{#if showYswsDropdown && filteredYswsOptions.length > 0}
				<div
					class="absolute top-full left-0 z-50 mt-1 max-h-48 w-56 overflow-y-auto rounded border bg-popover p-1 shadow-md"
				>
					{#each filteredYswsOptions as option (option)}
						<button
							class="w-full rounded-sm px-2 py-1.5 text-left text-sm hover:bg-muted"
							onmousedown={(e) => {
								e.preventDefault();
								selectYsws(option);
							}}
						>
							{option}
						</button>
					{/each}
				</div>
			{/if}
		</div>
		<Input
			type="text"
			placeholder="Filter country…"
			bind:value={filterCountry}
			oninput={onFilterChange}
			class="h-8 w-36"
		/>
		<Input
			type="text"
			placeholder="Filter user…"
			bind:value={filterUser}
			oninput={onFilterChange}
			class="h-8 w-36"
		/>
	</div>

	<div class="relative rounded border">
		{#if loading && data.length > 0}
			<div class="absolute inset-0 z-10 flex items-center justify-center rounded bg-background/50">
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
										class="flex items-center gap-1"
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
			<span class="text-sm text-muted-foreground">
				Page {pagination.pageIndex + 1} of {pageCount || 1}
			</span>
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
