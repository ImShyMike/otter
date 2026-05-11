export const SITEMAP_CHUNK_SIZE = 500;
export const SITEMAP_API_PAGE_SIZE = 100;
export const SITEMAP_CACHE_CONTROL =
	'public, max-age=3600, s-maxage=86400, stale-while-revalidate=604800';

export function escapeXml(s: string): string {
	return s
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;')
		.replace(/'/g, '&apos;');
}
