export const api = import.meta.env.VITE_PUBLIC_BACKEND_URL;
const getWebSocketPath = () => {
    const uri ="chat/websocket"
	const urlParts = api.split('://');
	const protocol = urlParts[0];
	const base = urlParts[1];
	if (protocol == 'https') {
		return `wss://${base}/chat/websocket`;
	}
	return `ws://${base}/${uri}`;
};

export const wsPath = getWebSocketPath();
