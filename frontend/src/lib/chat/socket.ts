import { isAuthorized, messages } from "../stores";

export class Socket {
  webSocket: WebSocket;
  isBlocked = false;
  loaded = 0;

  constructor() {
    this.connect();
    this.webSocket.onopen = (e) => {
      console.log("Chat opened");
    };

    this.webSocket.onmessage = (e) => {
      try {
        const message = JSON.parse(e.data);
        const key = Object.keys(message)[0] as Action;
        console.log(key);
        if (key == Action.LoadMessages) {
          let newMessages = message.LoadMessages as Array<MessageModel>;
          if (newMessages.length == 0 || newMessages.length % 5 != 0) {
            this.isBlocked = true;
          } else {
            this.isBlocked = false;
          }
          messages.set(newMessages);
        } else if (key == Action.Message) {
          let newMessage = message.Message as MessageModel;
          messages.update((oldMessages) => oldMessages.concat([newMessage]));
        } else if (key == Action.LoadRequested) {
          let oldMessages = message.LoadRequested as Array<MessageModel>;
          if (oldMessages.length == 0) {
            this.isBlocked = true;
          }

          messages.update((newerMessages) => oldMessages.concat(newerMessages));
        }
      } catch (e) {
        console.log(`${e}`);
      }
    };
    const unsubscribeMessages = messages.subscribe((value) => {
      console.log(`Loaded messages number: ${value.length}`);

      this.loaded = value.length;
    });
    const unsubscribeIsAUthorized = isAuthorized.subscribe((is) => {
      if (!is) {
        this.webSocket.close();
        console.log("Closing");
      }
    });
  }
  connect() {
    console.log("Connecting");
    this.webSocket = new WebSocket(
      `ws://${window.location.host}/api/chat/websocket`
    );
  }
  disconnect() {
    console.log("Disconnecting");
    this.webSocket.close(1000, "don't know why");
  }

  private socketSend(payload: any) {
    this.webSocket.send(JSON.stringify(payload));
  }

  changeGroup(group_id: string) {
    this.socketSend({ ChangeGroup: { group_id } });
  }

  sendMessage(content: string) {
    this.socketSend({ SendMessage: { content } });
  }

  requestMessageLoad() {
    if (!this.isBlocked)
      this.socketSend({ RequestMessages: { loaded: this.loaded } });
  }
}
enum Action {
  Message = "Message",
  LoadMessages = "LoadMessages",
  LoadRequested = "LoadRequested",
}
