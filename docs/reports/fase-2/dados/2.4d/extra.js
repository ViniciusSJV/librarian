// Passo 2.4d, Parada 1: contas extras (envelope, lacuna acima do teto, edition antiga, amostra validada contra os três tipos).
//   node extra.js <f-depois> <pasta do simulador>
const fs=require('fs'),path=require('path');const {load,isTestPath}=require('../2.4/load.js');const [dump,sim]=process.argv.slice(2);
const FX='../../../../../tools/precisao/fixtures/';
for(const [key,am] of [['healthchecks-25597e41','amostra-hc.json'],['renderer-9b78f3a9','amostra-rt.json']]){const P=load(dump,key),{N,g,show,declared}=P;const inv=JSON.parse(fs.readFileSync(path.join(sim,`inv-${key}-novo.json`)));
console.log('##',key);const at=o=>P.observer[o].ev;const one=c=>c.to.length===1&&c.res!=='homonym';const by=new Map(inv.map(c=>[c.occ,c]));
// Envelope: chamadores por nome com 1 candidato (funções e métodos)
const ed={};for(const e of g.edges)if(e.kind==='name_candidate')(ed[e.from]??=[]).push(e.to);
let hoje=0,ok=0,desc=0;const cert=new Set();for(const c of inv)if(one(c)&&c.rel==='calls'&&/method|function/.test(N[c.to[0]].kind))cert.add(c.occ);
let ganha=0;for(const c of inv){const e=ed[c.occ]??[];if(e.length===1&&/method|function/.test(N[e[0]].kind)){hoje++;if(one(c)&&c.to[0]===e[0])ok++;else desc++;}else if(cert.has(c.occ))ganha++;}
console.log('envelope "caller" (1 candidato por nome): ocorrências',hoje,'; a regra confirma',ok,'; a regra nega',desc,'; chamadas que a regra liga e o envelope não vê como chamador',ganha);
// Lacuna
const ac=inv.filter(c=>c.res==='above_candidate_limit');console.log('acima do teto:',ac.length,'produção',ac.filter(c=>!isTestPath(at(c.occ).path)).length);
for(const n of ['notify','add','is_noop','transform','Object::from']){const l=ac.filter(c=>N[c.occ].label.includes(n+'('));if(l.length)console.log('  ',n,l.length,'produção:',l.filter(c=>!isTestPath(at(c.occ).path)).map(c=>at(c.occ).path+':'+at(c.occ).start_line).slice(0,12).join(', '))}
// Edition antiga
const old=JSON.parse(fs.readFileSync(path.join(sim,`inv-${key}-velho.json`)));const s=l=>({chamadas:l.length,certas:l.filter(one).length,ambiguas:l.filter(c=>c.res==='homonym').length,acima:l.filter(c=>c.res==='above_candidate_limit').length,sem:l.filter(c=>c.res==='no_candidate_in_project').length,runs:l.filter(c=>c.runs).length});console.log('novo (V9)',s(inv),'\nantigo (V8)',s(old));
// Amostra
const A=JSON.parse(fs.readFileSync(FX+am));const list=Array.isArray(A)?A:A.chamadas??A.amostra??Object.values(A).find(Array.isArray);
const r={};const falt=[];for(const x of list){const v=x.verdade;const alvo=v?.alvo??null;if(!alvo||typeof alvo!=='string')continue;const o=inv.find(c=>`${at(c.occ).path}@${at(c.occ).start_byte}-${at(c.occ).end_byte}`===x.ocorrencia);if(!o){r.naoachada=(r.naoachada??0)+1;continue}
const B=Object.keys(N).find(i=>N[i].category==='symbol'&&show(i)===alvo);const k=!B?'alvo fora':one(o)&&o.to[0]===B?'certo':o.to.includes(B)?'possível':o.res==='above_candidate_limit'?'lacuna':'em nenhum';r[k]=(r[k]??0)+1;if(k==='em nenhum')falt.push(x.id)}
console.log('amostra com alvo no projeto: onde B aparece',r,falt)

}
