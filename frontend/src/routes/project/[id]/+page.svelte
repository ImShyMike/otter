<script lang="ts">
	import { imageUrl, title, truncate } from '$lib/search';
	import { marked } from 'marked';
	import xss from 'xss';
	import type { PageData } from './$types';
	import * as Card from '$lib/components/ui/card';
	import { ExpandableImage, Avatar } from '$lib/components/ui/image';
	import { Badge } from '$lib/components/ui/badge';
	import { formatApproved, formatHours } from '$lib/utils';
	import Star from '@lucide/svelte/icons/star';
	import { Button } from '$lib/components/ui/button';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import Code from '@lucide/svelte/icons/code';
	import Head from '$lib/components/Head.svelte';
	import { goBack } from '$lib/back';
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
	title={project ? title(project) : 'Project'}
	description={project
		? truncate(project?.description ?? 'A Hack Club project', 200)
		: 'Project not found'}
	twitterCard={project?.has_media ? 'summary_large_image' : 'summary'}
	image={project?.has_media ? imageUrl(project.airtable_id) : undefined}
/>

<div class="mx-auto flex min-h-screen max-w-4xl flex-col px-4 py-6.5 sm:py-8">
	<div class="mb-4 flex flex-row items-center justify-between text-center">
		<button
			onclick={goBack}
			class="flex cursor-pointer items-center justify-center gap-1 text-sm text-muted-foreground underline underline-offset-2 hover:text-foreground"
			data-umami-event="project-back"
		>
			<ArrowLeft class="h-3 w-3" /> Back
		</button>
		<div class="pr-13 sm:pr-0">
			{#if shareStatus === 'copied'}
				<span class="text-xs text-muted-foreground">Copied link!</span>
			{:else if shareStatus === 'failed'}
				<span class="text-xs text-muted-foreground">Copy failed</span>
			{/if}
			<Button
				variant="outline"
				size="sm"
				onclick={copyShareLink}
				class="ml-auto"
				data-umami-event="project-share"
				data-umami-event-project={project?.airtable_id ?? ''}
			>
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
					airtableId={p.airtable_id}
					blurhash={p.preview_blurhash ?? undefined}
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
					{#if p.slack_id}
						<Avatar
							slackId={p.slack_id}
							alt={p.inferred_username ?? p.github_username ?? ''}
							class="h-6 w-6"
							size="sm"
						/>
					{/if}
					<Card.Title class="text-base">{title(p)}</Card.Title>
					{#if p.github_stars > 0}
						<Badge variant="outline" class="text-xs">{p.github_stars} <Star /></Badge>
					{/if}
					{#if p.inferred_username ?? p.github_username}
						<a
							class="text-xs text-muted-foreground underline underline-offset-2 hover:text-foreground"
							href={`https://github.com/${p.inferred_username ?? p.github_username}`}
							target="_blank"
							rel="noopener external">@{p.inferred_username ?? p.github_username}</a
						>
					{/if}
				</div>
				<Card.Description
					class="mt-1 flex flex-wrap items-center gap-2 text-xs text-muted-foreground"
				>
					<Badge variant="secondary" class="text-xs">{p.ysws}</Badge>
					{#if formatHours(p)}
						<Badge variant="outline" class="text-xs">{formatHours(p)}</Badge>
					{/if}
					{#if p.country}
						<Badge variant="outline" class="text-xs">{p.country}</Badge>
					{/if}
					{#if formatApproved(p.approved_at)}
						<span>Approved {formatApproved(p.approved_at)}</span>
					{/if}
				</Card.Description>
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
						<a
							href={p.demo_url}
							target="_blank"
							rel="noopener external"
							data-umami-event="project-demo"
							data-umami-event-url={p.demo_url}
						>
							<Button variant="outline" size="sm">
								<ExternalLink class="mr-1 h-3 w-3" /> Demo
							</Button>
						</a>
					{/if}
					{#if p.code_url}
						<a
							href={p.code_url}
							target="_blank"
							rel="noopener external"
							data-umami-event="project-code"
							data-umami-event-url={p.code_url}
						>
							<Button variant="outline" size="sm">
								<Code class="mr-1 h-3 w-3" /> Code
							</Button>
						</a>
					{/if}
					{#if p.archived_demo}
						<a
							href={p.archived_demo}
							target="_blank"
							rel="noopener external"
							data-umami-event="project-archived-demo"
							data-umami-event-url={p.archived_demo}
						>
							<Button variant="outline" size="sm">
								<ExternalLink class="mr-1 h-3 w-3" /> Archived Demo
							</Button>
						</a>
					{/if}
					{#if p.archived_repo}
						<a
							href={p.archived_repo}
							target="_blank"
							rel="noopener external"
							data-umami-event="project-archived-code"
							data-umami-event-url={p.archived_repo}
						>
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
