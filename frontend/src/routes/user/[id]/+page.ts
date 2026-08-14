import { API_BASE, USER_PROJECTS_PER_PAGE } from '$lib/search';
import type { UserProfile } from '$lib/types';
import type { PageLoad } from './$types';

/** A username page that also covers the Slack account being viewed, plus others. */
export type SharedUsername = {
	identifier: string;
	totalSlackAccounts: number;
	totalProjects: number;
};

async function fetchUser(
	fetcher: typeof fetch,
	id: string,
	limit: number
): Promise<UserProfile | null> {
	try {
		const res = await fetcher(
			`${API_BASE}/api/v1/user/${encodeURIComponent(id)}?limit=${limit}&page=1`
		);
		if (!res.ok) return null;
		return (await res.json()) as UserProfile;
	} catch {
		return null;
	}
}

/**
 * Reverse lookup for a Slack ID page: the projects are usually also reachable under
 * their owner username, and that username may cover several Slack accounts.
 */
async function findSharedUsername(
	fetcher: typeof fetch,
	user: UserProfile
): Promise<SharedUsername | null> {
	if (user.matched_by !== 'slack_id') return null;

	const candidate = user.usernames[0];
	if (!candidate) return null;

	const owner = await fetchUser(fetcher, candidate, 1);
	if (!owner || owner.matched_by !== 'username') return null;
	if (Number(owner.total_slack_accounts) <= 1) return null;

	return {
		identifier: owner.identifier,
		totalSlackAccounts: Number(owner.total_slack_accounts),
		totalProjects: Number(owner.total_projects)
	};
}

export const load: PageLoad = async ({ fetch, params }) => {
	const user = await fetchUser(fetch, params.id, USER_PROJECTS_PER_PAGE);
	if (!user) {
		return { user: null, sharedUsername: null };
	}

	return { user, sharedUsername: await findSharedUsername(fetch, user) };
};
