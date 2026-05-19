<script lang="ts">
	import { cn } from '$lib/utils.js';
	import type { HTMLImgAttributes } from 'svelte/elements';

	const sizeMap = {
		sm: 'h-6 w-6',
		md: 'h-8 w-8',
		lg: 'h-12 w-12'
	};

	let {
		slackId,
		alt = 'Avatar',
		size = 'md',
		class: className,
		...restProps
	}: Omit<HTMLImgAttributes, 'src'> & { slackId: string; size?: keyof typeof sizeMap } = $props();

	let loaded = $state(false);

	const src = $derived(`https://cachet.dunkirk.sh/users/${slackId}/r`);
	const href = $derived(`https://hackclub.slack.com/team/${slackId}`);
</script>

<a
	{href}
	target="_blank"
	rel="noopener noreferrer external nofollow"
	class={cn(
		'relative inline-block aspect-square shrink-0 overflow-hidden rounded-[9px] bg-muted',
		className,
		sizeMap[size] ?? sizeMap['md']
	)}
>
	{#if !loaded}
		<div class="absolute inset-0 animate-pulse bg-muted-foreground/20"></div>
	{/if}
	<img
		{src}
		{alt}
		decoding="async"
		loading="eager"
		onload={() => (loaded = true)}
		class={cn(
			'h-full w-full rounded-[9px] object-cover transition-opacity duration-200',
			loaded ? 'opacity-100' : 'opacity-0'
		)}
		{...restProps}
	/>
</a>
