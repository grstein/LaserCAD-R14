# main

## 1. Responsabilidade

Ponto de entrada único do LaserCAD R14: aguardar `DOMContentLoaded` e disparar `window.LaserCAD.app.bootstrap.start()`. Nada mais.

## 2. Dependências

- runtime: `window.LaserCAD.app.bootstrap` (chama apenas `start()`)
- ordem de carga: ultimo script da lista canônica em `specs/_conventions/namespace.md` (posição #28). Carrega depois de `src/app/bootstrap.js` (#27) e depois de todos os scripts de `core/`, `render/`, `tools/`, `ui/`, `app/event-bus.js`, `app/state.js`, `app/config.js`, `app/shortcuts.js`.

## 3. API pública

Não expõe API. Não popula sub-namespace. Não anexa nada a `window.LaserCAD`. É o único arquivo do projeto que pode conter código de topo de arquivo executado fora de uma IIFE — porque seu papel é apenas registrar o listener de `DOMContentLoaded`.

Forma exata (descrição, não implementação):

```js
// src/main.js
// pseudocodigo descritivo - nao implementacao
(function () {
  'use strict';

  function kickoff() {
    if (!window.LaserCAD || !window.LaserCAD.app || !window.LaserCAD.app.bootstrap) {
      // erro fatal: ordem de scripts quebrada
      // mensagem visivel em document.body, log no console
      return;
    }
    window.LaserCAD.app.bootstrap.start();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', kickoff, { once: true });
  } else {
    // DOM ja parseado (rodando depois de </body>) — chama direto
    kickoff();
  }
})();
```

Contrato:

- Pre: o DOM precisa conter `#viewport-host` (responsabilidade de WS-B em `specs/index-html.md`).
- Pos: `LaserCAD.app.bootstrap.start()` foi chamado exatamente uma vez. Erros levantados de dentro de `bootstrap.start()` propagam para o console.

## 4. Invariantes e tolerâncias

- Nunca chama outros sub-namespaces alem de `app.bootstrap`. Nunca toca DOM diretamente alem da deteccao de `document.readyState`.
- `kickoff` executa exatamente uma vez (`{ once: true }` no listener).
- Se `window.LaserCAD.app.bootstrap` faltar quando o handler dispara, o erro deve ser exibido tanto no console quanto no DOM (responsabilidade real desse fallback eh delegada a `bootstrap`; aqui basta checar existencia e logar).
- Sem `setTimeout`, sem `requestAnimationFrame`, sem retries.

## 5. Exemplos de uso

O arquivo nao expoe API pública. O unico "uso" eh o browser carregando o script ordenadamente. Para depuracao, no DevTools (depois do load):

```js
window.LaserCAD.app.bootstrap;
// → { start: ƒ }

// Forcar re-inicializacao manual (desenvolvimento apenas):
window.LaserCAD.app.bootstrap.start();
```

## 6. Criterios de aceitacao testaveis manualmente

1. Abrir `index.html` por duplo-clique no Chrome/Firefox/Edge. O console nao mostra ReferenceError, e a chamada `LaserCAD.bus.on('app:ready', () => console.log('READY'))` colocada antes do load nao eh possivel — porem, apos o load, `LaserCAD.app.state.activeTool` ja eh `'select'`, evidencia de que `bootstrap.start()` rodou.
2. Inspecionar `document.getElementById('viewport-host').children.length > 0` retorna `true` apos o load — evidencia indireta de que `bootstrap` foi disparado por `main`.
3. Renomear temporariamente `src/app/bootstrap.js` para forcar ausencia de `LaserCAD.app.bootstrap`: ao reabrir `index.html`, o console mostra erro legivel e o DOM exibe mensagem de erro (delegada ao fallback do proprio bootstrap se carregado, ou ao log defensivo do `main` quando bootstrap nao carregou). Restaurar o nome ao final.
4. Recarregar a pagina (`F5`) — o handler `DOMContentLoaded` dispara `kickoff` apenas uma vez por load; nao ha listener duplicado (verificavel por `LaserCAD.bus` nao acumular subscribers entre F5s, pois cada reload zera `window`).

## 7. Notas de implementacao

- ADR 0001 §3 proibe `<script type="module">`; `main.js` eh script classico igual aos demais.
- A ordem #28 (ultimo) eh fixada por `specs/_conventions/namespace.md` §3 tabela canonica.
- Plan.md L142, L228–235 falavam em `<script type="module" src="main.js">` — esse trecho fica superado pelo ADR 0001 §3.
- `main.js` eh deliberadamente magro: a sequencia real de bootstrap (validacao, montagem do `<svg>`, registro de subscribers) vive em `app/bootstrap.js` (ver `specs/app/bootstrap.md`). Isso isola "quando comecar" de "como comecar".
- Nao emite nenhum evento no bus; nao consome nenhum.
