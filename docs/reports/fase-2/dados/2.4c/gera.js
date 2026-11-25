// Passo 2.4c, Parada 1: gera py24c.js do ../2.4b/py24b.js, trocando o fim pela regra do 2.4c.   node gera.js
'use strict';
const fs = require('fs'), path = require('path');
let s = fs.readFileSync(path.join(__dirname, '../2.4b/py24b.js'), 'utf8');
s = s.replace(/^\/\/ Passo 2\.4b, Parada 1[^\n]*/, '// Passo 2.4c, Parada 1 (gerado por gera.js a partir do ../2.4b/py24b.js; diagnóstico, não é código de produção).');
s = s.replace("require('./regras12.js')", "require('../2.4b/regras12.js')");
s = s.slice(0, s.indexOf('\nconst inits = [];') + 1);
s = s.replace('const rows = []', "const R4C = require('./regra24c.js')(() => ({ P, N, info, mro, lookup, descendants, classes, show, decorators, R, up, last, sym }));\nconst rows = []");
s = s.replace('\n  let mud;\n', '\n  R4C(c, p, t, r, text, at);\n  let mud;\n');
s += "require('./fim24c.js')(R4C, rows, check, outFile, { P, N, info, mro, classes, show, parts, slice, named, parent, declared, up, descendants, lookup, last, sym });\n";
fs.writeFileSync(path.join(__dirname, 'py24c.js'), s);
