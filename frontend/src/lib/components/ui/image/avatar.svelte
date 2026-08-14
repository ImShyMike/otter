<script lang="ts">
	import { cn } from '$lib/utils.js';
	import type { HTMLImgAttributes } from 'svelte/elements';

	const sizeMap = {
		sm: 'h-6 w-6',
		md: 'h-8 w-8',
		lg: 'h-12 w-12',
		hg: 'h-24 w-24'
	};

	let {
		slackId,
		alt = 'Avatar',
		size = 'md',
		href,
		src: srcOverride,
		class: className,
		...restProps
	}: Omit<HTMLImgAttributes, 'src'> & {
		slackId: string;
		size?: keyof typeof sizeMap;
		/** Internal link to use instead of the Slack profile. */
		href?: string;
		src?: string;
	} = $props();

	let loaded = $state(false);

	const src = $derived(srcOverride ?? `https://cachet.dunkirk.sh/users/${slackId}/r`);
	const external = $derived(href === undefined);
	const resolvedHref = $derived(href ?? `https://hackclub.slack.com/team/${slackId}`);
</script>

<!-- eslint-disable svelte/no-navigation-without-resolve -->
<a
	href={resolvedHref}
	target={external ? '_blank' : undefined}
	rel={external ? 'noopener noreferrer external nofollow' : undefined}
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
