export interface ProjectItem {
	id: number;
	airtable_id: string;
	approved_at: number | null;
	display_name: string | null;
	description: string | null;
	ysws: string;
	country: string | null;
	code_url: string | null;
	demo_url: string | null;
	github_username: string | null;
	hours: number | null;
	true_hours: number | null;
	has_media: boolean;
	github_stars: number;
	archived_demo: string | null;
	archived_repo: string | null;
	inferred_repo: string | null;
	inferred_username: string | null;
}

export interface SearchResult extends ProjectItem {
	score: number;
}

export interface SearchTimings {
	embeddings_ms: number;
	query_ms: number;
}

export interface SearchResults {
	data: SearchResult[];
	total: number;
	page: number;
	per_page: number;
	timings: SearchTimings;
}
