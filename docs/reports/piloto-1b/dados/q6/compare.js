// Q6: the new answer next to the original, with the send metadata.
const fs = require('fs');
const P = 'C:/Users/beatl/AppData/Local/Temp/claude/09139e964f85837f/c2d74a42-2372-4db9-9c84-8fc3b436eb57/scratchpad/q6/root/.studio/projects/healthchecks-25597e41/runs';
const sends = { original: P + '/20261118-061046/turns/14/sends/03-ollama/send.json', nova: P + '/20261118-205150/turns/01/sends/01-ollama/send.json' };
for (const [name, f] of Object.entries(sends)) {
  const s = JSON.parse(fs.readFileSync(f));
  console.log(`== ${name}: state ${s.state}; model ${s.model}; modelfile ${s.modelfile} ${s.modelfile_sha256.slice(0, 12)}; prompt_tokens ${s.input.prompt_tokens}; query_bytes ${s.input.query_bytes}; estimated ${s.input.estimated_tokens}; elapsed ${s.elapsed_ms} ms; checks ${s.link.checks.join(',')}; fact_ids ${s.link.selection.fact_ids.filter(x => x.startsWith('F')).join(' ')}; mechanical ${s.check.report.mechanical_status}; issues ${JSON.stringify(s.check.report.issues).slice(0, 300)}`);
  for (const c of s.check.claims) {
    const cites = c.citations.map(x => `${x.path.split('/').pop()}:${x.start_line}${x.end_line !== x.start_line ? '-' + x.end_line : ''}`).join(', ');
    const loc = c.location ? `${c.location.symbol_name}@${c.location.declaration_line} op ${c.location.operation_start_line}-${c.location.operation_end_line}` : '-';
    console.log(`  ${c.id} ${c.kind} [${loc}] ${c.text}\n     cita: ${cites}`);
    for (const x of c.citations) console.log(`       ${x.path.split('/').pop()}:${x.start_line} ${JSON.stringify(x.quote).slice(0, 140)}`);
  }
  if (s.check.resolutions?.length) console.log('  resoluções:', s.check.resolutions.map(r => `${r.claim_id} ${r.code} ${JSON.stringify(r.original)}→${r.start_line}-${r.end_line}`).join('; '));
}
