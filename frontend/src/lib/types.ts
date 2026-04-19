export interface SearchResult {
	id: number;
	display_name: string | null;
	description: string | null;
	ysws: string;
	country: string | null;
	code_url: string | null;
	demo_url: string | null;
	has_media: boolean;
	score: number;
}
