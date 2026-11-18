// Part B: what the operation inventory (the second resolution rule) follows from each case's loop.
const [port, root] = process.argv.slice(2);
const api = async (m, p, b) => { const r = await fetch(`http://127.0.0.1:${port}${p}`, { method: m, headers: { 'content-type': 'application/json' }, body: b && JSON.stringify(b) }); const t = await r.text(); if (!r.ok) throw new Error(p + ' ' + t); return JSON.parse(t); };
(async () => {
  const { key } = await api('POST', '/api/project', { root });
  const { id: run } = await api('POST', '/api/generate', { key });
  const inv = await api('GET', `/api/inventory?key=${key}&run=${run}`);
  const i = inv.inventory ?? inv;
  for (const l of i.loops.sort((a, b) => a.symbol_name.localeCompare(b.symbol_name))) {
    console.log(`== ${l.symbol_name} L${l.line} ${l.head}`);
    const firsts = new Set();
    for (const c of l.through_calls) { const s = c.steps[0]; firsts.add(`  ${s.text} -> ${s.to_name} [${s.resolution}${s.receiver_types?.length ? ' ' + s.receiver_types.join(',') : ''}]`); }
    for (const f of firsts) console.log(f);
    for (const u of l.unfollowed) console.log(`  NÃO SEGUIDA ${u.text} [${u.candidates} candidatos]`);
  }
})().catch(e => { console.error(e.message); process.exit(1); });
