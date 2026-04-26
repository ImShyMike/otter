<script lang="ts">
	import { onMount } from 'svelte';
	import Clock3 from '@lucide/svelte/icons/clock-3';
	import { API_BASE } from '$lib/search';

	interface ServerStatusResponse {
		last_refreshed_at: number | null;
	}

	let lastRefreshedAt = $state<number | null>(null);

	onMount(async () => {
		try {
			const res = await fetch(`${API_BASE}/api/v1/status`);
			const body = (await res.json()) as ServerStatusResponse;
			lastRefreshedAt = body.last_refreshed_at;
		} catch {
			lastRefreshedAt = null;
		}
	});

	function formatRefreshTime(ts: number): string {
		return new Date(ts * 1000).toLocaleString(undefined, {
			dateStyle: 'medium',
			timeStyle: 'short'
		});
	}
</script>

{#if lastRefreshedAt}
	<footer
		class="mt-auto w-full pt-6 text-center font-mono text-xs text-muted-foreground/65 transition-colors hover:text-foreground/80"
	>
		<div class="inline-flex items-center gap-2">
			<Clock3 class="h-3 w-3" />
			<span>last updated at {formatRefreshTime(lastRefreshedAt)}</span>
		</div>
	</footer>
{/if}
