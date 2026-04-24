import { API_BASE } from '$lib/search';
import type { ProjectItem } from '$lib/types';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch, params }) => {
	try {
		const res = await fetch(`${API_BASE}/api/v1/project/${params.id}`);
		if (!res.ok) {
			return { project: null };
		}

		const project = (await res.json()) as ProjectItem;
		return { project };
	} catch {
		return { project: null };
	}
};
