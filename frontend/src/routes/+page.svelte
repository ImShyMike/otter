<script lang="ts">
	import { Input } from '$lib/components/ui/input';
	import { Button } from '$lib/components/ui/button';
	import Search from '@lucide/svelte/icons/search';
	import LayoutGrid from '@lucide/svelte/icons/layout-grid';
	import TableIcon from '@lucide/svelte/icons/table';
	import SearchView from '$lib/components/SearchView.svelte';
	import CardsView from '$lib/components/CardsView.svelte';
	import TableView from '$lib/components/TableView.svelte';
	import { API_BASE } from '$lib/search';
	import type { SearchResult } from '$lib/types';

	type ViewMode = 'search' | 'cards' | 'table';

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
		<p class="mb-6 text-sm text-muted-foreground">
			Search through projects that Hack Clubbers have made!
		</p>

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
	</div>

	{#if searched}
		<div class="mb-4 flex items-center justify-between">
			<span class="text-sm text-muted-foreground">
				{#if loading}
					Searching…
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
				<Button
					variant={viewMode === 'table' ? 'default' : 'ghost'}
					size="sm"
					onclick={() => (viewMode = 'table')}
				>
					<TableIcon class="h-4 w-4" />
				</Button>
			</div>
		</div>

		{#if !loading && results.length === 0}
			<p class="py-12 text-center text-muted-foreground">No results found for "{query}"</p>
		{/if}

		{#if !loading && results.length > 0}
			{#if viewMode === 'search'}
				<SearchView {results} />
			{:else if viewMode === 'cards'}
				<CardsView {results} />
			{:else}
				<TableView {results} />
			{/if}
		{/if}
	{/if}
</div>
