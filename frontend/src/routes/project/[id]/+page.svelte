<script lang="ts">
	import { Spinner } from '$lib/components/ui/spinner';
	import { API_BASE, imageUrl, title, truncate } from '$lib/search';
	import type { SearchResult } from '$lib/types';
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import * as Card from '$lib/components/ui/card';
	import { ExpandableImage } from '$lib/components/ui/image';
	import { Badge } from '$lib/components/ui/badge';
	import { formatApproved, formatHours } from '$lib/utils';
	import Star from '@lucide/svelte/icons/star';
	import { Button } from '$lib/components/ui/button';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import Code from '@lucide/svelte/icons/code';
	import { resolve } from '$app/paths';

	let project = $state<SearchResult | null>(null);
	let loading = $state(false);

	onMount(async () => {
		loading = true;
		try {
			const res = await fetch(`${API_BASE}/api/project/${page.params.id}`);
			project = await res.json();
		} catch {
			project = null;
		} finally {
			loading = false;
		}
	});
</script>

<svelte:head>
	<title>{project ? title(project) : 'Project'} · Otter</title>
</svelte:head>

<div class="mx-auto flex min-h-screen max-w-4xl flex-col px-4 py-8">
	<div class="mb-8 flex flex-row items-start text-center">
		<a
			href={resolve('/')}
			class="mb-4 flex items-center justify-center gap-1 text-sm text-muted-foreground underline underline-offset-2 hover:text-foreground"
		>
			<ArrowLeft class="h-3 w-3" /> Back to search
		</a>
	</div>
	{#if loading}
		<div class="flex flex-1 items-center justify-center">
			<Spinner />
		</div>
	{:else if project}
		{@const p = project}
		<Card.Card class="flex flex-col">
			<ExpandableImage
				id={p.id}
				src={imageUrl(p.airtable_id)}
				alt={title(p)}
				missing={!p.has_media}
				buttonClass="w-full"
				thumbnailClass="h-60 w-full border-b bg-muted object-cover"
				transitionPrefix="cards-image"
			/>
			<Card.Header>
				<div class="flex flex-wrap items-center gap-2">
					<Card.Title class="text-base">{title(p)}</Card.Title>
					<Badge variant="secondary" class="text-xs">{p.ysws}</Badge>
					{#if p.github_stars > 0}
						<Badge variant="outline" class="text-xs">{p.github_stars} <Star /></Badge>
					{/if}
					{#if formatHours(p)}
						<Badge variant="outline" class="text-xs">{formatHours(p)}</Badge>
					{/if}
				</div>
				{#if p.country || p.github_username || formatApproved(p.approved_at)}
					<Card.Description>
						{p.country ?? ''}
						{#if p.country && p.github_username}
							·
						{/if}
						{#if p.github_username}
							<a
								class="text-xs text-muted-foreground underline underline-offset-2 hover:text-foreground"
								href={`https://github.com/${p.github_username}`}
								target="_blank"
								rel="noopener external">@{p.github_username}</a
							>
						{/if}
						{#if (p.country || p.github_username) && formatApproved(p.approved_at)}
							·
						{/if}
						{#if formatApproved(p.approved_at)}
							Approved {formatApproved(p.approved_at)}
						{/if}
					</Card.Description>
				{/if}
			</Card.Header>
			<Card.Content class="flex-1">
				<p class="text-sm text-muted-foreground">{truncate(p.description, 120)}</p>
			</Card.Content>
			{#if p.code_url || p.demo_url || p.archived_repo || p.archived_demo}
				<Card.Footer class="gap-2">
					{#if p.demo_url}
						<a href={p.demo_url} target="_blank" rel="noopener external">
							<Button variant="outline" size="sm">
								<ExternalLink class="mr-1 h-3 w-3" /> Demo
							</Button>
						</a>
					{/if}
					{#if p.code_url}
						<a href={p.code_url} target="_blank" rel="noopener external">
							<Button variant="outline" size="sm">
								<Code class="mr-1 h-3 w-3" /> Code
							</Button>
						</a>
					{/if}
				</Card.Footer>
			{/if}
		</Card.Card>
	{/if}
</div>
