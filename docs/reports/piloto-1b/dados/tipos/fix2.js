const fs = require('fs'), T = process.argv[2];
let p = fs.readFileSync(T + '/py-inv/pkg/casos.py', 'utf8');
p = 'from math import sqrt\n' + p.replace(/^(\s*)for (.*):$/gm, (m, ind, h) => `${m}\n${ind}    sqrt(2)`);
fs.writeFileSync(T + '/py-inv/pkg/casos.py', p);
let r = fs.readFileSync(T + '/rs-inv/src/casos.rs', 'utf8');
r = r.replace(/^(\s*)for (.*) \{$/gm, (m, ind) => `${m}\n${ind}    let _ = (2.0f64).sqrt();`);
fs.writeFileSync(T + '/rs-inv/src/casos.rs', r);
