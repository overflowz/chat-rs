<script lang="ts">
  import Chatroom from "./Chatroom.svelte";
  import { getBaseUrl } from "./config";
  import Registration from "./Registration.svelte";

  let token: string | undefined = sessionStorage.getItem("token");

  let tokenStatus = token
    ? fetch(`${getBaseUrl()}/status/${token}`).then((resp) => resp.json())
    : Promise.reject();
</script>

{#await tokenStatus}
  <h1>Loading...</h1>
{:then client}
  <Chatroom {client} {token} />
{:catch}
  <Registration />
{/await}
