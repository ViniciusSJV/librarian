// Minimal Python import table of a file, by text (diagnosis only): module-level and nested imports,
// `import a`, `import a.b`, `import a as x`, `from p import m [as x]`, relative `from . import m`.
'use strict';
const fs = require('fs'), path = require('path');

function logicalLines(src) {
  // joins parenthesized from-imports and backslash continuations; keeps indentation and line no.
  const raw = src.split(/\r?\n/), out = [];
  for (let i = 0; i < raw.length; i++) {
    let line = raw[i], start = i + 1;
    if (/^\s*(from\s+\S+\s+import\s*\(|import\s|from\s)/.test(line)) {
      while ((/\(\s*[^)]*$/.test(line) && !line.includes(')')) || /\\\s*$/.test(line)) { i++; if (i >= raw.length) break; line = line.replace(/\\\s*$/, ' ') + ' ' + raw[i]; }
    }
    out.push({ line: line.replace(/#.*$/, ''), no: start });
  }
  return out;
}

/** bindings: name -> [{module, kind: 'import'|'from', name?, nested, line}] */
function table(src) {
  const t = {};
  const add = (b, v) => (t[b] ??= []).push(v);
  for (const { line, no } of logicalLines(src)) {
    const nested = /^\s+/.test(line);
    let m;
    if ((m = line.match(/^\s*import\s+(.+)$/))) {
      for (const part of m[1].split(',').map(s => s.trim()).filter(Boolean)) {
        const [mod, as] = part.split(/\s+as\s+/);
        if (as) add(as.trim(), { kind: 'import', module: mod.trim(), nested, line: no });
        else { add(mod.split('.')[0], { kind: 'import', module: mod.split('.')[0], dotted: mod.trim(), nested, line: no }); }
      }
    } else if ((m = line.match(/^\s*from\s+(\S+)\s+import\s+\(?([^)]*)\)?/))) {
      for (const part of m[2].split(',').map(s => s.trim()).filter(Boolean)) {
        if (part === '*') { add('*', { kind: 'from', module: m[1], name: '*', nested, line: no }); continue; }
        const [name, as] = part.split(/\s+as\s+/);
        add((as ?? name).trim(), { kind: 'from', module: m[1], name: name.trim(), nested, line: no });
      }
    }
  }
  return t;
}

/** The project file a dotted module names, from the importing file (for relative imports). */
function moduleFile(root, fromPath, mod) {
  let base;
  if (mod.startsWith('.')) {
    const dots = mod.match(/^\.+/)[0].length;
    let dir = path.posix.dirname(fromPath);
    for (let i = 1; i < dots; i++) dir = path.posix.dirname(dir);
    const rest = mod.slice(dots);
    base = rest ? path.posix.join(dir, ...rest.split('.')) : dir;
  } else base = mod.split('.').join('/');
  for (const c of [base + '.py', base + '/__init__.py']) if (fs.existsSync(path.join(root, c))) return c.replace(/^\.\//, '');
  return null;
}

module.exports = { table, moduleFile };
