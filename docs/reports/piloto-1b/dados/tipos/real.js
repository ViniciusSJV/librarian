// B4/B5 on the Healthchecks copy: what the graph records inside notify and process_one_flip.
const key = 'healthchecks-25597e41', run = '20261118-061046', port = process.argv[2];
const sendalerts = 'src:04fd8532fcf9ba2615bc6ac62bffcbaee859b92e9af51ab808de06275c96a4f1:97769054348c8f590cabd15a0e44140664305d224bad550e9548ec2e674770e9';
(async () => {
  const g = await (await fetch(`http://127.0.0.1:${port}/api/graph?key=${key}&run=${run}`)).json();
  const by = {}; for (const e of g.edges) (by[e.from] ??= []).push(e);
  const parent = {}; for (const e of g.edges) if (e.kind === 'contains') parent[e.to] = e.from;
  for (const seed of [`${sendalerts}:function:609:1968`, `${sendalerts}:method:3008:4196`]) {
    console.log(`== ${g.nodes[seed].label}`);
    for (const e of by[seed].filter(e => e.kind === 'observes')) {
      const o = g.nodes[e.to];
      if (!['method_call_observed', 'call_observed', 'name_reference', 'type_reference', 'parameter', 'local_binding', 'loop_iterable', 'declared_return'].includes(o.kind)) continue;
      const c = (by[o.id] ?? []).filter(x => x.kind === 'name_candidate').map(x => `${g.nodes[x.to].label}@${x.evidence && g.nodes[x.to].kind}`);
      console.log(`  L${e.evidence.start_line} ${o.kind.padEnd(21)} ${JSON.stringify(o.label).slice(0, 45).padEnd(45)} ${o.ambiguous_candidates ? 'acima do teto ' + o.ambiguous_candidates : c.join(' | ').slice(0, 150) || '-'}`);
    }
  }
  const sel = Object.values(g.nodes).filter(n => n.label === 'Flip.select_channels');
  for (const s of sel) for (const e of by[s.id].filter(e => e.kind === 'observes')) { const o = g.nodes[e.to]; if (['declared_return', 'type_reference'].includes(o.kind) && e.evidence.start_line < 1395) console.log(`  select_channels L${e.evidence.start_line} ${o.kind} ${o.label} -> ${(by[o.id] ?? []).filter(x => x.kind === 'name_candidate').map(x => g.nodes[x.to].label).join(' | ') || '-'}`); }
})();
