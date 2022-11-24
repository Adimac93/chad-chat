export const api = import.meta.env.VITE_PUBLIC_BACKEND_URL;
const wsGen = () => {
	const urlParts = api.split('://');
	const protocol = urlParts[0];
	const path = urlParts[1];
	if (protocol == 'https') {
		return `wss://${path}`;
	}
	return `ws://${path}`;
};

export const ws = wsGen();
