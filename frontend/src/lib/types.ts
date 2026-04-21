export interface SearchResult {
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
	score: number | null;
}
