<script lang="ts">
	import DataTable from '$lib/components/DataTable.svelte';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import { resolve } from '$app/paths';
</script>

<svelte:head>
	<title>Explore · Otter</title>
	<meta property="og:title" content="Explore · Otter" />
</svelte:head>

<div class="mx-auto flex max-w-7xl flex-col px-4 py-8">
	<div class="mb-6 flex items-center gap-4">
		<a
			href={resolve('/')}
			class="flex cursor-pointer items-center gap-1 text-sm text-muted-foreground underline underline-offset-2 hover:text-foreground"
		>
			<ArrowLeft class="h-3 w-3" /> Back to search
		</a>
		<h1 class="text-xl font-bold tracking-tight">Explore</h1>
	</div>

	<DataTable />
</div>
