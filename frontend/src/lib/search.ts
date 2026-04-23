import { env } from '$env/dynamic/public';
import type { SearchResult } from './types';

export const API_BASE = (env.PUBLIC_API_BASE || 'http://localhost:3000').replace(/\/$/, '');

export function imageUrl(airtable_id: string) {
	return `${API_BASE}/api/media/${airtable_id}/r`;
}

export function title(r: SearchResult) {
	return r.inferred_repo ?? `Project #${r.id}`;
}

export function truncate(s: string | null, len = 200) {
	if (!s) return '';
	return s.length > len ? s.slice(0, len) + '…' : s;
}
