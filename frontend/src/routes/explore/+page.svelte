<script lang="ts">
	import DataTable from '$lib/components/DataTable.svelte';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import Head from '$lib/components/Head.svelte';
	import { goBack } from '$lib/stores/back';
</script>

<Head title="Explore · Otter" description="Explore Hack Club projects" />

<div class="mx-auto flex max-w-7xl flex-col px-4 py-6 sm:py-8">
	<div class="mb-4 flex items-center gap-4">
		<button
			class="flex cursor-pointer items-center gap-1 text-sm text-muted-foreground underline underline-offset-2 hover:text-foreground"
			onclick={goBack}
		>
			<ArrowLeft class="h-3 w-3" /> Back
		</button>
		<h1 class="text-xl font-bold tracking-tight">Explore</h1>
	</div>

	<DataTable />
</div>
