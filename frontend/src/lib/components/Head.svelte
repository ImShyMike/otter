<script lang="ts">
	import heroImage from '../assets/hero.png';
	import { page } from '$app/state';

	let {
		title = 'Otter',
		description = 'Search engine for Hack Club projects',
		twitterCard = 'summary_large_image',
		image,
		type = 'website',
		canonical
	}: {
		title?: string;
		description?: string;
		twitterCard?: 'summary' | 'summary_large_image';
		image?: string;
		type?: 'website' | 'article';
		canonical?: string;
	} = $props();

	const canonicalUrl = $derived(
		canonical ?? (page.url ? `${page.url.origin}${page.url.pathname}` : undefined)
	);
	const ogImage = $derived(image ?? heroImage);
</script>

<svelte:head>
	<title>{title}</title>
	<meta name="description" content={description} />
	<meta name="robots" content="index,follow" />
	{#if canonicalUrl}
		<link rel="canonical" href={canonicalUrl} />
		<meta property="og:url" content={canonicalUrl} />
	{/if}
	<meta property="og:site_name" content="Otter" />
	<meta property="og:type" content={type} />
	<meta property="og:title" content={title} />
	<meta property="og:description" content={description} />
	<meta name="twitter:card" content={twitterCard} />
	<meta name="twitter:title" content={title} />
	<meta name="twitter:description" content={description} />
	<meta property="og:image" content={ogImage} />
	<meta name="twitter:image" content={ogImage} />
</svelte:head>
