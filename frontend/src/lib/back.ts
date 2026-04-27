import { goto } from '$app/navigation';
import { resolve } from '$app/paths';

export function goBack(event: MouseEvent) {
	event.preventDefault();

	if (typeof window !== 'undefined' && window.history.length > 1) {
		window.history.back();
		return;
	}

	goto(resolve('/'));
}
