<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import * as Card from '$lib/components/ui/card';
	import { ExpandableImage } from '$lib/components/ui/image';
	import Code from '@lucide/svelte/icons/code';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import Star from '@lucide/svelte/icons/star';
	import { imageUrl, title, truncate } from '$lib/search';
	import type { SearchResult } from '$lib/types';
	import { formatHours, formatApproved } from '$lib/utils';

	let { results }: { results: SearchResult[] } = $props();
</script>

<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
	{#each results as r (r.id)}
		<Card.Card class="flex flex-col">
			<ExpandableImage
				id={r.id}
				src={imageUrl(r.airtable_id)}
				alt={title(r)}
				missing={!r.has_media}
				buttonClass="w-full"
				thumbnailClass="h-60 w-full border-b bg-muted object-cover"
				transitionPrefix="cards-image"
			/>
			<Card.Header>
				<div class="flex flex-wrap items-center gap-2">
					<Card.Title class="text-base">{title(r)}</Card.Title>
					<Badge variant="secondary" class="text-xs">{r.ysws}</Badge>
					{#if r.github_stars > 0}
						<Badge variant="outline" class="text-xs">{r.github_stars} <Star /></Badge>
					{/if}
					{#if formatHours(r)}
						<Badge variant="outline" class="text-xs">{formatHours(r)}</Badge>
					{/if}
				</div>
				{#if r.country || r.github_username || formatApproved(r.approved_at)}
					<Card.Description>
						{r.country ?? ''}
						{#if r.country && r.github_username}
							·
						{/if}
						{#if r.github_username}
							<a
								class="text-xs text-muted-foreground underline underline-offset-2 hover:text-foreground"
								href={`https://github.com/${r.github_username}`}
								target="_blank"
								rel="noopener external">@{r.github_username}</a
							>
						{/if}
						{#if (r.country || r.github_username) && formatApproved(r.approved_at)}
							·
						{/if}
						{#if formatApproved(r.approved_at)}
							Approved {formatApproved(r.approved_at)}
						{/if}
					</Card.Description>
				{/if}
			</Card.Header>
			<Card.Content class="flex-1">
				<p class="text-sm text-muted-foreground">{truncate(r.description, 120)}</p>
			</Card.Content>
			{#if r.code_url || r.demo_url || r.archived_repo || r.archived_demo}
				<Card.Footer class="gap-2">
					{#if r.demo_url}
						<a href={r.demo_url} target="_blank" rel="noopener external">
							<Button variant="outline" size="sm">
								<ExternalLink class="mr-1 h-3 w-3" /> Demo
							</Button>
						</a>
					{/if}
					{#if r.code_url}
						<a href={r.code_url} target="_blank" rel="noopener external">
							<Button variant="outline" size="sm">
								<Code class="mr-1 h-3 w-3" /> Code
							</Button>
						</a>
					{/if}
				</Card.Footer>
			{/if}
		</Card.Card>
	{/each}
</div>
