import { API_BASE } from './search';
import type { SlackAccount, UsernameSuggestion } from './types';

export type FilterKind = 'user' | 'slack';

/** The `user:`/`slack:` token the caret currently sits in. */
export type FilterToken = {
	kind: FilterKind;
	/** What was typed after the colon */
	value: string;
	/** Index of the `u`/`s` in the full query */
	start: number;
	/** Index just past the token in the full query */
	end: number;
};

export type Suggestion = {
	/** Text the token is replaced with */
	insert: string;
	label: string;
	sublabel: string | null;
	imageUrl: string | null;
	projectCount: number;
};

const TOKEN_RE = /(?:^|\s)(user|slack):(\S*)$/i;
const SUGGESTION_LIMIT = 7;

export const MIN_QUERY_LENGTH = 1;

export function activeToken(query: string, caret: number): FilterToken | null {
	const match = TOKEN_RE.exec(query.slice(0, caret));
	if (!match) return null;

	// the token can continue past the caret, so walk to the end of the run
	let end = caret;
	while (end < query.length && !/\s/.test(query[end])) end++;

	const start = caret - match[1].length - 1 - match[2].length;
	return {
		kind: match[1].toLowerCase() as FilterKind,
		value: query.slice(start + match[1].length + 1, end),
		start,
		end
	};
}

/** Replace `token` with `insert`, returning the new query and where the caret goes. */
export function applySuggestion(query: string, token: FilterToken, insert: string) {
	const trailing = query.slice(token.end);
	const spacer = trailing.startsWith(' ') ? '' : ' ';
	return {
		value: query.slice(0, token.start) + insert + spacer + trailing,
		caret: token.start + insert.length + spacer.length
	};
}

function slackLabel(account: SlackAccount): string {
	return account.display_name ?? account.handle ?? account.real_name ?? account.slack_id;
}

function slackSublabel(account: SlackAccount): string | null {
	const label = slackLabel(account);
	const extras = [account.handle, account.real_name].filter(
		(name): name is string => !!name && name !== label
	);
	return extras.length > 0 ? extras.join(' · ') : null;
}

const cache = new Map<string, Suggestion[]>();

export async function fetchSuggestions(
	kind: FilterKind,
	query: string,
	signal?: AbortSignal
): Promise<Suggestion[]> {
	if (query.length < MIN_QUERY_LENGTH) return [];

	const key = `${kind}:${query.toLowerCase()}`;
	const cached = cache.get(key);
	if (cached) return cached;

	const res = await fetch(
		`${API_BASE}/api/v1/autocomplete/${kind}?q=${encodeURIComponent(query)}&limit=${SUGGESTION_LIMIT}`,
		{ signal }
	);
	if (!res.ok) throw new Error(`Autocomplete failed with HTTP ${res.status}`);

	const suggestions: Suggestion[] =
		kind === 'slack'
			? ((await res.json()) as SlackAccount[]).map((account) => ({
					insert: `slack:${account.slack_id}`,
					label: slackLabel(account),
					sublabel: slackSublabel(account),
					imageUrl: account.image,
					projectCount: Number(account.project_count)
				}))
			: ((await res.json()) as UsernameSuggestion[]).map((suggestion) => ({
					insert: `user:${suggestion.username}`,
					label: suggestion.username,
					sublabel: suggestion.is_github ? 'GitHub' : null,
					imageUrl: null,
					projectCount: Number(suggestion.project_count)
				}));

	cache.set(key, suggestions);
	return suggestions;
}
