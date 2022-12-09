import { writable } from "svelte/store";
import { isAuthorized, messages } from "../stores";

let ws: WebSocket = undefined;

let loadedMessagesCount = 0;

export const isBlocked = writable(true);
const wsPath = `ws://${window.location.host}/api/chat/websocket`;
const createWebSocket = () => {
    console.log("Creating new socket");
    ws = new WebSocket(wsPath);
};

const unsubscribeMessages = messages.subscribe((value) => {
    loadedMessagesCount = value.length;
    console.log(`Loaded messages number: ${loadedMessagesCount}`);
});

const unsubscribeIsAuthorized = isAuthorized.subscribe((is) => {
    if (is) {
        console.log("Auhorized connection with socket");
        if (ws == undefined) {
            try {
                createWebSocket();
                handleSocketEvents();
            } catch (e) {
                console.error(`Unexpected error while opening socket: ${e}`);
            }
        } else {
            console.log("Using current socket");
        }
    } else {
        if (ws != undefined) {
            console.log("Closing socket connecetion by logout");
            ws.close(1000, "logout");
            ws = undefined;
        } else {
            console.log("Cannot close missing socket");
        }
    }
});

function handleSocketEvents() {
    ws.onopen = (e) => {
        console.log("Socket opened");
    };

    ws.onmessage = (e) => {
        const msg = e.data;
        handleSocketMessage(msg);
    };

    ws.onerror = (e) => {
        console.log("Socket error");
    };

    ws.onclose = (e) => {
        const { wasClean, code, reason } = e;
        if (wasClean) {
            console.log(`Socket closed cleanly`);
        } else {
            console.log(`Socket closed unexpectedly`);
            setTimeout(() => {
                createWebSocket();
            }, 1000);
        }
        console.log(`Reason: ${reason} Code: ${code}`);
    };
}

function handleSocketMessage(data: any) {
    let message: any;
    try {
        message = JSON.parse(data);
    } catch (e) {
        console.log("Failed to parse message sent by a server");
        return;
    }

    const key = Object.keys(message)[0] as Action;
    console.log(`Socket action: ${key}`);
    if (key == Action.LoadMessages) {
        console.log("Loading new messages");
        let newMessages = message.LoadMessages as Array<MessageModel>;
        isBlocked.set(newMessages.length == 0 || newMessages.length % 5 != 0);
        messages.set(newMessages);
    } else if (key == Action.Message) {
        console.log("Loading new message");
        let newMessage = message.Message as MessageModel;
        messages.update((oldMessages) => oldMessages.concat([newMessage]));
    } else if (key == Action.LoadRequested) {
        console.log("Loading old messages");
        let oldMessages = message.LoadRequested as Array<MessageModel>;
        isBlocked.set(oldMessages.length == 0);
        messages.update((newerMessages) => oldMessages.concat(newerMessages));
    } else {
        console.log("Unknown server action");
    }
}

function socketSend(payload: any) {
    if (ws.readyState == ws.CLOSED) {
        console.error("Cannot send message to closed socket");
        return;
    }
    ws.send(JSON.stringify(payload));
}

function disconnect() {
    console.log("Disconnecting");
    ws.close(1000, "don't know why");
}

export function changeGroup(group_id: string) {
    socketSend({ ChangeGroup: { group_id } });
}

export function sendMessage(content: string) {
    socketSend({ SendMessage: { content } });
}

export function requestMessageLoad() {
    socketSend({ RequestMessages: { loaded: loadedMessagesCount } });
}

enum Action {
    Message = "Message",
    LoadMessages = "LoadMessages",
    LoadRequested = "LoadRequested",
}
