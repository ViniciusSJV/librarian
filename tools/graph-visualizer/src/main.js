import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { forceSimulation, forceManyBody, forceLink, forceCenter, forceCollide } from 'd3-force-3d';

const COLORS={source:0x38bdf8,symbol:0xa78bfa,occurrence:0xfbbf24,chunk:0x34d399,default:0x94a3b8,edge:0x334155,trace:0xfb7185,e1:0xffffff,search:0x22d3ee};
const state={graph:null,trace:null,search:null,sources:[],symbols:[],chunks:[],nodes:[],edges:[],nodeById:new Map(),meshes:new Map(),edgeObjects:[],selected:null,searchMatches:new Set(),traceNodeLabels:new Set(),traceEdgeKeys:new Set(),e1NodeIds:new Set(),visibleKinds:new Set(['source','symbol','occurrence']),traceEnabled:true,e1Enabled:true};

const $=s=>document.querySelector(s);
const folderInput=$('#folderInput'),searchInput=$('#searchInput'),fitButton=$('#fitButton'),resetButton=$('#resetButton'),traceToggle=$('#traceToggle'),e1Toggle=$('#e1Toggle'),status=$('#status');
const nodeCount=$('#nodeCount'),edgeCount=$('#edgeCount'),traceCount=$('#traceCount'),e1Name=$('#e1Name'),selectedEmpty=$('#selectedEmpty'),selectedContent=$('#selectedContent'),selectedLabel=$('#selectedLabel'),selectedMeta=$('#selectedMeta');
const canvas=$('#scene'),viewport=$('.viewport');

const scene=new THREE.Scene(); scene.background=new THREE.Color(0x05070c); scene.fog=new THREE.FogExp2(0x05070c,.00155);
const camera=new THREE.PerspectiveCamera(55,1,.1,5000); camera.position.set(0,0,650);
const renderer=new THREE.WebGLRenderer({canvas,antialias:true,powerPreference:'high-performance'}); renderer.setPixelRatio(Math.min(devicePixelRatio,2));
const controls=new OrbitControls(camera,renderer.domElement); controls.enableDamping=true; controls.dampingFactor=.06; controls.minDistance=50; controls.maxDistance=2200;
scene.add(new THREE.AmbientLight(0xffffff,1.15)); const light=new THREE.PointLight(0xa7c7ff,1.4,1800); light.position.set(300,300,500); scene.add(light);
const graphGroup=new THREE.Group(); scene.add(graphGroup);
const raycaster=new THREE.Raycaster(),pointer=new THREE.Vector2(2,2);

const normalizePath=p=>p.replaceAll('\\','/');
const basename=p=>normalizePath(p).split('/').pop();
const parseJsonl=t=>t.split(/\r?\n/).map(x=>x.trim()).filter(Boolean).map((x,i)=>{try{return JSON.parse(x)}catch(e){throw new Error(`JSONL inválido na linha ${i+1}: ${e.message}`)}});

function sourceById(id){return state.sources.find(x=>x.id===id)}
function symbolById(id){return state.symbols.find(x=>x.id===id)}
function inferKind(n){const k=String(n?.kind??'').toLowerCase(),id=String(n?.id??'').toLowerCase(); if(k==='source'||(id.startsWith('src:')&&String(n?.label??'').endsWith('.rs')))return 'source'; if(k==='occurrence'||id.includes(':occurrence:'))return 'occurrence'; return 'symbol'}

function enrichNode(n){
  const src=sourceById(n.id),sym=symbolById(n.id);
  if(src)return {...n,nodeKind:'source',label:n.label??src.path,meta:src};
  if(sym)return {...n,nodeKind:'symbol',label:n.label??sym.qualified_name??sym.name,meta:sym};
  return {...n,nodeKind:inferKind(n),label:n.label??n.id,meta:n};
}

function clearScene(){
  for(const o of [...graphGroup.children]){graphGroup.remove(o);o.geometry?.dispose?.();if(Array.isArray(o.material))o.material.forEach(m=>m.dispose?.());else o.material?.dispose?.()}
  state.meshes.clear(); state.edgeObjects=[]; state.selected=null;
}

function addChunkNodes(nodes,edges){
  const existing=new Set(nodes.map(n=>n.id));
  for(const c of state.chunks){
    if(existing.has(c.id))continue;
    nodes.push({id:c.id,kind:'chunk',nodeKind:'chunk',label:`chunk ${c.start_line}-${c.end_line}`,meta:c});
    const owner=state.symbols.find(s=>s.chunk_id===c.id);
    if(owner&&existing.has(owner.id))edges.push({from:owner.id,to:c.id,kind:'evidence_chunk',resolution:'structural',evidence:{source_id:c.source_id,start_byte:c.start_byte,end_byte:c.end_byte,start_line:c.start_line,end_line:c.end_line,excerpt_sha256:c.sha256},synthetic:true});
  }
}

function layout(nodes,edges){
  const ns=nodes.map((n,i)=>({...n,x:Math.cos(i*.618)*10,y:Math.sin(i*.618)*10,z:Math.sin(i*1.13)*10}));
  const ids=new Set(ns.map(n=>n.id));
  const es=edges.filter(e=>ids.has(e.from)&&ids.has(e.to)).map(e=>({source:e.from,target:e.to}));
  const sim=forceSimulation(ns,3).force('charge',forceManyBody().strength(-55)).force('link',forceLink(es).id(d=>d.id).distance(48).strength(.42)).force('center',forceCenter(0,0,0)).force('collision',forceCollide().radius(d=>d.nodeKind==='source'?11:7).strength(.9)).stop();
  for(let i=0;i<320;i++)sim.tick();
  ns.forEach(n=>{n.x*=1.25;n.y*=1.25;n.z*=1.25}); return ns;
}

function buildTrace(){
  state.traceNodeLabels.clear(); state.traceEdgeKeys.clear();
  for(const r of state.trace?.relations??[]){if(r.from)state.traceNodeLabels.add(r.from);if(r.to)state.traceNodeLabels.add(r.to);state.traceEdgeKeys.add(`${r.from}|${r.kind}|${r.to}`)}
}

function resolveE1(){
  state.e1NodeIds.clear(); const hit=state.search?.hits?.[0]; if(!hit){e1Name.textContent='—';return} e1Name.textContent=hit.name??'E1';
  if(hit.symbol_id&&state.nodeById.has(hit.symbol_id))state.e1NodeIds.add(hit.symbol_id);
  for(const n of state.nodes){
    const lm=n.label===hit.name||n.meta?.qualified_name===hit.name||n.meta?.name===hit.name;
    const sm=!hit.path||n.meta?.path===hit.path||sourceById(n.meta?.source_id)?.path===hit.path;
    if(lm&&sm)state.e1NodeIds.add(n.id);
  }
  const sym=state.symbols.find(s=>s.chunk_id===hit.chunk_id); if(sym&&state.nodeById.has(sym.id))state.e1NodeIds.add(sym.id);
  if(hit.chunk_id&&state.nodeById.has(hit.chunk_id))state.e1NodeIds.add(hit.chunk_id);
}

function isTraceEdge(e){const a=state.nodeById.get(e.from)?.label,b=state.nodeById.get(e.to)?.label;return state.traceEdgeKeys.has(`${a}|${e.kind}|${b}`)}
function nodeRadius(k){return k==='source'?6.2:k==='chunk'?2.8:k==='occurrence'?3.2:4.4}
function baseColor(k){return COLORS[k]??COLORS.default}

function createNode(n){
  const g=new THREE.SphereGeometry(nodeRadius(n.nodeKind),14,14),c=baseColor(n.nodeKind);
  const m=new THREE.MeshStandardMaterial({color:c,emissive:c,emissiveIntensity:.18,roughness:.52,metalness:.1,transparent:true,opacity:.94});
  const mesh=new THREE.Mesh(g,m); mesh.position.set(n.x,n.y,n.z); mesh.userData={nodeId:n.id,baseScale:1,phase:Math.random()*Math.PI*2}; graphGroup.add(mesh); state.meshes.set(n.id,mesh);
}
function createEdge(e){
  const a=state.meshes.get(e.from),b=state.meshes.get(e.to); if(!a||!b)return;
  const trace=isTraceEdge(e),g=new THREE.BufferGeometry().setFromPoints([a.position.clone(),b.position.clone()]);
  const m=new THREE.LineBasicMaterial({color:trace&&state.traceEnabled?COLORS.trace:COLORS.edge,transparent:true,opacity:trace&&state.traceEnabled?.95:.2});
  const line=new THREE.Line(g,m); line.userData={edge:e,trace}; graphGroup.add(line); state.edgeObjects.push(line);
}

function refresh(){
  for(const n of state.nodes){
    const mesh=state.meshes.get(n.id); if(!mesh)continue; mesh.visible=state.visibleKinds.has(n.nodeKind);
    const e1=state.e1Enabled&&state.e1NodeIds.has(n.id),trace=state.traceEnabled&&state.traceNodeLabels.has(n.label),search=state.searchMatches.has(n.id),sel=state.selected===n.id;
    let c=baseColor(n.nodeKind),intensity=.18,scale=1;
    if(trace){c=COLORS.trace;intensity=.55;scale=1.12}
    if(search){c=COLORS.search;intensity=.75;scale=1.22}
    if(e1){c=COLORS.e1;intensity=1.15;scale=1.55}
    if(sel){intensity=1.4;scale=Math.max(scale,1.65)}
    mesh.material.color.setHex(c);mesh.material.emissive.setHex(c);mesh.material.emissiveIntensity=intensity;mesh.userData.baseScale=scale;
  }
  for(const line of state.edgeObjects){
    const e=line.userData.edge,a=state.nodeById.get(e.from),b=state.nodeById.get(e.to);
    line.visible=!!a&&!!b&&state.visibleKinds.has(a.nodeKind)&&state.visibleKinds.has(b.nodeKind);
    const tr=line.userData.trace&&state.traceEnabled;line.material.color.setHex(tr?COLORS.trace:COLORS.edge);line.material.opacity=tr?.95:.16;
  }
}

function fit(){
  const box=new THREE.Box3(); for(const m of state.meshes.values())if(m.visible)box.expandByObject(m); if(box.isEmpty())return;
  const s=box.getBoundingSphere(new THREE.Sphere()),d=Math.max(140,s.radius*2.6); controls.target.copy(s.center); camera.position.set(s.center.x,s.center.y,s.center.z+d);camera.far=d*8;camera.updateProjectionMatrix();controls.update();
}

function showNode(n){
  selectedEmpty.hidden=true;selectedContent.hidden=false;selectedLabel.textContent=n.label;selectedMeta.replaceChildren();
  const src=sourceById(n.meta?.source_id);
  const rows=[['tipo',n.nodeKind],['kind',n.kind??n.meta?.kind],['id',n.id],['arquivo',n.meta?.path??src?.path],['nome',n.meta?.name],['qualified',n.meta?.qualified_name],['linhas',n.meta?.start_line!=null?`${n.meta.start_line}-${n.meta.end_line}`:undefined],['sha256',n.meta?.sha256]].filter(([,v])=>v!=null&&v!=='');
  for(const [k,v] of rows){const dt=document.createElement('dt'),dd=document.createElement('dd');dt.textContent=k;dd.textContent=String(v);selectedMeta.append(dt,dd)}
}

async function loadRun(files){
  status.textContent='Lendo artefatos...';
  const map=new Map(files.map(f=>[normalizePath(f.webkitRelativePath||f.name),f]));
  const find=(...names)=>{for(const [p,f] of map)if(names.some(n=>p===n||p.endsWith('/'+n)||basename(p)===n))return f;return null};
  const gf=find('graph.json'); if(!gf)throw new Error('graph.json não encontrado.');
  const tf=find('graph-trace.json'),sf=find('search.json'),sof=find('edition/sources.jsonl','sources.jsonl'),syf=find('edition/symbols.jsonl','symbols.jsonl'),chf=find('edition/chunks.jsonl','chunks.jsonl');

  state.graph=JSON.parse(await gf.text());state.trace=tf?JSON.parse(await tf.text()):null;state.search=sf?JSON.parse(await sf.text()):null;
  state.sources=sof?parseJsonl(await sof.text()):[];state.symbols=syf?parseJsonl(await syf.text()):[];state.chunks=chf?parseJsonl(await chf.text()):[];

  const nodes=(state.graph.nodes??[]).map(enrichNode),edges=[...(state.graph.edges??[])]; addChunkNodes(nodes,edges);
  const laid=layout(nodes,edges); state.nodes=laid;state.edges=edges;state.nodeById=new Map(laid.map(n=>[n.id,n]));
  buildTrace();resolveE1();clearScene();state.nodeById=new Map(laid.map(n=>[n.id,n])); for(const n of laid)createNode(n);for(const e of edges)createEdge(e);
  nodeCount.textContent=laid.length;edgeCount.textContent=edges.length;traceCount.textContent=state.trace?.relations?.length??0;
  searchInput.disabled=false;fitButton.disabled=false;resetButton.disabled=false;refresh();fit();
  status.textContent=`Carregado: ${laid.length} nós, ${edges.length} arestas${state.trace?`, ${state.trace.relations.length} relações no trace`:''}.`;
}

folderInput.addEventListener('change',async e=>{try{await loadRun([...e.target.files])}catch(err){console.error(err);status.textContent='Erro: '+err.message}});
searchInput.addEventListener('input',e=>{state.searchMatches.clear();const q=e.target.value.trim().toLowerCase();if(q)for(const n of state.nodes){const h=[n.label,n.kind,n.meta?.name,n.meta?.qualified_name,n.meta?.path].filter(Boolean).join(' ').toLowerCase();if(h.includes(q))state.searchMatches.add(n.id)}refresh()});
fitButton.addEventListener('click',fit);resetButton.addEventListener('click',()=>{state.searchMatches.clear();state.selected=null;searchInput.value='';selectedContent.hidden=true;selectedEmpty.hidden=false;refresh()});
traceToggle.addEventListener('change',e=>{state.traceEnabled=e.target.checked;refresh()});e1Toggle.addEventListener('change',e=>{state.e1Enabled=e.target.checked;refresh()});
document.querySelectorAll('[data-kind]').forEach(cb=>cb.addEventListener('change',e=>{const k=e.target.dataset.kind;e.target.checked?state.visibleKinds.add(k):state.visibleKinds.delete(k);refresh()}));

renderer.domElement.addEventListener('pointermove',e=>{const r=renderer.domElement.getBoundingClientRect();pointer.x=((e.clientX-r.left)/r.width)*2-1;pointer.y=-((e.clientY-r.top)/r.height)*2+1});
renderer.domElement.addEventListener('click',()=>{raycaster.setFromCamera(pointer,camera);const hit=raycaster.intersectObjects([...state.meshes.values()].filter(m=>m.visible),false)[0];if(!hit)return;const id=hit.object.userData.nodeId,n=state.nodeById.get(id);if(!n)return;state.selected=id;showNode(n);refresh()});

function resize(){const w=viewport.clientWidth,h=viewport.clientHeight;renderer.setSize(w,h,false);camera.aspect=w/Math.max(1,h);camera.updateProjectionMatrix()}
window.addEventListener('resize',resize);resize();

const clock=new THREE.Clock();
(function animate(){requestAnimationFrame(animate);const t=clock.getElapsedTime();controls.update();graphGroup.rotation.y=Math.sin(t*.08)*.035;graphGroup.rotation.x=Math.cos(t*.06)*.02;for(const m of state.meshes.values())if(m.visible){const p=1+Math.sin(t*1.8+m.userData.phase)*.025;m.scale.setScalar((m.userData.baseScale??1)*p)}renderer.render(scene,camera)})();
