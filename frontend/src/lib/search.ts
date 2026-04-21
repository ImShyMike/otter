import { env } from '$env/dynamic/public';

export const API_BASE = (env.PUBLIC_API_BASE || 'http://localhost:3000').replace(/\/$/, '');

export function imageUrl(airtable_id: string) {
	return `${API_BASE}/api/media/${airtable_id}/r`;
}

export function nameFromCodeUrl(url: string | null) {
	return url
		?.replace(/https?:\/\/(www\.)?/, '')
		.split('/')[2]
		?.replace(/\.git$/, '')
		.replace(/[-_]/g, ' ');
}

export function title(r: { id: number; code_url: string | null }) {
	return nameFromCodeUrl(r.code_url) ?? `Project #${r.id}`;
}

export function truncate(s: string | null, len = 200) {
	if (!s) return '';
	return s.length > len ? s.slice(0, len) + '…' : s;
}
