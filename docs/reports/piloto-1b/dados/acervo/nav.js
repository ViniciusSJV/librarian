// The navigation links the pilot report marks [grafo] on the way to the facts' declarations.
const key = 'healthchecks-25597e41', run = '20261118-061046';
const api = async (m, p, b) => (await fetch('http://127.0.0.1:8781' + p, { method: m, headers: { 'content-type': 'application/json' }, body: b && JSON.stringify(b) })).json();
const want = { 'create_check': ['_update', '_lookup'], 'Command.handle': ['handle_going_down', 'process_one_flip'], 'Check.ping': ['going_down_after', 'create_flip'], 'ping_by_slug': ['ping'], 'notify': ['select_channels', 'update_next_nag_dates'], 'Command.handle_going_down': ['get_status', 'going_down_after'], 'checks': ['create_check'] };
(async () => {
  const g = await api('GET', `/api/graph?key=${key}&run=${run}`);
  const declared = {}; for (const e of g.edges) if (e.kind === 'declares') declared[e.to] = e.evidence;
  for (const [from, names] of Object.entries(want)) {
    const seeds = Object.values(g.nodes).filter(n => n.category === 'symbol' && n.label === from && /function|method/.test(n.kind) && !/tests?\//.test(declared[n.id].path));
    for (const s of seeds) {
      const c = await api('POST', '/api/calls', { key, run, node: s.id, depth: 1, chosen: {} });
      for (const name of names) {
        const ls = c.links.filter(l => l.text.split('(')[0].split('.').pop() === name);
        const occ = [...new Set(ls.map(l => l.text.split('\n')[0].slice(0, 40) + ' ' + l.resolution + '(' + l.candidates + ')'))];
        console.log(`${from} (${declared[s.id].path}) -> ${name}: ${occ.join('; ') || 'sem link'} => ${[...new Set(ls.map(l => g.nodes[l.to].label + '@' + declared[l.to].path))].join(' | ')}`);
      }
    }
  }
  // @authorize as name_reference, and include(uuid_urls).
  for (const label of ['authorize', 'uuid_urls']) {
    const target = Object.values(g.nodes).find(n => n.category === 'symbol' && n.label === label && /hc\/api\//.test(declared[n.id].path));
    const refs = g.edges.filter(e => e.kind === 'name_candidate' && e.to === target.id).map(e => g.nodes[e.from]);
    console.log(label, 'referências por nome:', refs.length, [...new Set(refs.map(r => r.kind))].join(','));
  }
})();
