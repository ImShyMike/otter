<script lang="ts">
	import './layout.css';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import { API_BASE } from '$lib/search';
	import { page } from '$app/state';

	let { children } = $props();

	const siteOrigin = $derived(page.url?.origin ?? 'https://search.shymike.dev');
	const websiteJsonLd = $derived(
		JSON.stringify({
			'@context': 'https://schema.org',
			'@type': 'WebSite',
			name: 'Otter',
			alternateName: 'Otter · Search engine for all Hack Club projects',
			url: `${siteOrigin}/`,
			description:
				'Semantic search engine for projects submitted to Hack Club YSWS (You Ship, We Ship) programs. Search thousands of teen-built websites, apps, games, and hardware projects.',
			potentialAction: {
				'@type': 'SearchAction',
				target: {
					'@type': 'EntryPoint',
					urlTemplate: `${siteOrigin}/?q={search_term_string}`
				},
				'query-input': 'required name=search_term_string'
			}
		})
	);
	const openTag = '<script type="application/ld+json">';
	// why write it like this, you may be asking? the linter thinks im closing a nonexistant tag for some reason
	const closeTag = '</scr' + 'ipt>';
</script>

<div class="absolute top-4 right-4 z-50 sm:fixed sm:top-auto sm:bottom-4">
	<ThemeToggle />
</div>

<svelte:head>
	<link rel="preconnect" href={API_BASE} />
	<link rel="preconnect" href="https://airtableusercontent.com" />
	<link
		rel="search"
		type="application/opensearchdescription+xml"
		title="Otter"
		href="/opensearch.xml"
	/>
	<!-- eslint-disable-next-line svelte/no-at-html-tags -->
	{@html `${openTag}${websiteJsonLd}${closeTag}`}
	<script>
		(() => {
			const storedTheme = localStorage.getItem('theme');
			const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
			const dark = storedTheme ? storedTheme === 'dark' : prefersDark;
			document.documentElement.classList.toggle('dark', dark);
			document.documentElement.style.colorScheme = dark ? 'dark' : 'light';
		})();
	</script>
</svelte:head>

{@render children()}
