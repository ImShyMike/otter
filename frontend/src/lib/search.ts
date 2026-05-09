import type { ProjectItem } from './types';

export const API_PREFIX = '/api/v1';

const SCORE_BRACKETS: [number, string][] = [
	[0.25, 'text-destructive'],
	[0.5, 'text-muted-foreground'],
	[0.75, 'text-foreground/70'],
	[1, 'text-foreground/90']
];

export function scoreClass(score: number | null): string {
	if (score === null) return SCORE_BRACKETS[0][1];
	for (const [threshold, cls] of SCORE_BRACKETS) {
		if (score <= threshold) return cls;
	}
	return SCORE_BRACKETS[SCORE_BRACKETS.length - 1][1];
}

export function imageUrl(airtable_id: string) {
	return `${API_PREFIX}/media/${airtable_id}/r`;
}

export function title(r: ProjectItem) {
	return r.inferred_repo ?? `Project #${r.id}`;
}

export function truncate(s: string | null, len = 200) {
	if (!s) return '';
	return s.length > len ? s.slice(0, len) + '…' : s;
}
