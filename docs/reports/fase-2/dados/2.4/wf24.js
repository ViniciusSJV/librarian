// Passo 2.4, Parada 1 (diagnóstico). Para os símbolos dos 4 workflows do Healthchecks (PIL, seção 3),
// cada chamada escrita, a resposta de hoje e a do 2.4 simulado (py.json).
'use strict';
const { load } = require('./load.js');
const py = require('./py.json');
const P = load(process.argv[2] ?? '../dump', 'healthchecks-25597e41');
const WF = {
  A: [['hc/api/views.py', 'checks'], ['hc/api/views.py', 'create_check'], ['hc/api/views.py', '_lookup'], ['hc/api/views.py', '_update'], ['hc/api/decorators.py', 'authorize'], ['hc/api/decorators.py', 'authorize.wrapper'], ['hc/api/models.py', 'Check.going_down_after'], ['hc/api/models.py', 'Check.get_grace_start']],
  B: [['hc/api/views.py', 'ping_by_slug'], ['hc/api/views.py', 'ping'], ['hc/api/models.py', 'Check.ping'], ['hc/api/models.py', 'Check.create_flip']],
  C: [['hc/api/management/commands/sendalerts.py', 'Command.handle'], ['hc/api/management/commands/sendalerts.py', 'Command.handle_going_down'], ['hc/api/models.py', 'Check.get_status']],
  D: [['hc/api/management/commands/sendalerts.py', 'Command.process_one_flip'], ['hc/api/management/commands/sendalerts.py', 'notify'], ['hc/api/models.py', 'Flip.select_channels'], ['hc/api/models.py', 'Channel.notify'], ['hc/api/models.py', 'Channel.transport'], ['hc/api/transports.py', 'Transport.notify'], ['hc/api/transports.py', 'Transport.is_noop'], ['hc/integrations/slack/transport.py', 'Slackalike.notify'], ['hc/integrations/discord/transport.py', 'Discord.notify'], ['hc/api/transports.py', 'HttpTransport.post'], ['hc/api/transports.py', 'HttpTransport.request']],
};
const sim = new Map();
for (const x of [...py.n0, ...py.n5]) sim.set(`${x.at}|${x.text.slice(0, 40)}`, x);
const sym = Object.entries(P.N).filter(([id, n]) => n.category === 'symbol' && P.declared[id]);
for (const [wf, list] of Object.entries(WF)) {
  console.log(`\n### ${wf}`);
  for (const [file, label] of list) {
    const ids = sym.filter(([id, n]) => n.label === label && P.declared[id].path === file).map(([id]) => id);
    if (!ids.length) { console.log(`(não achado: ${label})`); continue; }
    for (const id of ids) {
      console.log(`- ${label} (${P.where(id)})`);
      const cs = [...P.calls.values()].filter(c => c.owner === id || P.parent[c.owner] === id).sort((a, b) => a.sb - b.sb);
      for (const c of cs) {
        const s = sim.get(`${c.path}:${c.line}|${c.text.slice(0, 40)}`);
        const t = c.targets.map(P.show).join('; ');
        console.log(`    ${c.line} ${c.text.replace(/\s+/g, ' ').slice(0, 60)} | ${c.res}${c.rel && c.rel !== 'calls' ? '/' + c.rel : ''} ${t ? '→ ' + t.slice(0, 90) : '(' + (c.cands ?? 0) + ')'}${s && !/^igual|confirma/.test(s.mudanca) ? ' || 2.4: ' + s.mudanca + ' ' + (s.after ?? '') : ''}`);
      }
    }
  }
}
