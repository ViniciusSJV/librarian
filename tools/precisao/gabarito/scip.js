// Reads a SCIP index (rust-analyzer scip) without dependencies: the gabarito de tipos of the Rust (step 2.4e, DEC-48).
// Only the fields the measure uses (scip.proto):
//   Index { metadata = 1, documents = 2 }
//   Metadata { text_document_encoding = 4 }
//   Document { relative_path = 1, occurrences = 2, position_encoding = 6 }
//   Occurrence { range = 1 (packed int32: line, start, [end_line,] end), symbol = 2, symbol_roles = 3 }
// Positions are 0-based. `position_encoding`: 1 UTF-8 bytes, 2 UTF-16 units, 3 UTF-32 (0 = not said).
//
//   node scip.js <index.scip> [<path> <line 1-based>]   prints the encoding and, if asked, the occurrences of a line
'use strict';
const fs = require('fs');

function* fields(b, s, e) {
  let i = s;
  const varint = () => { let r = 0n, sh = 0n, x; do { x = b[i++]; r |= BigInt(x & 0x7f) << sh; sh += 7n; } while (x & 0x80); return r; };
  while (i < e) {
    const tag = Number(varint()), f = tag >> 3, w = tag & 7;
    if (w === 0) yield [f, varint()];
    else if (w === 2) { const n = Number(varint()); yield [f, [i, i + n]]; i += n; }
    else if (w === 1) i += 8;
    else if (w === 5) i += 4;
    else throw new Error(`SCIP: wire type ${w}`);
  }
}
function packed(b, [s, e]) {
  const out = []; let i = s;
  while (i < e) { let r = 0, sh = 0, x; do { x = b[i++]; r += (x & 0x7f) * 2 ** sh; sh += 7; } while (x & 0x80); out.push(r); }
  return out;
}
const str = (b, [s, e]) => b.toString('utf8', s, e);

/** { encoding, documents: [{ path ('/' separators), encoding, occurrences: [{ line, start, endLine, end, symbol, roles }] }] } */
function parse(file) {
  const b = fs.readFileSync(file);
  const out = { encoding: 0, documents: [] };
  for (const [f, v] of fields(b, 0, b.length)) {
    if (f === 1) for (const [g, w] of fields(b, v[0], v[1])) { if (g === 4) out.encoding = Number(w); }
    if (f !== 2) continue;
    const doc = { path: null, encoding: 0, occurrences: [] };
    for (const [g, w] of fields(b, v[0], v[1])) {
      if (g === 1) doc.path = str(b, w).replace(/\\/g, '/');
      else if (g === 6) doc.encoding = Number(w);
      else if (g === 2) {
        const o = { roles: 0 };
        for (const [h, x] of fields(b, w[0], w[1])) {
          if (h === 1) { const r = packed(b, x); Object.assign(o, r.length === 3 ? { line: r[0], start: r[1], endLine: r[0], end: r[2] } : { line: r[0], start: r[1], endLine: r[2], end: r[3] }); }
          else if (h === 2) o.symbol = str(b, x);
          else if (h === 3) o.roles = Number(x);
        }
        doc.occurrences.push(o);
      }
    }
    out.documents.push(doc);
  }
  return out;
}

module.exports = { parse };

if (require.main === module) {
  const [file, rel, line] = process.argv.slice(2);
  const idx = parse(file);
  const encs = {};
  for (const d of idx.documents) encs[d.encoding] = (encs[d.encoding] ?? 0) + 1;
  console.log(`metadata.text_document_encoding ${idx.encoding}; documentos ${idx.documents.length}; position_encoding por documento ${JSON.stringify(encs)}`);
  if (rel) for (const d of idx.documents) if (d.path === rel) for (const o of d.occurrences) if (o.line + 1 === +line) console.log(JSON.stringify(o));
}
