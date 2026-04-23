<script lang="ts">
	import favicon from '$lib/assets/favicon.svg';

	let {
		title = 'Otter',
		description = 'Search engine for Hack Club projects',
		twitterCard = 'summary',
		image,
		type = 'website'
	}: {
		title?: string;
		description?: string;
		twitterCard?: 'summary' | 'summary_large_image';
		image?: string;
		type?: 'website' | 'article';
	} = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<title>{title}</title>
	<meta name="description" content={description} />
	<meta property="og:site_name" content="Otter" />
	<meta property="og:type" content={type} />
	<meta property="og:title" content={title} />
	<meta property="og:description" content={description} />
	<meta name="twitter:card" content={twitterCard} />
	<meta name="twitter:title" content={title} />
	<meta name="twitter:description" content={description} />
	{#if image}
		<meta property="og:image" content={image} />
		<meta name="twitter:image" content={image} />
	{/if}
</svelte:head>
