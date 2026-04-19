<script lang="ts">
	import { cn } from '$lib/utils.js';
	import type { HTMLImgAttributes } from 'svelte/elements';

	export function tryVideoOnError(e: Event) {
		const image = e.currentTarget as HTMLImageElement;
		const video = document.createElement('video');
		video.src = image.src;
		video.autoplay = true;
		video.loop = true;
		video.muted = true;
		video.className = image.className;
		video.onerror = () => {
			// if video also fails, hide the element
			video.style.display = 'none';
		};
		image.replaceWith(video);
	}

	let {
		src,
		alt = '',
		class: className,
		...restProps
	}: HTMLImgAttributes = $props();
</script>

<img
	onerror={tryVideoOnError}
	src={src}
	alt={alt}
	class={cn('rounded-md border object-cover', className)}
	{...restProps}
/>
