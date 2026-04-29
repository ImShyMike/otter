<script lang="ts">
	import './layout.css';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import StarIcon from '@lucide/svelte/icons/star';
	import { API_BASE } from '$lib/search';

	let { children } = $props();
</script>

<div class="absolute top-4 right-4 z-50 sm:fixed sm:top-auto sm:bottom-4">
	<ThemeToggle />
</div>

<div class="absolute top-4 left-4 z-50 sm:fixed sm:top-auto sm:bottom-4">
	<a
		href="https://github.com/ImShyMike/otter"
		target="_blank"
		rel="noopener"
		class="inline-flex size-12 items-center justify-center rounded-md bg-transparent text-sm font-medium ring-offset-background transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:outline-none"
		aria-label="View on GitHub"
		title="View on GitHub"
		data-umami-event="github-link"
	>
		<StarIcon class="size-6" />
	</a>
</div>

<svelte:head>
	<link rel="preconnect" href={API_BASE} />
	<link rel="preconnect" href="https://airtableusercontent.com" />
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
