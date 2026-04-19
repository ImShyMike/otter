<script lang="ts">
	import { Badge } from '$lib/components/ui/badge';
	import Code from '@lucide/svelte/icons/code';
	import Globe from '@lucide/svelte/icons/globe';
	import { imageUrl, title, truncate, tryVideoOnError } from '$lib/search';
	import type { SearchResult } from '$lib/types';
	import { formatHours, formatApproved } from '$lib/utils';

	let { results }: { results: SearchResult[] } = $props();
</script>

<div class="flex flex-col gap-4">
	{#each results as r (r.id)}
		<div class="flex gap-4">
			<div class="flex-1">
				<div class="flex flex-wrap items-center gap-2">
					<h3 class="text-lg font-medium">{title(r)}</h3>
					<Badge variant="secondary" class="text-xs">{r.ysws}</Badge>
					{#if r.github_stars > 0}
						<Badge variant="outline" class="text-xs">{r.github_stars} stars</Badge>
					{/if}
					{#if formatHours(r)}
						<Badge variant="outline" class="text-xs">{formatHours(r)}</Badge>
					{/if}
					{#if r.country}
						<span class="text-xs text-muted-foreground">{r.country}</span>
					{/if}
					{#if r.github_username}
						<a
							class="text-xs text-muted-foreground underline underline-offset-2 hover:text-foreground"
							href={`https://github.com/${r.github_username}`}
							target="_blank"
							rel="noopener external">@{r.github_username}</a
						>
					{/if}
				</div>
				<p class="mt-1 text-sm text-muted-foreground">{truncate(r.description)}</p>
				<div class="mt-2 flex flex-wrap gap-3">
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
					{#if r.archived_demo}
						<a
							href={r.archived_demo}
							target="_blank"
							rel="noopener external"
							class="inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
						>
							<Globe class="h-3 w-3" /> Archived Demo
						</a>
					{/if}
					{#if r.archived_repo}
						<a
							href={r.archived_repo}
							target="_blank"
							rel="noopener external"
							class="inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
						>
							<Code class="h-3 w-3" /> Archived Repo
						</a>
					{/if}
					<span class="text-xs text-muted-foreground">
						Score: {r.score.toFixed(3)}
					</span>
					{#if formatApproved(r.approved_at)}
						<span class="text-xs text-muted-foreground"
							>Approved {formatApproved(r.approved_at)}</span
						>
					{/if}
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
