import { API_BASE } from '$lib/search';
import type { StatsResponse } from '$lib/types';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => {
	try {
		const res = await fetch(`${API_BASE}/api/v1/stats`);
		if (!res.ok) return { stats: null };
		return { stats: (await res.json()) as StatsResponse };
	} catch {
		return { stats: null };
	}
};
