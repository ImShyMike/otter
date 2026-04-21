<script lang="ts">
	import { cn } from '$lib/utils.js';
	import Input from './input.svelte';

	interface Props {
		value: string;
		options: string[];
		placeholder?: string;
		class?: string;
		onselect?: () => void;
	}

	let {
		value = $bindable(''),
		options,
		placeholder = '',
		class: className,
		onselect
	}: Props = $props();

	let open = $state(false);
	let highlightIndex = $state(0);

	const filtered = $derived(
		value.trim()
			? options.filter((o) => o.toLowerCase().includes(value.trim().toLowerCase()))
			: options
	);

	function select(option: string) {
		value = option;
		open = false;
		onselect?.();
	}
</script>

<div
	class="relative"
	onfocusin={() => {
		open = true;
		highlightIndex = 0;
	}}
	onfocusout={(e) => {
		if (!e.currentTarget.contains(e.relatedTarget as Node)) open = false;
	}}
>
	<Input
		type="text"
		{placeholder}
		bind:value
		oninput={() => {
			open = true;
			highlightIndex = 0;
		}}
		onkeydown={(e) => {
			if (!open || filtered.length === 0) return;
			if (e.key === 'ArrowDown') {
				e.preventDefault();
				highlightIndex = Math.min(highlightIndex + 1, filtered.length - 1);
			} else if (e.key === 'ArrowUp') {
				e.preventDefault();
				highlightIndex = Math.max(highlightIndex - 1, 0);
			} else if (e.key === 'Enter' || e.key === 'Tab') {
				e.preventDefault();
				select(filtered[highlightIndex]);
			} else if (e.key === 'Escape') {
				open = false;
			}
		}}
		class={cn('h-8 w-44', className)}
		autocomplete="off"
	/>
	{#if open && filtered.length > 0}
		<div
			class="absolute top-full left-0 z-50 mt-1 max-h-48 w-56 overflow-y-auto rounded border bg-popover p-1 shadow-md"
		>
			{#each filtered as option, i (option)}
				<button
					class="w-full rounded-sm px-2 py-1.5 text-left text-sm {i === highlightIndex
						? 'bg-muted'
						: 'hover:bg-muted'}"
					onmousedown={(e) => {
						e.preventDefault();
						select(option);
					}}
					onmouseenter={() => (highlightIndex = i)}
				>
					{option}
				</button>
			{/each}
		</div>
	{/if}
</div>
