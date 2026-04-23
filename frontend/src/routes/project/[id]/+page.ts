import { API_BASE } from '$lib/search';
import type { SearchResult } from '$lib/types';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch, params }) => {
	try {
		const res = await fetch(`${API_BASE}/api/project/${params.id}`);
		if (!res.ok) {
			return { project: null };
		}

		const project = (await res.json()) as SearchResult;
		return { project };
	} catch {
		return { project: null };
	}
};
