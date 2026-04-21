<script lang="ts">
	import { Input } from '$lib/components/ui/input';
	import { Button } from '$lib/components/ui/button';
	import Search from '@lucide/svelte/icons/search';
	import LayoutGrid from '@lucide/svelte/icons/layout-grid';
	import SearchView from '$lib/components/SearchView.svelte';
	import CardsView from '$lib/components/CardsView.svelte';
	import { API_BASE } from '$lib/search';
	import type { SearchResult } from '$lib/types';
	import Spinner from '$lib/components/ui/spinner/spinner.svelte';
	import TableIcon from '@lucide/svelte/icons/table';
	import { resolve } from '$app/paths';

	type ViewMode = 'search' | 'cards';

	let query = $state('');
	let results = $state<SearchResult[]>([]);
	let loading = $state(false);
	let searched = $state(false);
	let viewMode = $state<ViewMode>('search');

	async function doSearch() {
		const q = query.trim();
		if (!q) return;

		loading = true;
		searched = true;
		try {
			const res = await fetch(`${API_BASE}/api/search?q=${encodeURIComponent(q)}&limit=30`);
			results = await res.json();
		} catch {
			results = [];
		} finally {
			loading = false;
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') doSearch();
	}
</script>

<div class="mx-auto flex min-h-screen max-w-4xl flex-col px-4 py-8">
	<div class="mb-8 text-center" class:mt-[20vh]={!searched} class:mt-0={searched}>
		<h1 class="mb-2 text-3xl font-bold tracking-tight">Otter</h1>
		<p class="mb-6 text-sm text-muted-foreground">Search engine for Hack Club projects!</p>

		<div class="mx-auto flex max-w-xl gap-2">
			<Input
				type="text"
				placeholder="Search projects…"
				bind:value={query}
				onkeydown={handleKeydown}
				class="h-9"
			/>
			<Button onclick={doSearch} disabled={loading} size="lg">
				<Search class="mr-2 h-4 w-4" />
				Search
			</Button>
		</div>

		<a
			href={resolve('/explore')}
			class="mt-3 inline-flex items-center gap-1 text-sm text-muted-foreground underline underline-offset-2 hover:text-foreground"
		>
			<TableIcon class="h-3 w-3" /> Explore all projects
		</a>
	</div>

	{#if searched}
		<div class="mb-4 flex items-center justify-between">
			<span class="flex items-center gap-2 text-sm text-muted-foreground">
				{#if loading}
					<Spinner /><span>Searching…</span>
				{:else}
					{results.length} result{results.length !== 1 ? 's' : ''}
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
	{/if}
</div>
