import { getInvitationInfo } from '$lib/api/groups';
import type { PageLoad } from './$types';
export const ssr = false;
export const csr = true;
export const load: PageLoad = async ({ params }) => {
	const id = params.id;
	console.log(id);

	const info = await getInvitationInfo(id);
	console.log(info);
	return { info, id };
};
