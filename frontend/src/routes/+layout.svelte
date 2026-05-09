<script lang="ts">
	import './layout.css';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';

	let { children } = $props();
</script>

<div class="absolute top-4 right-4 z-50 sm:fixed sm:top-auto sm:bottom-4">
	<ThemeToggle />
</div>

<svelte:head>
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
