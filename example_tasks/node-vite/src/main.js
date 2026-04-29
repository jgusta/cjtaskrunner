const app = document.querySelector("#app");
const apiBase = import.meta.env.VITE_API_BASE || "unset";

app.innerHTML = `
  <main>
    <h1>CJTasks Vite Example</h1>
    <p>API base: ${apiBase}</p>
  </main>
`;
