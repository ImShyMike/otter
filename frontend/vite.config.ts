import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

const backendTarget = process.env.VITE_BACKEND_TARGET || 'http://localhost:3000';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	server: {
		proxy: {
			'/api': {
				target: backendTarget,
				changeOrigin: true
			}
		}
	},
	preview: {
		proxy: {
			'/api': {
				target: backendTarget,
				changeOrigin: true
			}
		}
	}
});
