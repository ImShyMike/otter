<script lang="ts">
	import X from '@lucide/svelte/icons/x';
	import { Image } from '$lib/components/ui/image';
	import { cn } from '$lib/utils.js';
	import { imageUrl } from '$lib/search';

	type DocumentWithViewTransition = Document & {
		startViewTransition?: (updateCallback: () => void) => { finished: Promise<void> };
	};

	let {
		id,
		src,
		airtableId,
		blurhash,
		alt = '',
		missing = false,
		loading = 'lazy',
		buttonClass,
		thumbnailClass,
		expandedClass,
		transitionPrefix = 'expandable-image',
		title = 'Click to expand',
		expandedAlt = 'Expanded view'
	}: {
		id: number | string;
		src?: string;
		airtableId?: string;
		blurhash?: string;
		alt?: string;
		missing?: boolean;
		loading?: 'lazy' | 'eager';
		buttonClass?: string;
		thumbnailClass?: string;
		expandedClass?: string;
		transitionPrefix?: string;
		title?: string;
		expandedAlt?: string;
	} = $props();

	let expanded = $state(false);
	let transitioning = $state(false);
	let preloadedFullSrc = $state<string | null>(null);
	let fullImageReady = $state(false);

	const effectiveMissing = $derived(missing);

	const thumbSrc = $derived.by(() => {
		if (airtableId) return imageUrl(airtableId, 'large');
		return src;
	});

	const fullSrc = $derived.by(() => {
		if (airtableId) return imageUrl(airtableId);
		return src;
	});

	$effect(() => {
		if (preloadedFullSrc && preloadedFullSrc !== fullSrc) {
			preloadedFullSrc = null;
			fullImageReady = false;
		}
	});

	function preloadFullImage() {
		const srcToPreload = fullSrc;
		if (!srcToPreload || preloadedFullSrc === srcToPreload || typeof window === 'undefined') {
			return;
		}

		const preloader = new window.Image();
		preloader.onload = () => {
			fullImageReady = true;
		};
		preloader.onerror = () => {
			fullImageReady = false;
		};
		preloader.src = srcToPreload;
		preloadedFullSrc = srcToPreload;
	}

	function runWithViewTransition(update: () => void, onFinish?: () => void) {
		const doc = document as DocumentWithViewTransition;

		if (doc.startViewTransition) {
			const transition = doc.startViewTransition(update);

			if (onFinish) {
				transition.finished.finally(onFinish);
			}

			return;
		}

		update();
		onFinish?.();
	}

	function transitionName() {
		if (expanded) {
			return 'none';
		}

		return transitioning ? `${transitionPrefix}-${id}` : 'none';
	}

	function open() {
		if (effectiveMissing) {
			return;
		}

		preloadFullImage();

		transitioning = true;

		runWithViewTransition(
			() => {
				expanded = true;
			},
			() => {
				transitioning = false;
			}
		);
	}

	function close() {
		if (!expanded) {
			return;
		}

		transitioning = true;

		runWithViewTransition(
			() => {
				expanded = false;
			},
			() => {
				transitioning = false;
			}
		);
	}
</script>

<button
	onclick={open}
	onpointerenter={preloadFullImage}
	onfocus={preloadFullImage}
	ontouchstart={preloadFullImage}
	class={cn('cursor-pointer transition-opacity hover:opacity-80', buttonClass)}
	type="button"
	{title}
	disabled={effectiveMissing}
	data-umami-event="image-expand"
>
	<Image
		src={thumbSrc}
		{blurhash}
		{alt}
		missing={effectiveMissing}
		class={cn('h-24 w-36 shrink-0 bg-muted object-cover', thumbnailClass)}
		style={`view-transition-name: ${transitionName()}`}
		{loading}
	/>
</button>

{#if expanded}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 backdrop-blur-sm"
		onclick={(e) => e.target === e.currentTarget && close()}
		role="dialog"
		aria-modal="true"
		tabindex="-1"
		onkeydown={(e) => e.key === 'Escape' && close()}
	>
		<button
			onclick={close}
			class="absolute top-4 right-4 z-100 cursor-pointer text-white transition-colors hover:text-gray-300"
			type="button"
			title="Close"
			aria-label="Close image"
		>
			<X class="h-8 w-8" />
		</button>
		<Image
			src={transitioning || !fullImageReady ? (thumbSrc ?? fullSrc) : fullSrc}
			alt={transitioning || !fullImageReady ? alt : expandedAlt}
			loading="eager"
			class={cn('h-[90vh] object-contain', expandedClass)}
			style={`view-transition-name: ${transitionPrefix}-${id}`}
		/>
	</div>
{/if}

<style>
	:global(:root) {
		view-transition-name: none;
	}

	:global(::view-transition-old(root)),
	:global(::view-transition-new(root)) {
		animation: none;
	}

	:global(::view-transition-group(*)) {
		animation-duration: 220ms;
		animation-timing-function: cubic-bezier(0.76, 0, 0.24, 1);
	}
</style>
