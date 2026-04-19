<script lang="ts">
	import { Badge } from '$lib/components/ui/badge';
	import Code from '@lucide/svelte/icons/code';
	import Globe from '@lucide/svelte/icons/globe';
	import { imageUrl, title, truncate, tryVideoOnError } from '$lib/search';
	import type { SearchResult } from '$lib/types';

	let { results }: { results: SearchResult[] } = $props();
</script>

<div class="flex flex-col gap-4">
	{#each results as r (r.id)}
		<div class="flex gap-4">
			<div class="flex-1">
				<div class="flex items-center gap-2">
					<h3 class="text-lg font-medium">{title(r)}</h3>
					<Badge variant="secondary" class="text-xs">{r.ysws}</Badge>
					{#if r.country}
						<span class="text-xs text-muted-foreground">{r.country}</span>
					{/if}
				</div>
				<p class="mt-1 text-sm text-muted-foreground">{truncate(r.description)}</p>
				<div class="mt-2 flex gap-3">
					{#if r.code_url}
						<a
							href={r.code_url}
							target="_blank"
							rel="noopener external"
							class="inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
						>
							<Code class="h-3 w-3" /> Code
						</a>
					{/if}
					{#if r.demo_url}
						<a
							href={r.demo_url}
							target="_blank"
							rel="noopener external"
							class="inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
						>
							<Globe class="h-3 w-3" /> Demo
						</a>
					{/if}
					<span class="text-xs text-muted-foreground">
						Score: {r.score.toFixed(3)}
					</span>
				</div>
			</div>
			{#if r.has_media}
				<img
					src={imageUrl(r.id)}
					alt={title(r)}
					class="hidden h-24 w-36 shrink-0 rounded bg-muted object-cover sm:block"
					loading="lazy"
					onerror={tryVideoOnError}
				/>
			{:else}
				<div
					class="hidden h-24 w-36 shrink-0 items-center justify-center rounded bg-muted text-sm text-muted-foreground sm:flex"
				>
					No Image :(
				</div>
			{/if}
		</div>
		<hr class="border-border" />
	{/each}
</div>
