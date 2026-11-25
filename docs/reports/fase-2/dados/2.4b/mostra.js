// Respostas do /api/calls num dump do sintetico.   node mostra.js <dump-dir>
const fs=require('fs'),path=require('path');const d=process.argv[2];
const g=JSON.parse(fs.readFileSync(path.join(d,'sint.graph.json'))),r=JSON.parse(fs.readFileSync(path.join(d,'sint.json'))).res;
for(const [id,x] of Object.entries(r)){const by={};for(const l of x.links)(by[l[0]]??={res:l[3],n:l[7],rule:l[8],runs:l[6],rel:l[4],to:[]}).to.push(g.nodes[l[2]].label);for(const u of x.unresolved)by[u[0]]={res:u[2],n:u[4],to:[]};
for(const [o,y] of Object.entries(by))console.log(g.nodes[id].label,'|',g.nodes[o].label,'|',y.res,y.n,y.rule??'','|',y.to.sort().join(', '),y.runs?'runs '+g.nodes[y.runs].label:'')}
