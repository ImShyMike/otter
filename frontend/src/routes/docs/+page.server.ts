import { redirect } from '@sveltejs/kit';
import { env } from '$env/dynamic/public';

export function load() {
	const api = env.PUBLIC_API_BASE || 'http://localhost:3000';
	redirect(307, `${api}/docs`);
}
