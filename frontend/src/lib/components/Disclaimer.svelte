<script lang="ts">
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import { onMount } from 'svelte';

	const STORAGE_KEY = 'disclaimer-dismissed';

	let open = $state(false);

	onMount(() => {
		open = localStorage.getItem(STORAGE_KEY) !== 'true';
	});

	function handleContinue() {
		localStorage.setItem(STORAGE_KEY, 'true');
		open = false;
	}
</script>

<AlertDialog.Root bind:open>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>Important Project Disclaimer</AlertDialog.Title>
			<AlertDialog.Description class="space-y-3 text-left">
				<p>
					Projects can take a while to appear in the Unified DB, and some may never be added if they
					do not meet quality standards.
				</p>
				<p>THE DATABASE ONLY CONTAINS <span class="font-bold">SUBMITTED PROJECTS</span></p>
				<p>Tips if you cannot find a project:</p>
				<ul class="list-disc pl-5">
					<li>
						Use quotes for exact matches (for example,
						<code>"rustytime"</code>).
					</li>
					<li>
						If it still does not show up, it may still be processing or may not be eligible yet.
					</li>
				</ul>
			</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Action onclick={handleContinue}>Got it</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
