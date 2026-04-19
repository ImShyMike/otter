<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import * as Card from '$lib/components/ui/card';
	import Code from '@lucide/svelte/icons/code';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import { imageUrl, title, truncate, tryVideoOnError } from '$lib/search';
	import type { SearchResult } from '$lib/types';

	let { results }: { results: SearchResult[] } = $props();
</script>

<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
	{#each results as r (r.id)}
		<Card.Card class="flex flex-col">
			{#if r.has_media}
				<img
					src={imageUrl(r.id)}
					alt={title(r)}
					class="h-60 w-full bg-muted object-cover border-b"
					loading="lazy"
					onerror={tryVideoOnError}
				/>
			{:else}
				<div
					class="flex h-60 w-full items-center justify-center bg-muted text-sm text-muted-foreground"
				>
					No Image :(
				</div>
			{/if}
			<Card.Header>
				<div class="flex items-center gap-2">
					<Card.Title class="text-base">{title(r)}</Card.Title>
					<Badge variant="secondary" class="text-xs">{r.ysws}</Badge>
				</div>
				{#if r.country}
					<Card.Description>{r.country}</Card.Description>
				{/if}
			</Card.Header>
			<Card.Content class="flex-1">
				<p class="text-sm text-muted-foreground">{truncate(r.description, 120)}</p>
			</Card.Content>
			{#if r.code_url || r.demo_url}
				<Card.Footer class="gap-2">
					{#if r.code_url}
						<a href={r.code_url} target="_blank" rel="noopener external">
							<Button variant="outline" size="sm">
								<Code class="mr-1 h-3 w-3" /> Code
							</Button>
						</a>
					{/if}
					{#if r.demo_url}
						<a href={r.demo_url} target="_blank" rel="noopener external">
							<Button variant="outline" size="sm">
								<ExternalLink class="mr-1 h-3 w-3" /> Demo
							</Button>
						</a>
					{/if}
				</Card.Footer>
			{/if}
		</Card.Card>
	{/each}
</div>
