<script lang="ts">
	import { Badge } from '$lib/components/ui/badge';
	import * as Table from '$lib/components/ui/table';
	import Code from '@lucide/svelte/icons/code';
	import Globe from '@lucide/svelte/icons/globe';
	import { title } from '$lib/search';
	import type { SearchResult } from '$lib/types';
	import { formatHours, formatApproved } from '$lib/utils';

	let { results }: { results: SearchResult[] } = $props();
</script>

<div class="rounded border">
	<Table.Table>
		<Table.Header>
			<Table.Row>
				<Table.Head>Name</Table.Head>
				<Table.Head>YSWS</Table.Head>
				<Table.Head>User</Table.Head>
				<Table.Head>Country</Table.Head>
				<Table.Head>Stars</Table.Head>
				<Table.Head>Hours</Table.Head>
				<Table.Head>Approved</Table.Head>
				<Table.Head>Score</Table.Head>
				<Table.Head>ID</Table.Head>
				<Table.Head>Links</Table.Head>
			</Table.Row>
		</Table.Header>
		<Table.Body>
			{#each results as r (r.id)}
				<Table.Row>
					<Table.Cell class="font-medium">{title(r)}</Table.Cell>
					<Table.Cell>
						<Badge variant="secondary">{r.ysws}</Badge>
					</Table.Cell>
					<Table.Cell>{r.github_username ? `@${r.github_username}` : '—'}</Table.Cell>
					<Table.Cell>{r.country ?? '—'}</Table.Cell>
					<Table.Cell>{r.github_stars > 0 ? r.github_stars : '—'}</Table.Cell>
					<Table.Cell>{formatHours(r)}</Table.Cell>
					<Table.Cell>{formatApproved(r.approved_at)}</Table.Cell>
					<Table.Cell>{r.score!.toFixed(3)}</Table.Cell>
					<Table.Cell class="font-mono text-xs">{r.airtable_id}</Table.Cell>
					<Table.Cell>
						<div class="flex justify-start gap-2">
							{#if r.demo_url}
								<a href={r.demo_url} target="_blank" rel="noopener external">
									<Globe class="h-4 w-4 text-muted-foreground hover:text-foreground" />
								</a>
							{/if}
							{#if r.code_url}
								<a href={r.code_url} target="_blank" rel="noopener external">
									<Code class="h-4 w-4 text-muted-foreground hover:text-foreground" />
								</a>
							{/if}
							{#if r.archived_demo}
								<a
									href={r.archived_demo}
									target="_blank"
									rel="noopener external"
									title="Archived Demo"
								>
									<Globe class="h-4 w-4 text-muted-foreground hover:text-foreground" />
								</a>
							{/if}
							{#if r.archived_repo}
								<a
									href={r.archived_repo}
									target="_blank"
									rel="noopener external"
									title="Archived Repo"
								>
									<Code class="h-4 w-4 text-muted-foreground hover:text-foreground" />
								</a>
							{/if}
						</div>
					</Table.Cell>
				</Table.Row>
			{/each}
		</Table.Body>
	</Table.Table>
</div>
