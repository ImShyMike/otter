import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import type { PathnameWithSearchOrHash } from '$app/types';
import type { RouteIdWithSearchOrHash } from '$app/types';
import { writable } from 'svelte/store';

export const lastPageUrl = writable<string | null>(null);
let lastPage: string | null = null;

lastPageUrl.subscribe((value) => {
	lastPage = value;
});

export function goBack(event: MouseEvent) {
	if (lastPage) {
		event.preventDefault();
		goto(resolve(lastPage as RouteIdWithSearchOrHash | PathnameWithSearchOrHash));
	} else {
		goto(resolve('/'));
	}
}
