<script lang="ts">
	import { cn } from '$lib/utils.js';
	import type { HTMLImgAttributes } from 'svelte/elements';

	let imageFailed = $state(false);
	let missing = $state(false);

	let {
		src,
		alt = '',
		missing: showMissing = false,
		class: className,
		...restProps
	}: HTMLImgAttributes & { missing?: boolean } = $props();
</script>

{#if showMissing || missing}
	<div
		class={cn(
			'relative h-full w-full bg-muted object-cover text-sm text-muted-foreground',
			className
		)}
	>
		<p class="absolute inset-0 m-0 flex items-center justify-center text-center">No Image :(</p>
	</div>
{:else if imageFailed}
	<video
		{src}
		autoplay
		loop
		muted
		class={cn('object-cover', className)}
		onerror={() => (missing = true)}
	></video>
{:else}
	<img
		onerror={() => (imageFailed = true)}
		{src}
		{alt}
		class={cn('object-cover', className)}
		{...restProps}
	/>
{/if}
