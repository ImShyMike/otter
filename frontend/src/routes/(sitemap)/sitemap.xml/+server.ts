import { API_BASE } from '$lib/search';
import { SITEMAP_CACHE_CONTROL, SITEMAP_CHUNK_SIZE, escapeXml } from '$lib/sitemap-utils';
import type { RequestHandler } from './$types';

interface QueryResponse {
	total: number;
}

async function fetchTotalProjects(fetcher: typeof fetch): Promise<number> {
	try {
		const res = await fetcher(`${API_BASE}/api/v1/query`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({
				filters: [],
				order_by: 'approved_at',
				sort_direction: 'desc',
				limit: 1,
				page: 1
			})
		});

		if (!res.ok) return 0;

		const body = (await res.json()) as QueryResponse;
		return body.total ?? 0;
	} catch {
		return 0;
	}
}

function buildIndex(origin: string, numProjectChunks: number): string {
	const today = new Date().toISOString().slice(0, 10);

	const sitemaps = [
		{ loc: `${origin}/sitemap-static.xml`, lastmod: today },
		...Array.from({ length: numProjectChunks }, (_, i) => ({
			loc: `${origin}/sitemap-projects/${i + 1}.xml`,
			lastmod: today
		}))
	];

	return `<?xml version="1.0" encoding="UTF-8"?>
<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${sitemaps
	.map(
		(s) => `	<sitemap>
		<loc>${escapeXml(s.loc)}</loc>
		<lastmod>${s.lastmod}</lastmod>
	</sitemap>`
	)
	.join('\n')}
</sitemapindex>
`;
}

export const GET: RequestHandler = async ({ url, fetch, request, platform }) => {
	const cache = platform?.caches?.default;

	if (cache) {
		const cached = await cache.match(request);
		if (cached) return cached;
	}

	const total = await fetchTotalProjects(fetch);
	const numProjectChunks = Math.max(1, Math.ceil(total / SITEMAP_CHUNK_SIZE));

	const xml = buildIndex(url.origin, numProjectChunks);

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
