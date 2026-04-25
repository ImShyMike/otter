<script lang="ts">
	import { imageUrl, title, truncate } from '$lib/search';
	import { marked } from 'marked';
	import xss from 'xss';
	import type { PageData } from './$types';
	import * as Card from '$lib/components/ui/card';
	import { ExpandableImage } from '$lib/components/ui/image';
	import { Badge } from '$lib/components/ui/badge';
	import { formatApproved, formatHours } from '$lib/utils';
	import Star from '@lucide/svelte/icons/star';
	import { Button } from '$lib/components/ui/button';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import Code from '@lucide/svelte/icons/code';
	import Head from '$lib/components/Head.svelte';
	import { goBack } from '$lib/stores/back';
	import Share2 from '@lucide/svelte/icons/share-2';

	let shareStatus = $state<'idle' | 'copied' | 'failed'>('idle');

	function renderDescription(description: string | null): string {
		const markdownHtml = marked.parse(description ?? '', {
			async: false,
			gfm: true,
			breaks: true
		});

		return xss(markdownHtml, {
			stripIgnoreTag: true,
			stripIgnoreTagBody: ['script', 'style']
		});
	}

	async function copyShareLink() {
		if (typeof window === 'undefined') return;

		const url = new URL(window.location.href);

		try {
			await navigator.clipboard.writeText(url.toString());
			shareStatus = 'copied';
		} catch {
			shareStatus = 'failed';
		}

		setTimeout(() => {
			shareStatus = 'idle';
		}, 2000);
	}

	let { data }: { data: PageData } = $props();
	const project = $derived(data.project);
</script>

<Head
	title={(project ? title(project) : 'Project') + ' · Otter'}
	description={project
		? truncate(project?.description ?? 'A Hack Club project', 200)
		: 'Project not found'}
	twitterCard={project?.has_media ? 'summary_large_image' : 'summary'}
	image={project?.has_media ? imageUrl(project.airtable_id) : undefined}
/>

<div class="mx-auto flex min-h-screen max-w-4xl flex-col px-4 py-6 sm:py-8">
	<div class="mb-4 flex flex-row items-center justify-between text-center">
		<button
			onclick={goBack}
			class="flex cursor-pointer items-center justify-center gap-1 text-sm text-muted-foreground underline underline-offset-2 hover:text-foreground"
		>
			<ArrowLeft class="h-3 w-3" /> Back
		</button>
		<div>
			{#if shareStatus === 'copied'}
				<span class="text-xs text-muted-foreground">Copied link!</span>
			{:else if shareStatus === 'failed'}
				<span class="text-xs text-muted-foreground">Copy failed</span>
			{/if}
			<Button variant="outline" size="sm" onclick={copyShareLink} class="ml-auto">
				<Share2 class="mr-1 h-3 w-3" /> Share
			</Button>
		</div>
	</div>
	{#if project}
		{@const p = project}
		<Card.Card class="flex flex-col">
			<div class="aspect-video bg-muted">
				<ExpandableImage
					id={p.id}
					src={imageUrl(p.airtable_id)}
					alt={title(p)}
					missing={!p.has_media}
					loading="eager"
					buttonClass="h-full w-full"
					thumbnailClass="h-full w-full border-b bg-muted object-contain"
					transitionPrefix="cards-image"
				/>
			</div>
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
				<div class="prose prose-sm max-w-none text-muted-foreground dark:prose-invert">
					<!-- eslint-disable-next-line svelte/no-at-html-tags -->
					{@html renderDescription(p.description)}
				</div>
			</Card.Content>
			{#if p.code_url || p.demo_url || p.archived_repo || p.archived_demo}
				<Card.Footer class="flex flex-row flex-wrap gap-2">
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
					{#if p.archived_demo}
						<a href={p.demo_url} target="_blank" rel="noopener external">
							<Button variant="outline" size="sm">
								<ExternalLink class="mr-1 h-3 w-3" /> Archived Demo
							</Button>
						</a>
					{/if}
					{#if p.archived_repo}
						<a href={p.code_url} target="_blank" rel="noopener external">
							<Button variant="outline" size="sm">
								<Code class="mr-1 h-3 w-3" /> Archived Code
							</Button>
						</a>
					{/if}
				</Card.Footer>
			{/if}
		</Card.Card>
	{:else}
		<div class="flex flex-1 items-center justify-center">
			<div class="text-sm text-muted-foreground">Project not found.</div>
		</div>
	{/if}
</div>
