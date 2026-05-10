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
	import { SvelteSet, SvelteURLSearchParams } from 'svelte/reactivity';
	import { untrack } from 'svelte';
	import Head from '$lib/components/Head.svelte';
	import StarIcon from '@lucide/svelte/icons/star';
	import ServerStatus from '$lib/components/ServerStatus.svelte';
	import Disclaimer from '$lib/components/Disclaimer.svelte';

	type ViewMode = 'search' | 'cards';

	type SuggestedSearch = {
		label: string;
		query: string;
	};

	const LOW_SCORE_THRESHOLD = 0.25;

	function dedupeByAirtableId(items: SearchResult[]) {
		const seen = new SvelteSet<string>();
		return items.filter((item) => {
			if (seen.has(item.airtable_id)) return false;
			seen.add(item.airtable_id);
			return true;
		});
	}

	const suggestedSearches: SuggestedSearch[] = [
		{ label: 'DoomPDF', query: 'DoomPDF' },
		{ label: 'VERT', query: 'VERT' },
		{ label: 'Specter', query: 'Specter' },
		{ label: 'Blind Defusal', query: 'Blind Defusal' },
		{ label: 'High Seas', query: '"High Seas"' },
		{ label: 'Art', query: 'art' },
		{ label: 'Music', query: 'music' },
		{ label: 'ShyMike', query: 'user:ImShyMike' }
	];

	let query = $state(page.url.searchParams.get('q') ?? '');
	let results = $state<SearchResult[]>([]);
	let showLowScore = $state(false);
	let dedupedResults = $derived(dedupeByAirtableId(results));
	let validResults = $derived(
		showLowScore
			? dedupedResults
			: dedupedResults.filter((r) => r.score !== null && r.score >= LOW_SCORE_THRESHOLD)
	);
	let loading = $state(false);
	let loadingMore = $state(false);
	let searched = $state(false);
	let viewMode = $state<ViewMode>('search');
	let lastSearchedQuery = $state('');
	let lastSubmittedQuery = $state('');
	let currentPage = $state(1);
	let totalResults = $state(0);
	let perPage = $state(20);
	let timings = $state<SearchTimings | null>(null);
	let hasMore = $derived(results.length < totalResults);
	let sentinel = $state<HTMLDivElement | null>(null);

	async function doSearch(q: string, page = 1, append = false) {
		lastSearchedQuery = q;

		if (!q) {
			results = [];
			searched = false;
			totalResults = 0;
			timings = null;
			currentPage = 1;
			return;
		}

		if (append) {
			loadingMore = true;
		} else {
			loading = true;
		}
		searched = true;
		try {
			const res = await fetch(
				`${API_BASE}/api/v1/search?q=${encodeURIComponent(q)}&limit=${perPage}&page=${page}`
			);
			const body: SearchResults = await res.json();
			results = append ? [...results, ...body.data] : body.data;
			totalResults = body.total;
			currentPage = body.page;
			timings = body.timings;
		} catch {
			if (!append) {
				results = [];
				totalResults = 0;
				timings = null;
			}
		} finally {
			loading = false;
			loadingMore = false;
		}
	}

	async function loadMore() {
		if (loading || loadingMore || !hasMore) return;
		const q = lastSearchedQuery;
		if (!q) return;
		await doSearch(q, currentPage + 1, true);
	}

	$effect(() => {
		if (!sentinel) return;
		const observer = new IntersectionObserver(
			(entries) => {
				if (entries.some((e) => e.isIntersecting)) void loadMore();
			},
			{ rootMargin: '400px' }
		);
		observer.observe(sentinel);
		return () => observer.disconnect();
	});

	function changeViewMode(mode: ViewMode) {
		viewMode = mode;
		const params = new SvelteURLSearchParams(page.url.search);
		if (mode === 'search') {
			params.delete('v');
		} else {
			params.set('v', mode);
		}
		const href = resolve(`/?${params.toString()}`);
		goto(href, { replaceState: true, keepFocus: true, noScroll: true });
	}

	async function submitSearch() {
		const q = query.trim();

		if (q === lastSubmittedQuery) {
			return;
		}

		lastSubmittedQuery = q;
		showLowScore = false;
		const href = resolve(q ? `/?q=${encodeURIComponent(q)}` : '/');

		await goto(href, { replaceState: true, keepFocus: true, noScroll: true });
	}

	$effect(() => {
		const q = page.url.searchParams.get('q') ?? '';
		const v = page.url.searchParams.get('v') as ViewMode | null;

		if (v === 'search' || v === 'cards') {
			viewMode = v;
		}

		if (q !== untrack(() => lastSearchedQuery)) {
			query = q;
			if (q) {
				lastSubmittedQuery = q;
				void doSearch(q, 1);
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
	}
</script>

<Head title="Otter" description="Search engine for Hack Club projects" />

<Disclaimer />

<div class="absolute top-4 left-4 z-50 sm:fixed sm:top-auto sm:bottom-4">
	<a
		href="https://github.com/ImShyMike/otter"
		target="_blank"
		rel="noopener"
		class="inline-flex size-12 items-center justify-center rounded-md bg-transparent text-sm font-medium ring-offset-background transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:outline-none"
		aria-label="View on GitHub"
		title="View on GitHub"
		data-umami-event="github-link"
	>
		<StarIcon class="size-6" />
	</a>
</div>

<div class="overflow-none mx-auto flex min-h-screen max-w-4xl flex-col px-4 py-4 pt-8">
	<div class="mb-8 text-center" class:mt-[20vh]={!searched} class:mt-0={searched}>
		<a href={resolve('/')}>
			<h1 class="mb-2 text-3xl font-bold tracking-tight">Otter</h1>
		</a>
		<p class="mb-6 text-sm text-muted-foreground">Search engine for Hack Club projects!</p>

		<div class="mx-auto flex max-w-xl gap-2">
			<div class="relative w-full">
				<Input
					type="text"
					placeholder="Search projects…"
					bind:value={query}
					onkeydown={handleKeydown}
					class="h-9 pr-9"
					autofocus
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
				disabled={loading || (query.trim() === lastSubmittedQuery && lastSubmittedQuery !== '')}
				size="lg"
				data-umami-event="search-submit"
			>
				<Search class="mr-2 h-4 w-4" />
				Search
			</Button>
		</div>

		<div class="mt-3 mb-2 text-center">
			<div class="mt-2 flex flex-wrap justify-center gap-2">
				{#each suggestedSearches as suggestion (suggestion.query)}
					<Button
						onclick={() => {
							query = suggestion.query;
							void submitSearch();
						}}
						variant={query.trim() === suggestion.query ? 'default' : 'outline'}
						size="sm"
						class="rounded-full"
						data-umami-event="search-suggestion"
						data-umami-event-query={suggestion.query}
						aria-label={`Search for ${suggestion.label}`}
					>
						{suggestion.label}
					</Button>
				{/each}
			</div>
		</div>

		<div class="flex flex-col items-center">
			<p class="mb-3 text-center text-xs leading-relaxed wrap-break-word text-muted-foreground">
				<span class="font-medium tracking-wide text-foreground/80">tip:</span>
				<span class="opacity-70">use </span>
				<span
					class="rounded bg-muted px-1 py-0.5 font-mono text-[11px] wrap-break-word whitespace-nowrap text-foreground"
					>"quoted phrase"</span
				>
				<span class="opacity-70"> for exact phrase matches, and </span>
				<span
					class="rounded bg-muted px-1 py-0.5 font-mono text-[11px] wrap-break-word whitespace-nowrap text-foreground"
					>user:username</span
				>
				<span class="opacity-70"> to search projects by user</span>
			</p>
			<a
				href={resolve('/explore')}
				class="mt-2 inline-flex items-center gap-1 text-sm text-muted-foreground underline underline-offset-2 hover:text-foreground"
				data-umami-event="explore-link"
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
					onclick={() => changeViewMode('search')}
					data-umami-event="view-mode-search"
				>
					<Search class="h-4 w-4" />
				</Button>
				<Button
					variant={viewMode === 'cards' ? 'default' : 'ghost'}
					size="sm"
					onclick={() => changeViewMode('cards')}
					data-umami-event="view-mode-cards"
				>
					<LayoutGrid class="h-4 w-4" />
				</Button>
			</div>
		</div>

		{#if !loading && results.length === 0}
			<p class="py-12 text-center text-muted-foreground">No results found for "{query}"</p>
		{:else if !loading && results.length > 0}
			{@const displayResults = validResults.length > 0 ? validResults : dedupedResults}
			{#if viewMode === 'search'}
				<SearchView results={displayResults} />
			{:else}
				<CardsView results={displayResults} />
			{/if}
		{/if}

		{@const hiddenCount = dedupedResults.length - validResults.length}
		{@const pageOffset = (currentPage - 1) * perPage}
		{@const trueHiddenCount = Math.max(0, totalResults - pageOffset - validResults.length)}
		{@const showHiddenResultsNotice = hiddenCount > 0 && validResults.length > 0}
		{#if showHiddenResultsNotice && !loading}
			<p class="mt-6 text-center text-sm text-muted-foreground">
				{trueHiddenCount} result{trueHiddenCount !== 1 ? 's' : ''} hidden...
				<button
					class="cursor-pointer underline hover:text-foreground"
					onclick={() => (showLowScore = !showLowScore)}
					data-umami-event="toggle-low-score"
				>
					{showLowScore ? 'Hide' : 'Show'} them?
				</button>
			</p>
		{/if}

		{#if !loading && results.length > 0 && hasMore && !showHiddenResultsNotice}
			<div bind:this={sentinel} class="mt-6 flex items-center justify-center py-4">
				{#if loadingMore}
					<Spinner /><span class="ml-2 text-sm text-muted-foreground">Loading more…</span>
				{/if}
			</div>
		{/if}
	{/if}

	<ServerStatus />
</div>
