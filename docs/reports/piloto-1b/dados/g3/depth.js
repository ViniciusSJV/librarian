// A2/A3: unresolved at depth 3 and 6 for views.ping and sendalerts.notify: totals, by reason,
// repeated locations, and whether the list comes in (path, line, column) order.
const key = 'healthchecks-25597e41', run = '20261118-061046';
const sendalerts = 'src:04fd8532fcf9ba2615bc6ac62bffcbaee859b92e9af51ab808de06275c96a4f1:97769054348c8f590cabd15a0e44140664305d224bad550e9548ec2e674770e9';
const views = 'src:627a48233053768114a51eadccf97aadd78ac0fc450648e2b5da871ba305777e:d0121156422dcefcd2035b4c7f2222f23eba84765c7abf28520153d051ca673d';
const seeds = { 'views.ping': `${views}:function:6860:9238`, 'sendalerts.notify': `${sendalerts}:function:609:1968` };
const port = process.argv[2];
(async () => {
  for (const [name, node] of Object.entries(seeds)) for (const depth of [1, 3, 6]) {
    const r = await fetch(`http://127.0.0.1:${port}/api/calls`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ key, run, node, depth, chosen: {} }) });
    const c = await r.json();
    const u = c.unresolved, by = {};
    for (const x of u) by[x.reason] = (by[x.reason] ?? 0) + 1;
    const loc = x => `${x.evidence.path}:${x.evidence.start_line}:${x.evidence.start_column ?? x.evidence.column ?? '?'}`;
    const seen = {}; for (const x of u) seen[loc(x)] = (seen[loc(x)] ?? 0) + 1;
    const rep = Object.entries(seen).filter(([, n]) => n > 1);
    const occ = {}; for (const x of u) occ[x.occurrence] = (occ[x.occurrence] ?? 0) + 1;
    const repOcc = Object.entries(occ).filter(([, n]) => n > 1);
    const tuple = x => [x.evidence.path, x.evidence.start_line, x.evidence.start_byte];
    const cmp = (a, b) => { const [p, l, k] = tuple(a), [q, m, j] = tuple(b); return p < q ? -1 : p > q ? 1 : l - m || k - j; };
    let firstBreak = null;
    for (let i = 1; i < u.length; i++) if (cmp(u[i - 1], u[i]) > 0) { firstBreak = `${loc(u[i - 1])} antes de ${loc(u[i])}`; break; }
    // Same call twice? ask twice and compare.
    const r2 = await (await fetch(`http://127.0.0.1:${port}/api/calls`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ key, run, node, depth, chosen: {} }) })).json();
    console.log(JSON.stringify({ seed: name, depth, nodes: c.nodes.length, links: c.links.length, truncated: c.truncated, total: u.length, by, repeated_locations: rep.length, repeated_examples: rep.slice(0, 3), repeated_occurrence_ids: repOcc.length, sorted_path_line_col: !firstBreak, first_break: firstBreak, same_order_on_repeat: JSON.stringify(r2.unresolved) === JSON.stringify(u) }));
  }
  if (process.argv[3]) { const c = await (await fetch(`http://127.0.0.1:${port}/api/calls`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ key, run, node: seeds['views.ping'], depth: 1, chosen: {} }) })).json(); console.log(JSON.stringify(c.unresolved[0].evidence)); console.log(c.unresolved.slice(0, 8).map(x => `${x.evidence.path}:${x.evidence.start_line} ${x.occurrence.split(':').slice(-3).join(':')}`).join('\n')); }
})();
