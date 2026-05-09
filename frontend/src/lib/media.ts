import { writable, type Readable, type Writable } from 'svelte/store';
import { API_BASE } from './search';
import type { MediaBatchResponse, MediaItem } from './types';

export type MediaState =
	| { status: 'loading' }
	| { status: 'loaded'; items: MediaItem[] }
	| { status: 'error' };

const cache = new Map<string, Writable<MediaState>>();
const pending = new Set<string>();
let flushTimer: ReturnType<typeof setTimeout> | null = null;
const FLUSH_DELAY_MS = 30;
const MAX_BATCH = 100;

function ensureFlushScheduled() {
	if (flushTimer || pending.size === 0) return;
	flushTimer = setTimeout(flush, FLUSH_DELAY_MS);
}

async function flush() {
	flushTimer = null;
	if (pending.size === 0) return;

	const ids = Array.from(pending).slice(0, MAX_BATCH);
	for (const id of ids) pending.delete(id);

	try {
		const res = await fetch(`${API_BASE}/api/v1/media/batch`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ ids })
		});
		if (!res.ok) throw new Error(`Batch failed: ${res.status}`);
		const data = (await res.json()) as MediaBatchResponse;

		for (const id of ids) {
			const items = data.media?.[id] ?? [];
			const store = cache.get(id);
			store?.set({ status: 'loaded', items });
		}
	} catch {
		for (const id of ids) {
			const store = cache.get(id);
			store?.set({ status: 'error' });
		}
	}

	if (pending.size > 0) ensureFlushScheduled();
}

export function getMedia(id: string): Readable<MediaState> {
	let store = cache.get(id);
	if (!store) {
		store = writable<MediaState>({ status: 'loading' });
		cache.set(id, store);
		pending.add(id);
		ensureFlushScheduled();
	}
	return store;
}

export function thumbUrl(item: MediaItem): string {
	return item.thumb_large_url ?? item.thumb_small_url ?? item.url;
}

export function fullUrl(item: MediaItem): string {
	return item.url;
}
