// O que as baterias de tela precisam da máquina: nada de caminho fixo aqui, só variáveis de ambiente.
// O que cada uma quer dizer está no README.md desta pasta.
const path = require('path');

const pede = nome => {
  const valor = process.env[nome];
  if (!valor) { console.error(`Falta a variável de ambiente ${nome}: veja crates/librarian-studio/tests/tela/README.md`); process.exit(2); }
  return valor;
};

module.exports = {
  // playwright-core não é dependência do repositório: vem de fora, por caminho ou do NODE_PATH.
  chromium: require(process.env.LIBRARIAN_TESTE_PLAYWRIGHT || 'playwright-core').chromium,
  RENDERER: pede('LIBRARIAN_TESTE_RENDERER'),
  PYTHON: pede('LIBRARIAN_TESTE_PYTHON'),
  // O gabarito fica em .studio, fora do Git; a fixture vale de qualquer raiz, porque só é lida.
  CASOS: process.env.LIBRARIAN_TESTE_CASOS || path.join(__dirname, '../../../../.studio/gabarito/casos.json'),
  // A chave do projeto é o nome mais o hash do caminho: estas valem só para os caminhos originais.
  CHAVE_RENDERER: process.env.LIBRARIAN_TESTE_CHAVE_RENDERER || 'renderer-librarian-26c4a0fd',
  CHAVE_PYTHON: process.env.LIBRARIAN_TESTE_CHAVE_PYTHON || 'backend-97d6e6b0',
};
