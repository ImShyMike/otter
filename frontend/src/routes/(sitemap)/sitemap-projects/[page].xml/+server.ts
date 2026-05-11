import { API_BASE } from '$lib/search';
import {
	SITEMAP_API_PAGE_SIZE,
	SITEMAP_CACHE_CONTROL,
	SITEMAP_CHUNK_SIZE,
	escapeXml
} from '$lib/sitemap-utils';
import type { ProjectItem } from '$lib/types';
import { error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

interface QueryResponse {
	data: ProjectItem[];
	total: number;
	page: number;
	per_page: number;
}

async function fetchChunk(fetcher: typeof fetch, chunkIndex: number): Promise<ProjectItem[]> {
	const startOffset = chunkIndex * SITEMAP_CHUNK_SIZE;
	const startApiPage = Math.floor(startOffset / SITEMAP_API_PAGE_SIZE) + 1;
	const numApiPages = Math.ceil(SITEMAP_CHUNK_SIZE / SITEMAP_API_PAGE_SIZE);

	const items: ProjectItem[] = [];
	for (let i = 0; i < numApiPages; i++) {
		const apiPage = startApiPage + i;
		const res = await fetcher(`${API_BASE}/api/v1/query`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({
				filters: [],
				order_by: 'approved_at',
				sort_direction: 'desc',
				limit: SITEMAP_API_PAGE_SIZE,
				page: apiPage
			})
		});

		if (!res.ok) break;

		const body = (await res.json()) as QueryResponse;
		if (!body.data?.length) break;

		items.push(...body.data);

		if (items.length >= SITEMAP_CHUNK_SIZE) break;
		if (body.data.length < SITEMAP_API_PAGE_SIZE) break;
	}

	return items.slice(0, SITEMAP_CHUNK_SIZE);
}

function buildProjectsSitemap(origin: string, projects: ProjectItem[]): string {
	const today = new Date().toISOString().slice(0, 10);

	return `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${projects
	.map((p) => {
		const lastmod = p.approved_at
			? new Date(p.approved_at * 1000).toISOString().slice(0, 10)
			: today;
		return `	<url>
		<loc>${escapeXml(`${origin}/project/${p.airtable_id}`)}</loc>
		<lastmod>${lastmod}</lastmod>
		<changefreq>monthly</changefreq>
		<priority>0.5</priority>
	</url>`;
	})
	.join('\n')}
</urlset>
`;
}

export const GET: RequestHandler = async ({ url, fetch, request, params, platform }) => {
	const pageNum = Number(params.page);
	if (!Number.isInteger(pageNum) || pageNum < 1) {
		throw error(404, 'Not Found');
	}

	const cache = platform?.caches?.default;

	if (cache) {
		const cached = await cache.match(request);
		if (cached) return cached;
	}

	let projects: ProjectItem[];
	try {
		projects = await fetchChunk(fetch, pageNum - 1);
	} catch {
		projects = [];
	}

	const xml = buildProjectsSitemap(url.origin, projects);

	const response = new Response(xml, {
		headers: {
			'Content-Type': 'application/xml; charset=utf-8',
			'Cache-Control': SITEMAP_CACHE_CONTROL
		}
	});

	if (cache && platform?.context) {
		platform.context.waitUntil(cache.put(request, response.clone()));
	}

	return response;
};
