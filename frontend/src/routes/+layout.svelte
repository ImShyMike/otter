<script lang="ts">
	import './layout.css';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import { page } from '$app/state';
	import { lastPageUrl } from '$lib/stores/back';
	import { API_BASE } from '$lib/search';

	let { children } = $props();
	let previousUrl: string | null = null;

	$effect(() => {
		const currentUrl = `${page.url.pathname}${page.url.search}${page.url.hash}`;

		if (previousUrl && previousUrl !== currentUrl) {
			lastPageUrl.set(previousUrl);
		}

		previousUrl = currentUrl;
	});
</script>

<div class="fixed right-4 bottom-4 z-50">
	<ThemeToggle />
</div>

<svelte:head>
	<link rel="preconnect" href={API_BASE} />
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
