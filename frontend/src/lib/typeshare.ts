/*
 Generated by typeshare 1.7.0
*/

export interface ErrorResponse {
	error: string;
}

export interface LoginCredentials {
	email: string;
	password: string;
}

export interface RegisterCredentials {
	email: string;
	password: string;
	username: string;
}

export interface AddresedMessage {
	content: string;
	user_id: string;
	group_id: string;
}

export interface GroupUserMessageModel {
	nickname: string;
	content: string;
	sent_at: string;
}

export interface GroupUserMessage {
	nickname: string;
	content: string;
	sat: number;
}

export interface KickMessage {
	from: string;
	reason: string;
}

export interface IdentifiedFriendIvitation {
	user_id: string;
}

export interface FriendInvitationResponse {
	sender_id: string;
	is_accepted: boolean;
}

export enum ActivityStatus {
	Online = "Online",
	Offline = "Offline",
	Idle = "Idle",
}

export interface FriendModel {
	note: string;
	status: ActivityStatus;
	profile_picture_url: string;
}

export interface FriendList {
	friends: FriendModel[];
}

export interface NewGroup {
	name: string;
}

export interface Group {
	id: string;
	name: string;
}

export interface GroupUser {
	user_id: string;
	group_id: string;
}

export interface NewGroupInvitation {
	group_id: string;
}

export interface GroupInfo {
	name: string;
	members: number;
}

export enum Role {
	Member = "member",
	Admin = "admin",
	Owner = "owner",
}

export interface ReceiveRoleOutput {
	role: Role;
}

export interface UserRoleChangeInput {
	group_id: string;
	value: Role;
}

export interface GroupPrivileges {
	privileges: Record<Role, number>;
}

export type Privilege = 
	| { type: "CanInvite", content: boolean }
	| { type: "CanSendMessages", content: boolean };

