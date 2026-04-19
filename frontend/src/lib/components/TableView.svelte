<script lang="ts">
	import { Badge } from '$lib/components/ui/badge';
	import * as Table from '$lib/components/ui/table';
	import Code from '@lucide/svelte/icons/code';
	import Globe from '@lucide/svelte/icons/globe';
	import { title } from '$lib/search';
	import type { SearchResult } from '$lib/types';

	let { results }: { results: SearchResult[] } = $props();
</script>

<div class="rounded border">
	<Table.Table>
		<Table.Header>
			<Table.Row>
				<Table.Head>Name</Table.Head>
				<Table.Head>YSWS</Table.Head>
				<Table.Head>Country</Table.Head>
				<Table.Head>Score</Table.Head>
				<Table.Head class="text-right">Links</Table.Head>
			</Table.Row>
		</Table.Header>
		<Table.Body>
			{#each results as r (r.id)}
				<Table.Row>
					<Table.Cell class="font-medium">{title(r)}</Table.Cell>
					<Table.Cell>
						<Badge variant="secondary">{r.ysws}</Badge>
					</Table.Cell>
					<Table.Cell>{r.country ?? '—'}</Table.Cell>
					<Table.Cell>{r.score.toFixed(3)}</Table.Cell>
					<Table.Cell class="text-right">
						<div class="flex justify-end gap-2">
							{#if r.code_url}
								<a href={r.code_url} target="_blank" rel="noopener external">
									<Code class="h-4 w-4 text-muted-foreground hover:text-foreground" />
								</a>
							{/if}
							{#if r.demo_url}
								<a href={r.demo_url} target="_blank" rel="noopener external">
									<Globe class="h-4 w-4 text-muted-foreground hover:text-foreground" />
								</a>
							{/if}
						</div>
					</Table.Cell>
				</Table.Row>
			{/each}
		</Table.Body>
	</Table.Table>
</div>
