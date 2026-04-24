import { env } from '$env/dynamic/public';
import type { SearchResult } from './types';

export const API_BASE = (env.PUBLIC_API_BASE || 'http://localhost:3000').replace(/\/$/, '');

const SCORE_BRACKETS: [number, string][] = [
	[0, 'text-muted-foreground'],
	[0.25, 'text-red-500'],
	[0.5, 'text-yellow-500'],
	[0.75, 'text-green-700'],
	[1, 'text-blue-700']
];

export function scoreClass(score: number | null): string {
	if (score === null) return SCORE_BRACKETS[0][1];
	for (const [threshold, cls] of SCORE_BRACKETS) {
		if (score <= threshold) return cls;
	}
	return SCORE_BRACKETS[SCORE_BRACKETS.length - 1][1];
}

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
