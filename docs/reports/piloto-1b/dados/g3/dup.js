const key = 'healthchecks-25597e41', run = '20261118-061046';
const sa = 'src:04fd8532fcf9ba2615bc6ac62bffcbaee859b92e9af51ab808de06275c96a4f1:97769054348c8f590cabd15a0e44140664305d224bad550e9548ec2e674770e9';
const vw = 'src:627a48233053768114a51eadccf97aadd78ac0fc450648e2b5da871ba305777e:d0121156422dcefcd2035b4c7f2222f23eba84765c7abf28520153d051ca673d';
(async () => {
  for (const [name, node] of [['views.ping', `${vw}:function:6860:9238`], ['sendalerts.notify', `${sa}:function:609:1968`]]) for (const depth of [3, 6]) {
    const c = await (await fetch('http://127.0.0.1:8781/api/calls', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ key, run, node, depth, chosen: {} }) })).json();
    const at = {}; for (const u of c.unresolved) (at[`${u.evidence.path}:${u.evidence.start_byte}`] ??= []).push(u.text.split('\n')[0].slice(0, 60));
    const same = Object.entries(at).filter(([, v]) => v.length > 1);
    const ids = new Set(c.unresolved.map(u => u.occurrence + '|' + u.from));
    const sorted = c.unresolved.every((u, i, a) => !i || (a[i - 1].evidence.path < u.evidence.path) || (a[i - 1].evidence.path === u.evidence.path && a[i - 1].evidence.start_byte <= u.evidence.start_byte));
    console.log(name, 'd' + depth, 'total', c.unresolved.length, 'mesma posição de início:', same.length, 'ocorrências repetidas:', c.unresolved.length - ids.size, 'ordenado:', sorted);
    for (const [k, v] of same.slice(0, 2)) console.log('   ', k, JSON.stringify(v));
  }
})();
