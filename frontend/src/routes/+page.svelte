<script lang="ts">
	import { Input } from '$lib/components/ui/input';
	import { Button } from '$lib/components/ui/button';
	import Search from '@lucide/svelte/icons/search';
	import LayoutGrid from '@lucide/svelte/icons/layout-grid';
	import SearchView from '$lib/components/SearchView.svelte';
	import CardsView from '$lib/components/CardsView.svelte';
	import { API_BASE } from '$lib/search';
	import type { SearchResult, SearchResults, SearchTimings } from '$lib/types';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import Spinner from '$lib/components/ui/spinner/spinner.svelte';
	import TableIcon from '@lucide/svelte/icons/table';
	import X from '@lucide/svelte/icons/x';
	import { SvelteURLSearchParams } from 'svelte/reactivity';
	import { untrack } from 'svelte';

	type ViewMode = 'search' | 'cards';

	let query = $state(page.url.searchParams.get('q') ?? '');
	let results = $state<SearchResult[]>([]);
	let loading = $state(false);
	let searched = $state(false);
	let viewMode = $state<ViewMode>('search');
	let lastSearchedQuery = $state('');
	let lastSubmittedQuery = $state('');
	let currentPage = $state(1);
	let totalResults = $state(0);
	let perPage = $state(20);
	let timings = $state<SearchTimings | null>(null);
	let totalPages = $derived(Math.max(1, Math.ceil(totalResults / perPage)));

	async function doSearch(q: string, page = 1) {
		lastSearchedQuery = q;

		if (!q) {
			results = [];
			searched = false;
			totalResults = 0;
			timings = null;
			return;
		}

		loading = true;
		searched = true;
		try {
			const res = await fetch(
				`${API_BASE}/api/search?q=${encodeURIComponent(q)}&limit=${perPage}&page=${page}`
			);
			const body: SearchResults = await res.json();
			results = body.data;
			totalResults = body.total;
			currentPage = body.page;
			timings = body.timings;
		} catch {
			results = [];
			totalResults = 0;
			timings = null;
		} finally {
			loading = false;
		}
	}

	function goToPage(p: number) {
		if (p < 1 || p > totalPages || loading) return;
		const q = query.trim();
		const params = new SvelteURLSearchParams();
		if (q) params.set('q', q);
		if (p > 1) params.set('p', String(p));
		const href = resolve(`/?${params.toString()}`);
		goto(href, { replaceState: true, keepFocus: true, noScroll: true });
	}

	async function submitSearch() {
		const q = query.trim();

		if (q === lastSubmittedQuery) {
			return;
		}

		lastSubmittedQuery = q;
		const href = resolve(q ? `/?q=${encodeURIComponent(q)}` : '/');

		await goto(href, { replaceState: true, keepFocus: true, noScroll: true });
	}

	$effect(() => {
		const q = page.url.searchParams.get('q') ?? '';
		const p = Math.max(1, Number(page.url.searchParams.get('p') ?? '1'));

		if (q !== untrack(() => lastSearchedQuery) || p !== untrack(() => currentPage)) {
			query = q;
			if (q) {
				lastSubmittedQuery = q;
				void doSearch(q, p);
			} else {
				results = [];
				searched = false;
				lastSearchedQuery = '';
				currentPage = 1;
			}
		}
	});

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') void submitSearch();
	}

	function clearSearch() {
		if (!query) return;
		query = '';
		currentPage = 1;
		totalResults = 0;
		timings = null;
	}
</script>

<svelte:head>
	<title>Otter</title>
</svelte:head>

<div class="overflow-none mx-auto flex min-h-screen max-w-4xl flex-col px-4 py-8">
	<div class="mb-8 text-center" class:mt-[20vh]={!searched} class:mt-0={searched}>
		<h1 class="mb-2 text-3xl font-bold tracking-tight">Otter</h1>
		<p class="mb-6 text-sm text-muted-foreground">Search engine for Hack Club projects!</p>

		<div class="mx-auto flex max-w-xl gap-2">
			<div class="relative w-full">
				<Input
					type="text"
					placeholder="Search projects…"
					bind:value={query}
					onkeydown={handleKeydown}
					class="h-9 pr-9"
				/>
				{#if query}
					<button
						type="button"
						onclick={clearSearch}
						aria-label="Clear search"
						class="absolute top-1/2 right-2 -translate-y-1/2 cursor-pointer text-muted-foreground transition-colors hover:text-foreground"
					>
						<X class="h-4 w-4" />
					</button>
				{/if}
			</div>
			<Button
				onclick={() => void submitSearch()}
				disabled={loading || query.trim() === lastSubmittedQuery}
				size="lg"
			>
				<Search class="mr-2 h-4 w-4" />
				Search
			</Button>
		</div>

		<div class="flex flex-col items-center">
			<p
				class="m-3 flex flex-wrap items-center justify-center gap-x-2 gap-y-1 text-center text-xs text-muted-foreground"
			>
				<span class="font-medium tracking-wide text-foreground/80">tip:</span>
				<span class="opacity-70">use</span>
				<span
					class="rounded bg-muted px-1.5 py-0.5 font-mono text-[11px] whitespace-nowrap text-foreground"
					>user:username</span
				>
				<span class="opacity-70">to search for projects by a user</span>
			</p>
			<a
				href={resolve('/explore')}
				class="mt-2 inline-flex items-center gap-1 text-sm text-muted-foreground underline underline-offset-2 hover:text-foreground"
			>
				<TableIcon class="h-3 w-3" /> Explore all projects
			</a>
		</div>
	</div>

	{#if searched}
		<div class="mb-4 flex items-center justify-between">
			<span class="flex items-center gap-2 text-sm text-muted-foreground">
				{#if loading}
					<Spinner /><span>Searching…</span>
				{:else}
					<span title="displaying {results.length}/{totalResults}"
						>{totalResults} result{totalResults !== 1 ? 's' : ''}</span
					>
					{#if timings}
						<span
							class="text-xs opacity-60"
							title={`embeddings: ${timings.embeddings_ms.toFixed(1)}ms, query: ${timings.query_ms.toFixed(1)}ms`}
						>
							in {Math.round(timings.embeddings_ms + timings.query_ms)}ms
						</span>
					{/if}
				{/if}
			</span>

			<div class="flex gap-1">
				<Button
					variant={viewMode === 'search' ? 'default' : 'ghost'}
					size="sm"
					onclick={() => (viewMode = 'search')}
				>
					<Search class="h-4 w-4" />
				</Button>
				<Button
					variant={viewMode === 'cards' ? 'default' : 'ghost'}
					size="sm"
					onclick={() => (viewMode = 'cards')}
				>
					<LayoutGrid class="h-4 w-4" />
				</Button>
			</div>
		</div>

		{#if !loading && results.length === 0}
			<p class="py-12 text-center text-muted-foreground">No results found for "{query}"</p>
		{:else if !loading && results.length > 0}
			{#if viewMode === 'search'}
				<SearchView {results} />
			{:else}
				<CardsView {results} />
			{/if}
		{/if}

		{#if !loading && totalPages > 1}
			<div class="mt-6 flex items-center justify-center gap-2">
				<Button
					variant="outline"
					size="sm"
					disabled={currentPage <= 1}
					onclick={() => goToPage(currentPage - 1)}
				>
					Previous
				</Button>
				<span class="text-sm text-muted-foreground">
					Page {currentPage} of {totalPages}
				</span>
				<Button
					variant="outline"
					size="sm"
					disabled={currentPage >= totalPages}
					onclick={() => goToPage(currentPage + 1)}
				>
					Next
				</Button>
			</div>
		{/if}
	{/if}
</div>
