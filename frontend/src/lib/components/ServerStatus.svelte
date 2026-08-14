<script lang="ts">
	import { onMount } from 'svelte';
	import Clock3 from '@lucide/svelte/icons/clock-3';
	import ChartColumnBig from '@lucide/svelte/icons/chart-column-big';
	import { API_BASE } from '$lib/search';
	import type { ServerStatus } from '$lib/types';

	let lastRefreshedAt = $state<number | null>(null);
	let totalProjects = $state<number | null>(null);

	onMount(async () => {
		try {
			const res = await fetch(`${API_BASE}/api/v1/status`);
			const body = (await res.json()) as ServerStatus;
			const refreshed = Number(body.last_refreshed_at);
			const projects = Number(body.total_projects);
			lastRefreshedAt = Number.isFinite(refreshed) ? refreshed : null;
			totalProjects = Number.isFinite(projects) ? projects : null;
		} catch {
			lastRefreshedAt = null;
			totalProjects = null;
		}
	});

	function formatRefreshTime(ts: number | null): string {
		if (ts === null || !Number.isFinite(ts)) return 'unknown';
		return new Date(ts * 1000).toLocaleString(undefined, {
			dateStyle: 'medium',
			timeStyle: 'short'
		});
	}
</script>

<footer
	class="mt-auto w-full pt-6 text-center font-mono text-xs text-muted-foreground/65 transition-colors hover:text-foreground/80"
>
	<p class="mb-1.5">projects may take a few days to get added here even after being submitted</p>
	<div class="flex flex-wrap items-center justify-center gap-x-3 gap-y-1">
		<span class="inline-flex items-center gap-1.5 whitespace-nowrap">
			<Clock3 class="h-3 w-3 shrink-0" aria-hidden="true" />
			last updated at {formatRefreshTime(lastRefreshedAt)}
		</span>
		<span class="inline-flex items-center gap-1.5 whitespace-nowrap">
			<ChartColumnBig class="h-3 w-3 shrink-0" aria-hidden="true" />
			total projects: {totalProjects ?? 'unknown'}
		</span>
	</div>
</footer>
