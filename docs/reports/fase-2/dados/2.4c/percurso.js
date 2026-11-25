// Percurso de profundidade 6, com as fontes registradas na saida.   node percurso.js
const L = require("C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/lib.js"), path = require("path");
(async () => {
  const bin = L.prepareBinaries(path.join(L.REPO, "target/release"));
  console.log(L.binaryLine(bin));
  const key = "healthchecks-25597e41";
  await L.withStudio({ studio: path.join(L.REPO, ".studio"), bin: bin.pasta, keys: [key] }, async ({ api, root }) => {
    const { id: run } = await api("POST", "/api/generate", { key }), G = L.loadGraph(root, key, run);
    const find = l => Object.keys(G.g.nodes).find(i => G.g.nodes[i].category == "symbol" && G.g.nodes[i].label == l);
    for (const l of ["Channel.notify", "Slackalike.notify", "HttpTransport._request", "Slackalike.payload", "Discord.notify"]) {
      const c = await api("POST", "/api/calls", { key, run, node: find(l), depth: 6, chosen: {} });
      console.log(l, c.nodes.length, "nos, truncado", c.truncated);
    }
  });
})();
