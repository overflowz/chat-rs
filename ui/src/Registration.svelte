<script lang="ts">
  import { getBaseUrl } from "./config";

  let username = "";
  let error_message: string = "";

  const register = async () => {
    try {
      error_message = "";

      let resp = await fetch(`${getBaseUrl()}/register`, {
        body: JSON.stringify({
          name: username,
        }),
        method: "POST",
        headers: {
          "content-type": "application/json",
          accept: "application/json",
        },
      }).then((res) => (res.status === 200 ? res.json() : Promise.reject()));

      sessionStorage.setItem("token", resp.token);
      window.location.reload();
    } catch (err) {
      console.log(err);
      error_message = "username is taken";
    }
  };
</script>

<div class="container">
  {#if error_message}
    <h1>{error_message}</h1>
  {/if}

  <div>
    <input type="text" placeholder="enter username" bind:value={username} />
    <button on:click={register}>login</button>
  </div>
</div>

<style>
  .container {
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    height: 100vh;
  }
</style>
