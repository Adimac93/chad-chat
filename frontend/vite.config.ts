import { sveltekit } from '@sveltejs/kit/vite';
import type { UserConfig } from 'vite';
import settings from './config/settings.json';

const config: UserConfig = {
	plugins: [sveltekit()],
	server: {
		port: settings.origin.port,
		host: settings.origin.ip,
		https: settings.origin.secure,
		fs: { allow: ['..'] }
	}
};

export default config;
