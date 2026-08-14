<script lang="ts">
	import type { Suggestion } from '$lib/autocomplete';

	let {
		items,
		highlighted,
		onselect,
		onhighlight
	}: {
		items: Suggestion[];
		highlighted: number;
		onselect: (suggestion: Suggestion) => void;
		onhighlight: (index: number) => void;
	} = $props();
</script>

<ul
	id="search-suggestions"
	role="listbox"
	aria-label="Filter suggestions"
	class="absolute top-full right-0 left-0 z-50 mt-1 max-h-72 overflow-y-auto rounded-md border bg-popover p-1 text-left shadow-md"
>
	{#each items as item, i (item.insert)}
		<li>
			<button
				type="button"
				role="option"
				id="search-suggestion-{i}"
				aria-selected={i === highlighted}
				class="flex w-full cursor-pointer items-center gap-2 rounded-sm px-2 py-1.5 text-left {i ===
				highlighted
					? 'bg-muted'
					: 'hover:bg-muted'}"
				onmousedown={(e) => {
					// keep the input focused so the caret can be restored
					e.preventDefault();
					onselect(item);
				}}
				onmouseenter={() => onhighlight(i)}
				data-umami-event="autocomplete-select"
				data-umami-event-insert={item.insert}
			>
				{#if item.imageUrl}
					<img
						src={item.imageUrl}
						alt=""
						loading="lazy"
						class="size-6 shrink-0 rounded-[7px] bg-muted object-cover"
					/>
				{/if}
				<span class="min-w-0 flex-1 truncate text-sm">
					{item.label}
					{#if item.sublabel}
						<span class="ml-1 text-xs text-muted-foreground">{item.sublabel}</span>
					{/if}
				</span>
				<span class="shrink-0 text-xs text-muted-foreground">
					{item.projectCount} project{item.projectCount === 1 ? '' : 's'}
				</span>
			</button>
		</li>
	{/each}
</ul>
