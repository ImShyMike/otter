import { SITEMAP_CACHE_CONTROL, escapeXml } from '$lib/sitemap-utils';
import type { RequestHandler } from './$types';

function buildStaticSitemap(origin: string): string {
	const today = new Date().toISOString().slice(0, 10);

	const urls = [
		{ loc: `${origin}/`, changefreq: 'daily', priority: '1.0', lastmod: today },
		{ loc: `${origin}/explore`, changefreq: 'daily', priority: '0.8', lastmod: today }
	];

	return `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls
	.map(
		(u) => `	<url>
		<loc>${escapeXml(u.loc)}</loc>
		<lastmod>${u.lastmod}</lastmod>
		<changefreq>${u.changefreq}</changefreq>
		<priority>${u.priority}</priority>
	</url>`
	)
	.join('\n')}
</urlset>
`;
}

export const GET: RequestHandler = async ({ url, request, platform }) => {
	const cache = platform?.caches?.default;

	if (cache) {
		const cached = await cache.match(request);
		if (cached) return cached;
	}

	const xml = buildStaticSitemap(url.origin);

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
