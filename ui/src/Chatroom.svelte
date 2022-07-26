<script lang="ts">
  import { getBaseUrl, getWsUrl } from "./config";
  import { onMount } from "svelte";

  interface Client {
    name: string;
  }

  export let client: Client;
  export let token: string;

  let users: Client[] = [];

  let textField = "";
  let targetUser = client.name;
  let messageHistory = [];

  onMount(() => {
    const ws = new WebSocket(`${getWsUrl()}/${token}`);

    ws.onmessage = ({ data }) => {
      let msg = JSON.parse(data);

      messageHistory.push({
        from: msg.from,
        to: client.name,
        body: msg.body,
      });

      sessionStorage.setItem("chat_history", JSON.stringify(messageHistory));
      messageHistory = messageHistory;
    };

    fetch(`${getBaseUrl()}/clients`)
      .then((resp) => resp.json())
      .then((x) => (users = x));

    messageHistory = JSON.parse(sessionStorage.getItem("chat_history") ?? "[]");
  });

  const selectUser = (user: string) => {
    targetUser = user;
  };

  const sendMessage = async (event?: KeyboardEvent) => {
    if ((event && event.key !== "Enter") || !textField.length) {
      return;
    }

    await fetch(`${getBaseUrl()}/send_message`, {
      body: JSON.stringify({
        token,
        body: textField,
        to: targetUser,
      }),
      method: "POST",
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
    }).then(() => {
      messageHistory.push({
        from: client.name,
        to: targetUser,
        body: textField,
      });

      sessionStorage.setItem("chat_history", JSON.stringify(messageHistory));
      messageHistory = messageHistory;
      textField = "";
    });
  };
</script>

<div class="container">
  <div class="users w3-container">
    <ul class="w3-ul">
      <li
        on:click={() => selectUser(client.name)}
        class:w3-blue={targetUser === client.name}
        style="cursor: pointer;"
      >
        {client.name} (you)
      </li>
      {#each users as user}
        {#if user.name !== client.name}
          <li
            class:w3-blue={targetUser === user.name}
            style="cursor: pointer;"
            on:click={() => selectUser(user.name)}
          >
            {user.name}
          </li>
        {/if}
      {/each}
    </ul>
  </div>
  <div class="messages">
    <div class="messages-inner">
      <ul class="messages-ul">
        {#each messageHistory.filter((x) => (x.from === client.name && x.to === targetUser) || (x.from === targetUser && x.to === client.name)) as message}
          <li>{message.from}: {message.body}</li>
        {/each}
      </ul>
    </div>
    <div class="sender-container">
      <input
        class="message-sender-input"
        bind:value={textField}
        on:keypress={sendMessage}
      />
      <button class="send-message-btn" on:click={() => sendMessage()}
        >send</button
      >
    </div>
  </div>
</div>

<style lang="css">
  .container {
    display: flex;
    height: 100vh;
    padding: 1em;
  }

  .send-message-btn {
    flex-grow: 0.05;
    margin-left: 1em;
  }

  .message-sender-input {
    flex-grow: 0.95;
  }

  .sender-container {
    display: flex;
    flex-direction: row;
  }

  .messages-ul {
    list-style-type: none;
    padding-left: 1em;
  }

  .messages {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
  }

  .users {
    border: 1px solid black;
    flex-grow: 0.1;
    flex-wrap: nowrap;
  }

  .messages {
    margin-left: 1em;
    flex-grow: 0.9;
    padding: 5px;
    overflow-y: auto;
    overflow-x: hidden;
    word-wrap: break-word;
    border: 1px solid black;
  }
</style>
