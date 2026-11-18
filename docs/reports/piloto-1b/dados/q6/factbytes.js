// Bytes each reviewed fact takes in the query sent (compact JSON of its entry).
const fs = require('fs');
const P = 'C:/Users/beatl/AppData/Local/Temp/claude/09139e964f85837f/c2d74a42-2372-4db9-9c84-8fc3b436eb57/scratchpad/q6/root/.studio/projects/healthchecks-25597e41/runs';
const q = JSON.parse(fs.readFileSync(P + '/20261118-205150/turns/01/claims/query.json'));
const rows = q.evidence.reviewed_facts.map(f => [f.id, Buffer.byteLength(JSON.stringify(f))]);
const group = ids => rows.filter(([id]) => ids.includes(id)).reduce((a, [, b]) => a + b, 0);
console.log(rows.map(r => r.join(' ')).join(', '));
console.log('P1 (F2,F4,F5,F6):', group(['F2', 'F4', 'F5', 'F6']), 'B; F26:', group(['F26']), 'B; F18-F25:', group(['F18', 'F19', 'F20', 'F21', 'F22', 'F23', 'F24', 'F25']), 'B');
console.log('F6 no pacote:', JSON.stringify(q.evidence.reviewed_facts.find(f => f.id === 'F6')).slice(0, 400));
