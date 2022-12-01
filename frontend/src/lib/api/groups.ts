export const getGroups = async () => {
	const res = await fetch(`/api/groups`, {
		method: 'GET',
	});
	if (!res.ok) return [];

	const json = await res.json();
	console.log(json);
	return json.groups as Array<Group>;
};

export const getInvitationID = async (groupID: string): Promise<string | undefined> => {
	const res = await fetch(`/api/groups/invite`, {
		method: 'POST',
		headers: { 'Content-type': 'application/json' },
		body: JSON.stringify({ group_id: groupID })
	});
	if (!res.ok) return;

	const json = await res.json();
	console.log(json);
	return json.id as string;
};

export interface InvitationInfo {
	name: string;
	members: number;
}

export const getInvitationInfo = async (id: string): Promise<InvitationInfo | undefined> => {
	console.log(`/api/api/groups/info/${id}`);
	const res = await fetch(`/api/groups/info/${id}`, {
		method: 'GET',
	});
	if (!res.ok) return;
	const json = await res.json();

	return json as InvitationInfo;
};

export const joinGroupById = async (id: string): Promise<boolean> => {
	const res = await fetch(`/api/groups/join/${id}`, {
		method: 'GET',
	});
	if (!res.ok) false;
	return true;
};

export const createNewGroup = async (name: string) => {
	const res = await fetch(`/api/groups`, {
		method: 'POST',
		headers: { 'Content-type': 'application/json' },
		body: JSON.stringify({ name })
	});
	if (!res.ok) false;
	return true;
};
export interface Group {
	name: string;
	id: string;
}
