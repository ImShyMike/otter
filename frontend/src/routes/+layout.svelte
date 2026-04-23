<script lang="ts">
	import './layout.css';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';

	let { children } = $props();
</script>

<div class="fixed right-4 bottom-4 z-50">
	<ThemeToggle />
</div>

<svelte:head>
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
