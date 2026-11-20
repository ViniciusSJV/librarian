// Shared pieces of the precision gate: arguments, the studio on a copy of .studio, hashes and the
// call pool. Nothing here writes to the original .studio: the studio only ever sees a copy.
'use strict';
const fs = require('fs'), path = require('path'), os = require('os'), crypto = require('crypto'), net = require('net');
const { spawn, execFileSync } = require('child_process');

const REPO = path.resolve(__dirname, '../..');
const DEFAULTS = { hc: 'healthchecks-25597e41/20261118-061046', rt: 'renderer-9b78f3a9/20261118-020654' };

/** `--name value` pairs and bare `--flag`s. */
function parseArgs(argv) {
  const out = {};
  for (let i = 0; i < argv.length; i++) {
    if (!argv[i].startsWith('--')) throw new Error(`argumento inesperado: ${argv[i]}`);
    const name = argv[i].slice(2), next = argv[i + 1];
    if (next === undefined || next.startsWith('--')) out[name] = true; else { out[name] = next; i++; }
  }
  return out;
}

/** Aggregate sha256 of every file under .studio/projects/<key>, as the recorded command computes it:
 *  lines "hash  projects/<key>/..." sorted by bytes, one per file, then the sha256 of that list. */
function hashProject(studio, key) {
  const lines = [];
  const walk = (dir) => {
    for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, e.name);
      if (e.isDirectory()) walk(full);
      else if (e.isFile()) {
        const rel = path.relative(studio, full).split(path.sep).join('/');
        lines.push(`${crypto.createHash('sha256').update(fs.readFileSync(full)).digest('hex')}  ${rel}`);
      }
    }
  };
  walk(path.join(studio, 'projects', key));
  lines.sort((a, b) => Buffer.compare(Buffer.from(a.slice(66)), Buffer.from(b.slice(66))));
  return { files: lines.length, sha256: crypto.createHash('sha256').update(lines.map(l => l + '\n').join('')).digest('hex') };
}

function freePort() {
  return new Promise((resolve, reject) => {
    const s = net.createServer().listen(0, '127.0.0.1', () => { const { port } = s.address(); s.close(() => resolve(port)); }).on('error', reject);
  });
}

/** A studio over `root` (a folder holding `.studio`), on a free port. Returns { api, stop }. */
async function startStudio(bin, root) {
  const exe = path.join(bin, process.platform === 'win32' ? 'librarian_studio.exe' : 'librarian_studio');
  if (!fs.existsSync(exe)) throw new Error(`binário não encontrado: ${exe}`);
  const port = await freePort();
  const child = spawn(exe, ['--port', String(port)], { env: { ...process.env, LIBRARIAN_ROOT: root }, stdio: ['ignore', 'pipe', 'pipe'] });
  let log = '';
  child.stdout.on('data', d => { log += d; }); child.stderr.on('data', d => { log += d; });
  const base = `http://127.0.0.1:${port}`;
  const api = async (method, p, body) => {
    const r = await fetch(base + p, { method, headers: { 'content-type': 'application/json' }, body: body && JSON.stringify(body) });
    const text = await r.text();
    if (!r.ok) throw new Error(`${method} ${p}: ${r.status} ${text.slice(0, 300)}`);
    return JSON.parse(text);
  };
  // Ready when it accepts connections (/api/status also needs the Modelfiles, absent from the copy).
  const accepts = () => new Promise((resolve, reject) => { const c = net.connect(port, '127.0.0.1', () => { c.end(); resolve(); }).on('error', reject); });
  for (let i = 0; ; i++) {
    try { await accepts(); break; } catch (e) {
      if (child.exitCode !== null || i > 100) { child.kill(); throw new Error(`o studio não respondeu: ${log}`); }
      await new Promise(r => setTimeout(r, 100));
    }
  }
  return { api, port, stop: () => child.kill() };
}

/** A temporary root holding a copy of the named projects of `studio`. */
function copyStudio(studio, keys) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'librarian-precisao-'));
  for (const k of keys) fs.cpSync(path.join(studio, 'projects', k), path.join(root, '.studio', 'projects', k), { recursive: true });
  return root;
}

/** The graph of a run, read from the copy, with the indexes every measure uses. */
function loadGraph(root, key, run) {
  const g = JSON.parse(fs.readFileSync(path.join(root, '.studio', 'projects', key, 'runs', run, 'graph-v2.json'), 'utf8'));
  const declared = {}, parent = {};
  for (const e of g.edges) {
    if (e.kind === 'declares') declared[e.to] = e.evidence;
    else if (e.kind === 'contains') parent[e.to] = e.from;
  }
  const where = id => declared[id] ? `${declared[id].path}:${declared[id].start_line}` : '?';
  return { g, declared, parent, where };
}

/** Stable key of an occurrence: path and byte span, the same in any run of the same source. */
const occKey = ev => `${ev.path}@${ev.start_byte}-${ev.end_byte}`;

/** Part C's pool: object.method() calls of functions and methods outside tests, without self.m(). */
function pool(G) {
  const out = [];
  for (const e of G.g.edges) {
    if (e.kind !== 'observes') continue;
    const o = G.g.nodes[e.to], owner = G.g.nodes[e.from];
    if (o.kind !== 'method_call_observed' || !/function|method/.test(owner.kind)) continue;
    if (/(^|\/)tests?\/|test_|_test\.|\/benches\//.test(e.evidence.path) || owner.label.includes('test')) continue;
    if (!/^[A-Za-z_][\w.]*\.\w+\(/.test(o.label) || /^self\.\w+\(/.test(o.label)) continue;
    out.push({ edge: e, occ: o, owner, key: occKey(e.evidence), h: crypto.createHash('sha256').update(o.id).digest('hex') });
  }
  return out;
}

/** What "what does it call?" answers for each occurrence, asking once per owner. */
async function answers(api, key, run, G, owners) {
  const out = new Map();
  for (const [owner, occs] of owners) {
    const c = await api('POST', '/api/calls', { key, run, node: owner, depth: 1, chosen: {} });
    for (const id of occs) {
      const u = c.unresolved.find(x => x.occurrence === id && x.reason !== 'node_limit');
      const ls = c.links.filter(x => x.occurrence === id);
      const cut = c.unresolved.filter(x => x.occurrence === id && x.reason === 'node_limit');
      const targets = [...ls, ...cut].map(l => ({ label: G.g.nodes[l.to].label, kind: G.g.nodes[l.to].kind, at: G.where(l.to) }));
      // `relation` (step 2.3a, DEC-39): `instantiates` or `calls`; absent before, read as `calls`.
      out.set(id, u ? { resolution: u.reason, candidates: u.candidates, targets: [], relation: null }
        : { resolution: (ls[0] ?? cut[0]).resolution, candidates: (ls[0] ?? cut[0]).candidates, targets, node_limit: cut.length > 0, relation: ls[0]?.relation ?? 'calls' });
    }
  }
  return out;
}

/** Runs `fn` with a studio over a fresh copy; checks the original did not change. */
async function withStudio({ studio, bin, keys }, fn) {
  const before = Object.fromEntries(keys.map(k => [k, hashProject(studio, k)]));
  const root = copyStudio(studio, keys);
  const s = await startStudio(bin, root);
  try {
    return await fn({ ...s, root, before });
  } finally {
    s.stop();
    const after = Object.fromEntries(keys.map(k => [k, hashProject(studio, k)]));
    for (const k of keys) if (after[k].sha256 !== before[k].sha256) throw new Error(`o .studio original mudou: ${k}`);
    try { fs.rmSync(root, { recursive: true, force: true }); } catch { /* the studio may still hold a file on Windows */ }
  }
}

function gitHead(dir) {
  try { return execFileSync('git', ['-C', dir, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(); } catch { return null; }
}

module.exports = { REPO, DEFAULTS, parseArgs, hashProject, startStudio, copyStudio, loadGraph, occKey, pool, answers, withStudio, gitHead };
