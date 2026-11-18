const fs = require('fs'), T = process.argv[2], f = T + '/rs-inv/src/casos.rs';
const o = fs.readFileSync(T + '/rs/src/casos.rs', 'utf8');
let r = fs.readFileSync(f, 'utf8');
const piece = (text, name) => { const a = text.indexOf('pub fn ' + name + '('); const b = text.indexOf('\n}\n', a) + 3; return [a, b]; };
for (const n of ['n6', 'n6b']) { const [a, b] = piece(r, n), [c, d] = piece(o, n); r = r.slice(0, a) + o.slice(c, d) + r.slice(b); }
fs.writeFileSync(f, r);
