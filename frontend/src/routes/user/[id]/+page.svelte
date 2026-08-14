<script lang="ts">
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import Clock3 from '@lucide/svelte/icons/clock-3';
	import Code from '@lucide/svelte/icons/code';
	import MessageCircle from '@lucide/svelte/icons/message-circle';
	import Search from '@lucide/svelte/icons/search';
	import Star from '@lucide/svelte/icons/star';
	import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
	import Users from '@lucide/svelte/icons/users';
	import CardsView from '$lib/components/CardsView.svelte';
	import Head from '$lib/components/Head.svelte';
	import ServerStatus from '$lib/components/ServerStatus.svelte';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Avatar } from '$lib/components/ui/image';
	import Spinner from '$lib/components/ui/spinner/spinner.svelte';
	import { goBack } from '$lib/back';
	import { API_BASE, USER_PROJECTS_PER_PAGE, userDisplayName, userSearchQuery } from '$lib/search';
	import type { ProjectItem, SearchResult, UserProfile } from '$lib/types';
	import { formatApproved, formatFloat } from '$lib/utils';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	/** Share of the top username's project count a username needs to be shown. */
	const USERNAME_DOMINANCE = 0.15;

	const user = $derived(data.user);
	const sharedUsername = $derived(data.sharedUsername);
	const identifier = $derived(page.params.id ?? '');
	let selectedYsws = $state<string | null>(null);

	let extraProjects = $state<ProjectItem[]>([]);
	let loadedPage = $state(1);
	let loadingMore = $state(false);
	let loadMoreError = $state<string | null>(null);
	let sentinel = $state<HTMLDivElement | null>(null);
	// do we still need to load more pages?
	let exhausted = $state(false);

	// reset the appended pages whenever the route lands on a different user
	let loadedFor = $state<string | null>(null);
	$effect(() => {
		if (loadedFor !== identifier) {
			loadedFor = identifier;
			extraProjects = [];
			loadedPage = 1;
			loadingMore = false;
			loadMoreError = null;
			exhausted = false;
			selectedYsws = null;
		}
	});

	const projects = $derived([...(user?.projects ?? []), ...extraProjects]);
	const shownProjects = $derived(
		selectedYsws === null ? projects : projects.filter((p) => p.ysws === selectedYsws)
	);
	// map all values as score = 1 since theres sno actual value
	const results = $derived(shownProjects.map((p): SearchResult => ({ ...p, score: 1 })));
	const totalProjects = $derived(Number(user?.total_projects ?? 0));
	const hasMore = $derived(!exhausted && projects.length < totalProjects);

	const name = $derived(user ? userDisplayName(user) : identifier);
	const otherAccounts = $derived(
		user ? user.slack_accounts.filter((a) => a.slack_id !== user.slack?.slack_id) : []
	);

	// keep only usernames that own a real share of the loaded projects and ignore lowercase dupes
	const githubUsernames = $derived.by(() => {
		const allowed = new Set((user?.github_usernames ?? []).map((n) => n.toLowerCase()));
		if (allowed.size === 0) return [];

		const tallies: Record<string, { count: number; spellings: Record<string, number> }> = {};
		for (const project of projects) {
			const owners = [project.github_username, project.inferred_username].filter(
				(raw): raw is string => !!raw && allowed.has(raw.toLowerCase())
			);
			const counted: string[] = [];
			for (const raw of owners) {
				const key = raw.toLowerCase();
				const tally = (tallies[key] ??= { count: 0, spellings: {} });
				if (!counted.includes(key)) {
					tally.count += 1;
					counted.push(key);
				}
				tally.spellings[raw] = (tally.spellings[raw] ?? 0) + 1;
			}
		}

		const ranked = Object.values(tallies).sort((a, b) => b.count - a.count);
		const topCount = ranked[0]?.count ?? 0;
		const names = ranked
			.filter((tally) => tally.count >= topCount * USERNAME_DOMINANCE)
			.map((tally) => Object.entries(tally.spellings).sort((a, b) => b[1] - a[1])[0][0]);

		// fallback to the first username
		return names.length > 0 ? names : (user?.github_usernames.slice(0, 1) ?? []);
	});

	async function loadMore() {
		if (!user || loadingMore || !hasMore) return;
		loadingMore = true;
		loadMoreError = null;
		try {
			const next = loadedPage + 1;
			const res = await fetch(
				`${API_BASE}/api/v1/user/${encodeURIComponent(identifier)}?limit=${USER_PROJECTS_PER_PAGE}&page=${next}`
			);
			if (!res.ok) throw new Error(`Failed with HTTP ${res.status}`);
			const body = (await res.json()) as UserProfile;
			extraProjects = [...extraProjects, ...body.projects];
			loadedPage = next;
			exhausted = body.projects.length === 0;
		} catch (error) {
			loadMoreError = error instanceof Error ? error.message : 'Could not load more projects.';
		} finally {
			loadingMore = false;
		}
	}

	$effect(() => {
		if (!sentinel) return;
		const observer = new IntersectionObserver(
			(entries) => {
				if (entries.some((e) => e.isIntersecting)) void loadMore();
			},
			{ rootMargin: '400px' }
		);
		observer.observe(sentinel);
		return () => observer.disconnect();
	});

	const pageDescription = $derived(
		user
			? `${name} has ${totalProjects} Hack Club project${totalProjects === 1 ? '' : 's'} on Otter, across ${user.ysws.length} YSWS program${user.ysws.length === 1 ? '' : 's'}.`
			: `No Otter user found for "${identifier}".`
	);
</script>

<Head
	title={user ? `${name} · Otter` : 'User not found · Otter'}
	description={pageDescription}
	image={user?.slack?.image ?? undefined}
	twitterCard="summary"
/>

<div class="mx-auto flex min-h-screen max-w-4xl flex-col px-4 py-6.5 sm:py-8">
	<div class="mb-4 flex items-center gap-4">
		<button
			onclick={goBack}
			class="flex cursor-pointer items-center gap-1 text-sm text-muted-foreground underline underline-offset-2 hover:text-foreground"
			data-umami-event="user-back"
		>
			<ArrowLeft class="h-3 w-3" /> Back
		</button>
	</div>

	{#if user}
		{@const slack = user.slack}
		<div class="flex flex-col gap-4 sm:flex-row sm:items-start">
			{#if slack}
				<Avatar slackId={slack.slack_id} src={slack.image ?? undefined} alt={name} size="hg" />
			{/if}
			<div class="flex min-w-0 flex-col gap-2">
				<div class="flex flex-wrap items-center gap-x-2 gap-y-1">
					<h1 class="text-2xl font-bold tracking-tight wrap-break-word">{name}</h1>
					{#if slack?.handle && slack.handle !== name}
						<span class="text-sm text-muted-foreground">@{slack.handle}</span>
					{/if}
				</div>

				<div class="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
					<span class="inline-flex items-center gap-1.5 whitespace-nowrap">
						{totalProjects} project{totalProjects === 1 ? '' : 's'}
					</span>
					<span class="inline-flex items-center gap-1.5 whitespace-nowrap">
						<Clock3 class="h-3 w-3 shrink-0" aria-hidden="true" />
						{formatFloat(user.total_hours, 1)}h
					</span>
					{#if Number(user.total_stars) > 0}
						<span class="inline-flex items-center gap-1.5 whitespace-nowrap">
							<Star class="h-3 w-3 shrink-0" aria-hidden="true" />
							{Number(user.total_stars)} star{Number(user.total_stars) === 1 ? '' : 's'}
						</span>
					{/if}
					{#if user.first_approved_at}
						<span class="whitespace-nowrap">
							First shipped {formatApproved(user.first_approved_at)}
						</span>
					{/if}
				</div>

				<div class="flex flex-wrap items-center gap-2">
					{#each githubUsernames as username (username)}
						<a href={`https://github.com/${username}`} target="_blank" rel="noopener external">
							<Button variant="outline" size="sm" data-umami-event="user-github">
								<Code class="mr-1 h-3 w-3" />
								{username}
							</Button>
						</a>
					{/each}
					{#if slack}
						<a
							href={`https://hackclub.slack.com/team/${slack.slack_id}`}
							target="_blank"
							rel="noopener noreferrer external nofollow"
						>
							<Button variant="outline" size="sm" data-umami-event="user-slack">
								<MessageCircle class="mr-1 h-3 w-3" /> Slack
							</Button>
						</a>
					{/if}
					<a href={resolve(`/?q=${encodeURIComponent(userSearchQuery(user))}`)}>
						<Button variant="outline" size="sm" data-umami-event="user-search">
							<Search class="mr-1 h-3 w-3" /> Search projects
						</Button>
					</a>
				</div>
			</div>
		</div>

		{#if user.ysws.length > 0}
			<div class="mt-6 flex flex-wrap items-center gap-2">
				{#each user.ysws as program (program)}
					{@const selected = selectedYsws === program}
					<button
						class="cursor-pointer"
						onclick={() => (selectedYsws = selected ? null : program)}
						aria-pressed={selected}
						data-umami-event="user-ysws-filter"
						data-umami-event-ysws={program}
					>
						<Badge variant={selected ? 'default' : 'secondary'} class="cursor-pointer text-xs">
							{program}
						</Badge>
					</button>
				{/each}
			</div>
			{#if selectedYsws !== null}
				<p class="mt-2 text-xs text-muted-foreground">
					Showing {shownProjects.length} of {projects.length} loaded project{projects.length === 1
						? ''
						: 's'} in
					<span class="font-medium text-foreground/80">{selectedYsws}</span>
					·
					<button
						class="cursor-pointer underline hover:text-foreground"
						onclick={() => (selectedYsws = null)}
						data-umami-event="user-ysws-clear">Clear</button
					>
				</p>
			{/if}
		{/if}

		{#if sharedUsername}
			<div
				class="mt-6 flex gap-2 rounded-md border border-border bg-muted/40 px-4 py-3 text-xs text-muted-foreground"
			>
				<Users class="mt-0.5 h-3.5 w-3.5 shrink-0" aria-hidden="true" />
				<p>
					These projects are also listed under
					<a
						href={resolve('/user/[id]', { id: sharedUsername.identifier })}
						class="font-medium text-foreground/80 underline underline-offset-2 hover:text-foreground"
						data-umami-event="user-shared-username">@{sharedUsername.identifier}</a
					>, a username shared by {sharedUsername.totalSlackAccounts} Slack accounts ({sharedUsername.totalProjects}
					projects in total).
				</p>
			</div>
		{/if}

		{#if user.ambiguous}
			<div
				class="mt-6 flex gap-2 rounded-md border border-border bg-muted/40 px-4 py-3 text-xs text-muted-foreground"
			>
				<TriangleAlert class="mt-0.5 h-3.5 w-3.5 shrink-0" aria-hidden="true" />
				<div class="flex flex-col gap-2">
					<p>
						<span class="font-medium text-foreground/80">"{user.identifier}"</span>
						is linked to {Number(user.total_slack_accounts)} Slack accounts, so some projects below may
						belong to someone else. Open a Slack account to see only their projects.
					</p>
					{#if otherAccounts.length > 0}
						<div class="flex flex-wrap gap-2">
							{#each user.slack_accounts as account (account.slack_id)}
								<a
									href={resolve('/user/[id]', { id: account.slack_id })}
									class="underline underline-offset-2 hover:text-foreground"
								>
									{account.display_name ?? account.real_name ?? account.handle ?? account.slack_id}
									({Number(account.project_count)})
								</a>
							{/each}
						</div>
					{/if}
				</div>
			</div>
		{/if}

		<div class="mt-8">
			{#if results.length > 0}
				<CardsView {results} />
			{:else if hasMore}
				<p class="py-12 text-center text-sm text-muted-foreground">
					Loading projects{selectedYsws === null ? '' : ` in ${selectedYsws}`}…
				</p>
			{:else}
				<p class="py-12 text-center text-muted-foreground">No projects to show.</p>
			{/if}
		</div>

		{#if loadMoreError}
			<div
				role="alert"
				class="mt-6 rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive"
			>
				<p class="font-medium">Could not load more projects</p>
				<p class="mt-1 wrap-break-word">{loadMoreError}</p>
				<button
					class="mt-2 cursor-pointer underline hover:text-foreground"
					onclick={() => void loadMore()}
					data-umami-event="user-load-more-retry"
				>
					Try again
				</button>
			</div>
		{:else if hasMore}
			<div bind:this={sentinel} class="mt-6 flex items-center justify-center py-4">
				{#if loadingMore}
					<Spinner /><span class="ml-2 text-sm text-muted-foreground">Loading more…</span>
				{/if}
			</div>
		{/if}
	{:else}
		<div class="flex flex-1 flex-col items-center justify-center gap-3">
			<p class="text-sm text-muted-foreground">
				No user found for "{identifier}".
			</p>
			<p class="max-w-md text-center text-xs text-muted-foreground">
				User pages work with a Slack ID, a GitHub username, or the username inferred from a
				project's code URL.
			</p>
		</div>
	{/if}

	<ServerStatus />
</div>
