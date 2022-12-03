import { request } from "./fetch";

export const getGroups = async () => {
	const res = await request(`/api/groups`);
	if (!res.ok) return [];
	return res.data.groups as Array<Group>;
};

export const getInvitationID = async (groupID: string): Promise<string | undefined> => {
	const res = await request(`/api/groups/invite`, {
		method: 'POST',
		body: { group_id: groupID }
	});
	if (!res.ok) return;
	return res.data.id as string;
};

export interface InvitationInfo {
	name: string;
	members: number;
}

export const createNewGroup = async (name: string) => {
	const res = await request(`/api/groups`, {
		method: 'POST',
		body: { name }
	});
	return res.ok
};
export interface Group {
	name: string;
	id: string;
}
