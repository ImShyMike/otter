<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';

	let { children } = $props();
</script>

<div class="fixed right-4 bottom-4 z-50">
	<ThemeToggle />
</div>

<svelte:head>
	<link rel="icon" href={favicon} />
	<meta property="og:site_name" content="Otter" />
	<meta property="og:type" content="website" />
	<meta name="description" content="Search engine for Hack Club projects" />
	<meta property="og:description" content="Search engine for Hack Club projects" />
	<meta name="twitter:card" content="summary" />
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
