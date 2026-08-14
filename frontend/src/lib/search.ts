import { env } from '$env/dynamic/public';
import type { ProjectItem, UserProfile } from './types';

export const API_BASE = (env.PUBLIC_API_BASE || 'http://localhost:3000').replace(/\/$/, '');

export const USER_PROJECTS_PER_PAGE = 50;

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

export type ImageSize = 'small' | 'large' | 'full' | 'original';

export function imageUrl(airtable_id: string, size?: ImageSize) {
	const qs = size ? `?size=${size}` : '';
	return `${API_BASE}/api/v1/media/${airtable_id}/r${qs}`;
}

export function title(r: ProjectItem) {
	return r.inferred_repo ?? `Project #${r.id}`;
}

export function truncate(s: string | null, len = 200) {
	if (!s) return '';
	return s.length > len ? s.slice(0, len) + '…' : s;
}

export function userIdentifier(r: ProjectItem): string | null {
	return r.slack_id ?? r.inferred_username ?? r.github_username;
}

export function userDisplayName(u: UserProfile): string {
	return (
		u.slack?.display_name ??
		u.slack?.real_name ??
		u.slack?.handle ??
		u.usernames[0] ??
		u.display_names[0] ??
		u.identifier
	);
}

export function userSearchQuery(u: UserProfile): string {
	return u.matched_by === 'slack_id' ? `slack:${u.identifier}` : `user:${u.identifier}`;
}
