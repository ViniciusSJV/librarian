// G3 real validation: /api/calls on the Healthchecks copy, new binary vs the binary before P1/G3.
// Usage: node calls.js <newPort> <oldPort> <out.json>
const fs = require('fs');
const [newPort, oldPort, out] = process.argv.slice(2);
const key = 'healthchecks-25597e41', run = '20261118-061046';
const models = 'src:8ce3e55438349ca1cfd362c1daa62b1f586496f894fd70587817115292f75e4d:d03082adb2aacfd44f96bb0df8339a145b4979fee4277225c3a4b8612d1493e6';
const sendalerts = 'src:04fd8532fcf9ba2615bc6ac62bffcbaee859b92e9af51ab808de06275c96a4f1:97769054348c8f590cabd15a0e44140664305d224bad550e9548ec2e674770e9';
const views = 'src:627a48233053768114a51eadccf97aadd78ac0fc450648e2b5da871ba305777e:d0121156422dcefcd2035b4c7f2222f23eba84765c7abf28520153d051ca673d';
const seeds = {
  'sendalerts.notify': `${sendalerts}:function:609:1968`,
  'process_one_flip': `${sendalerts}:method:3008:4196`,
  'views.ping': `${views}:function:6860:9238`,
  'Check.ping': `${models}:method:19528:22942`,
  'Channel.notify': `${models}:method:43568:44760`,
};
const call = async (port, body) => {
  const r = await fetch(`http://127.0.0.1:${port}/api/calls`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });
  const text = await r.text();
  if (!r.ok) throw new Error(`${port}: ${r.status} ${text.slice(0, 200)}`);
  return JSON.parse(text);
};
(async () => {
  const report = {};
  for (const [name, node] of Object.entries(seeds)) {
    for (const depth of [1, 3, 6]) {
      const body = { key, run, node, depth, chosen: {} };
      const [now, before] = await Promise.all([call(newPort, body), call(oldPort, body)]);
      const { unresolved, ...rest } = now;
      const same = JSON.stringify(rest) === JSON.stringify(before);
      const count = {};
      for (const u of unresolved) count[u.reason] = (count[u.reason] ?? 0) + 1;
      const row = { links: now.links.length, nodes: now.nodes.length, truncated: now.truncated, compatible_with_before: same, unresolved: count };
      if (depth === 1) row.samples = unresolved.slice(0, 40).map(u => `${u.reason} ${u.candidates} ${u.text} @${u.evidence.path}:${u.evidence.start_line}`);
      const limited = unresolved.filter(u => u.reason === 'node_limit');
      if (limited.length) row.node_limit_samples = limited.slice(0, 5).map(u => ({ text: u.text, resolution: u.resolution, candidates: u.candidates, to: u.to?.split(':').slice(-3).join(':'), at: `${u.evidence.path}:${u.evidence.start_line}` }));
      // Sanity: no unresolved occurrence has a link, and node_limit entries name a link of a full walk.
      const linked = new Set(now.links.map(l => `${l.occurrence}→${l.to}`));
      row.no_overlap = unresolved.every(u => u.reason === 'node_limit' ? !linked.has(`${u.occurrence}→${u.to}`) : !now.links.some(l => l.occurrence === u.occurrence));
      report[`${name} d${depth}`] = row;
    }
  }
  fs.writeFileSync(out, JSON.stringify(report, null, 2));
  for (const [k, v] of Object.entries(report)) console.log(k, JSON.stringify({ links: v.links, nodes: v.nodes, truncated: v.truncated, compatible: v.compatible_with_before, no_overlap: v.no_overlap, unresolved: v.unresolved }));
  console.log('\nsendalerts.notify d1 samples:\n' + report['sendalerts.notify d1'].samples.join('\n'));
  console.log('\nviews.ping d1 samples:\n' + report['views.ping d1'].samples.join('\n'));
  for (const [k, v] of Object.entries(report)) if (v.node_limit_samples) { console.log(`\n${k} node_limit:`, JSON.stringify(v.node_limit_samples, null, 1)); break; }
})().catch(e => { console.error(e); process.exit(1); });
