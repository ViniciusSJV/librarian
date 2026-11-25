// Passo 2.4d, Parada 1: por símbolo, todos os usos que o simulador dá (para a amostra de "quem usa B"), e contas extras.
//   node lista.js <f-depois> <pasta do simulador>
const fs=require('fs'),path=require('path');const {load,isTestPath}=require('../2.4/load.js');const [dump,sim]=process.argv.slice(2);
for(const key of ['healthchecks-25597e41','renderer-9b78f3a9']){const P=load(dump,key),{N,g,show,declared}=P;const inv=JSON.parse(fs.readFileSync(path.join(sim,`inv-${key}-novo.json`)));
const cands={};for(const e of g.edges)if(e.kind==='name_candidate')(cands[e.from]??=[]).push(e.to);
const at=c=>`${P.observer[c.occ].ev.path}:${P.observer[c.occ].ev.start_line}`;const one=c=>c.to.length===1&&c.res!=='homonym';
const k={};for(const c of inv)if(one(c)){const recv=N[c.occ].kind==='method_call_observed';const n=`${c.res} ${N[c.to[0]].kind}${recv?' com receptor':' sem receptor'}`;k[n]=(k[n]??0)+1}
console.log(key,'certos por resolução',k);
const un=inv.filter(c=>one(c)&&c.res==='unique'&&N[c.occ].kind==='method_call_observed'&&N[c.to[0]].kind==='method');console.log(' unique com receptor, método:',un.length,'produção:',un.filter(c=>!isTestPath(at(c))).length);
const self=inv.filter(c=>one(c)&&c.to[0]===c.from).length;console.log(' A == B (recursão):',self);
const want=key[0]==='h'?['Webhook.is_noop (','Discord.fix_asterisks (','RocketChat.fix_asterisks (','TokenBucket.authorize (','authorize (hc/api/dec','Telegram.send (','Check.ping (','EmailThread.__init__ (','Ping.get_body (','cors (','Signal.send (','HttpTransport.raise_for_response (','Transport.notify (','Slackalike.payload (','Shell.run_cmd','EmailThread ('] : ['impl World::color_at','impl Groups::intersect','Matrix<4>::inverse','impl Cylinder::new','impl Camera::ray_from','tests_camera::create_default_world','impl Equivalence<f64> for f64'];
for(const w of want)for(const id in N)if(N[id].category==='symbol'&&show(id).includes(w)){console.log('##',show(id));const occs=new Set();
for(const c of inv){let t=c.runs===id?'INSTANCIA':!c.to.includes(id)?null:one(c)?(c.rel==='decorates'?'DECORA':'CERTO'):c.basis==='type'?'POSSIVEL-tipo':'POSSIVEL-hom';if(t){occs.add(c.occ);console.log('  ',t,at(c),c.res,c.rule??'',c.to.length>1?'entre '+c.to.length:'',N[c.occ].label.replace(/\s+/g,' ').slice(0,60),'| de',N[c.from].label)}}
for(const o in cands)if(cands[o].includes(id)&&!occs.has(o)){const call=/call_observed/.test(N[o].kind);console.log('  ',call?'descartado':'NOME '+N[o].kind,at({occ:o}),N[o].label.replace(/\s+/g,' ').slice(0,50))}}
}
