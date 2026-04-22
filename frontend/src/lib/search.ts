import { env } from '$env/dynamic/public';

export const API_BASE = (env.PUBLIC_API_BASE || 'http://localhost:3000').replace(/\/$/, '');

export function imageUrl(airtable_id: string) {
	return `${API_BASE}/api/media/${airtable_id}/r`;
}

function parseCodeUrl(url: string | null, index: number) {
	return (
		url
			?.replace(/^git@[^:]+:/, '')
			.replace(/^[a-z]+:\/\/[^/]+\//i, '')
			.replace(/^\/+/, '')
			.replace(/\.git$/, '')
			.split('/')[index] ||
		''.replace(/[-_]/g, ' ') ||
		''
	);
}

export function repoFromCodeUrl(url: string | null) {
	const repo = parseCodeUrl(url, 1);

	if (repo?.length > 50 || repo.length === 0) {
		return null;
	}

	return repo;
}

export function usernameFromCodeUrl(url: string | null) {
	const username = parseCodeUrl(url, 0);

	if (username?.length > 50 || username.length === 0) {
		return null;
	}

	return username;
}

export function title(r: { id: number; code_url: string | null }) {
	return repoFromCodeUrl(r.code_url) ?? `Project #${r.id}`;
}

export function truncate(s: string | null, len = 200) {
	if (!s) return '';
	return s.length > len ? s.slice(0, len) + '…' : s;
}
