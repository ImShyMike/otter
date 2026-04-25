<script lang="ts">
	import Sun from '@lucide/svelte/icons/sun';
	import Moon from '@lucide/svelte/icons/moon';
	import { Button } from '$lib/components/ui/button';
	import { onMount } from 'svelte';

	let isDark = $state(false);

	function setTheme(dark: boolean) {
		document.documentElement.classList.toggle('dark', dark);
		document.documentElement.style.colorScheme = dark ? 'dark' : 'light';
		localStorage.setItem('theme', dark ? 'dark' : 'light');
		isDark = dark;
	}

	function toggleTheme() {
		setTheme(!isDark);
	}

	onMount(() => {
		isDark = document.documentElement.classList.contains('dark');
	});
</script>

<Button
	variant="ghost"
	size="icon"
	class="size-12"
	onclick={toggleTheme}
	aria-label={isDark ? 'Switch to light mode' : 'Switch to dark mode'}
	title={isDark ? 'Switch to light mode' : 'Switch to dark mode'}
	data-umami-event="theme-toggle"
	data-umami-event-theme={isDark ? 'light' : 'dark'}
>
	{#if isDark}
		<Sun class="size-6" />
	{:else}
		<Moon class="size-6" />
	{/if}
</Button>
