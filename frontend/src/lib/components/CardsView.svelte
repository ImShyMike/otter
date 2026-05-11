<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import * as Card from '$lib/components/ui/card';
	import { Avatar, ExpandableImage } from '$lib/components/ui/image';
	import Code from '@lucide/svelte/icons/code';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import Star from '@lucide/svelte/icons/star';
	import { scoreClass, title, truncate } from '$lib/search';
	import type { SearchResult } from '$lib/types';
	import { formatHours, formatApproved, formatFloat, cn } from '$lib/utils';
	import { resolve } from '$app/paths';

	let { results }: { results: SearchResult[] } = $props();
</script>

<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
	{#each results as r (r.airtable_id)}
		<Card.Card class="flex flex-col">
			<ExpandableImage
				id={r.id}
				airtableId={r.airtable_id}
				blurhash={r.preview_blurhash ?? undefined}
				alt={title(r)}
				missing={!r.has_media}
				buttonClass="w-full"
				thumbnailClass="h-60 w-full border-b bg-muted object-cover"
				transitionPrefix="cards-image"
			/>
			<Card.Header>
				<div class="flex flex-wrap items-center gap-2">
					<Card.Title class="text-base"
						><a
							href={resolve('/project/[id]', { id: r.airtable_id })}
							class="hover:text-foreground"
							data-umami-event="card-result-click"
							data-umami-event-project={r.airtable_id}>{title(r)}</a
						></Card.Title
					>
					<Badge variant="secondary" class="text-xs">{r.ysws}</Badge>
					{#if r.github_stars > 0}
						<Badge variant="outline" class="text-xs">{r.github_stars} <Star /></Badge>
					{/if}
					{#if formatHours(r)}
						<Badge variant="outline" class="text-xs">{formatHours(r)}</Badge>
					{/if}
					{#if r.inferred_username}
						<div class="flex items-center gap-1">
							{#if r.slack_id}
								<Avatar slackId={r.slack_id} alt={r.inferred_username} class="ml-1" size="sm" />
							{/if}
							<a
								class="text-xs text-muted-foreground underline underline-offset-2 hover:text-foreground"
								href={`https://github.com/${r.inferred_username}`}
								target="_blank"
								rel="noopener external">@{r.inferred_username}</a
							>
						</div>
					{/if}
				</div>
				{#if r.country || formatApproved(r.approved_at) || (r.score !== null && r.score <= 1)}
					<Card.Description
						class="mt-1 flex flex-wrap items-center gap-2 text-xs text-muted-foreground"
					>
						{r.country ?? ''}
						{#if r.country && formatApproved(r.approved_at)}
							·
						{/if}
						{#if formatApproved(r.approved_at)}
							Approved {formatApproved(r.approved_at)}
						{/if}
						{#if r.country && formatApproved(r.approved_at) && r.score !== null && r.score <= 1}
							·
						{/if}
						{#if r.score !== null && r.score <= 1}
							<span class={cn('text-xs', scoreClass(r.score))} title="Search score"
								>Score {formatFloat(r.score * 100, 1)}%</span
							>
						{/if}
					</Card.Description>
				{/if}
			</Card.Header>
			<Card.Content class="flex-1">
				<p class="text-sm wrap-break-word text-muted-foreground">{truncate(r.description, 120)}</p>
			</Card.Content>
			{#if r.code_url || r.demo_url || r.archived_repo || r.archived_demo}
				<Card.Footer class="gap-2">
					{#if r.demo_url}
						<a
							href={r.demo_url}
							target="_blank"
							rel="noopener external"
							data-umami-event="card-demo-click"
							data-umami-event-url={r.demo_url}
						>
							<Button variant="outline" size="sm">
								<ExternalLink class="mr-1 h-3 w-3" /> Demo
							</Button>
						</a>
					{/if}
					{#if r.code_url}
						<a
							href={r.code_url}
							target="_blank"
							rel="noopener external"
							data-umami-event="card-code-click"
							data-umami-event-url={r.code_url}
						>
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
