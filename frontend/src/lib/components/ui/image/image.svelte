<script lang="ts">
	import { decode } from 'blurhash';
	import { cn } from '$lib/utils.js';
	import type { HTMLImgAttributes } from 'svelte/elements';

	let imageFailed = $state(false);
	let missing = $state(false);
	let highLoaded = $state(false);
	let blurhashSrc = $state<string | undefined>(undefined);

	function decodeBlurhash(blurhash: string): string | undefined {
		try {
			const width = 32;
			const height = 32;
			const pixels = decode(blurhash, width, height);
			const canvas = document.createElement('canvas');
			canvas.width = width;
			canvas.height = height;
			const context = canvas.getContext('2d');
			if (!context) return undefined;
			const imageData = context.createImageData(width, height);
			imageData.data.set(pixels);
			context.putImageData(imageData, 0, 0);
			return canvas.toDataURL();
		} catch {
			return undefined;
		}
	}

	let {
		src,
		blurhash,
		alt = '',
		missing: showMissing = false,
		class: className,
		...restProps
	}: HTMLImgAttributes & { missing?: boolean; blurhash?: string } = $props();

	$effect(() => {
		if (!blurhash || typeof window === 'undefined') {
			blurhashSrc = undefined;
			return;
		}

		blurhashSrc = decodeBlurhash(blurhash);
	});

	$effect(() => {
		if (!src) {
			highLoaded = true;
			return;
		}

		highLoaded = false;
		if (typeof window === 'undefined') return;
		const preloader = new window.Image();
		const done = () => (highLoaded = true);
		preloader.onload = done;
		preloader.onerror = done;
		preloader.src = src;
		return () => {
			preloader.onload = null;
			preloader.onerror = null;
		};
	});

	const showBlurhash = $derived(!!blurhashSrc && !!src && !highLoaded);
	const currentSrc = $derived(showBlurhash ? blurhashSrc : (src ?? blurhashSrc));
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
		onerror={() => {
			if (src) imageFailed = true;
		}}
		src={currentSrc}
		{alt}
		decoding="async"
		class={cn('object-cover', className)}
		{...restProps}
	/>
{/if}
