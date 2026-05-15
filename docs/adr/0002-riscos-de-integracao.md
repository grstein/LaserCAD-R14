# ADR 0002 — Riscos de integração entre módulos

Este ADR consolida três decisões que endereçam riscos sinalizados pelos subagentes WS-A, WS-B e WS-C ao final da Sprint 1 (specs). São pré-requisito para a Sprint 2 (implementação): os três workstreams de código vão consumir essas regras simultaneamente.

- **Status global:** Aceito
- **Data:** 2026-05-12
- **Decisores:** Equipe LaserCAD
- **Riscos endereçados:** WS-A risco #1 (acoplamento core→render), WS-B risco #1 (ordem mount↔getScreenCTM), WS-C risco #1 (hosts DOM).

---

## 1. Lazy lookup do namespace para a ponte core → render

### 1.1 Status

Aceito — 2026-05-12 — Decisores: Equipe LaserCAD.

### 1.2 Contexto

A `plan.md` L222 fixa que `core/geometry` e `core/document` são puros: sem DOM, sem state global, sem bus. Mas a `plan.md` L269 também pede que conversões `world↔screen` sejam centralizadas (via `getScreenCTM()`) para evitar divergência tela↔documento. Sprint 1 resolveu isso especificando que `specs/render/camera.md` é o **único** dono da implementação e `specs/core/geometry/project.md` apenas re-expõe a interface para callers fora de `render/`.

A `namespace.md` define ordem de carga das folhas para as raízes: `project.js` carrega no slot 6, `camera.js` no slot 15. Uma referência **literal** a `window.LaserCAD.render.camera` em `project.js` no momento do registro (top-level da IIFE) falharia — `render.camera` ainda não existe quando `project.js` é avaliado.

### 1.3 Decisão

Adotar **lazy lookup** dentro dos métodos de `core.geometry.project`. As funções olham `window.LaserCAD.render.camera` **no momento da chamada**, não no momento da carga:

```js
// src/core/geometry/project.js (esqueleto)
(function (LaserCAD) {
  'use strict';
  const ns = LaserCAD.core.geometry;
  ns.project = {
    worldFromScreen(screenPt, camera) {
      const impl = window.LaserCAD.render && window.LaserCAD.render.camera;
      if (!impl)
        throw new Error(
          'LaserCAD: render.camera não carregado — project.worldFromScreen exige ordem de scripts conforme namespace.md',
        );
      return impl.worldFromScreen(screenPt, camera);
    },
    screenFromWorld(worldPt, camera) {
      const impl = window.LaserCAD.render && window.LaserCAD.render.camera;
      if (!impl) throw new Error('LaserCAD: render.camera não carregado');
      return impl.screenFromWorld(worldPt, camera);
    },
  };
})(
  (window.LaserCAD = window.LaserCAD || {
    core: { geometry: {}, document: {} },
    render: {},
    tools: {},
    ui: {},
    io: {},
    app: {},
    bus: {},
  }),
);
```

### 1.4 Consequências

- `core.geometry.project` é a **única exceção** documentada à pureza do kernel — só nesse arquivo o core olha para `render`. Tudo o mais em `core/` permanece puro.
- A ordem de carga em `namespace.md` continua válida; o lazy lookup não exige alterá-la.
- Erros de ordem aparecem de forma clara (mensagem nomeando o módulo que faltou), não como `NaN` silencioso.
- Custo de runtime desprezível: 1 hop de propriedade no namespace por chamada. Em pontos quentes (cursor:moved), os módulos chamam `render.camera.*` diretamente — `project.*` é só para callers fora de `render/`.
- O padrão pode ser reutilizado se outras pontes excepcionais aparecerem no futuro (registrar nova exceção em ADR específico).

---

## 2. Ordem obrigatória em bootstrap: mount antes de wire-up de input

### 2.1 Status

Aceito — 2026-05-12 — Decisores: Equipe LaserCAD.

### 2.2 Contexto

WS-B sinalizou que `camera.worldFromScreen` depende de `getScreenCTM()` do `<svg>` raiz; antes do `<svg>` estar no DOM (mounted), `getScreenCTM()` retorna `null` ou matriz identidade — qualquer conversão de coordenada produz lixo. Se `bootstrap` registrasse listeners de `pointermove` antes de montar o `<svg>`, o `cursor:moved` no momento da primeira renderização emitiria coordenadas inválidas, contaminando o state.

Adicionalmente, WS-B nota que `viewport:resized` pode disparar antes da câmera ter `viewportW/viewportH` válidos, causando race com `refreshViewBox` do `svg-root`.

### 2.3 Decisão

A função `LaserCAD.app.bootstrap.start()` executa **estritamente** nesta ordem, sem paralelizar:

```
1. validateNamespacePresence()      // confere que window.LaserCAD.core.geometry.vec2 (e demais) existem
2. state = LaserCAD.app.state.init()
3. config = LaserCAD.app.config.load()
4. validateDomHosts()               // ver decisão §3 deste ADR
5. svgRoot = LaserCAD.render.svgRoot.mount('#viewport-host')
6. state.setViewportSize(svgRoot.getSize())   // primeira medida
7. LaserCAD.render.camera.attach(svgRoot)     // câmera passa a usar getScreenCTM()
8. LaserCAD.render.grid.mount(svgRoot)
9. LaserCAD.render.overlays.mount(svgRoot)    // crosshair + label de coords
10. LaserCAD.ui.menubar.mount('#menubar-host')
11. LaserCAD.ui.toolbar.mount('#toolbar-host')
12. LaserCAD.ui.commandLine.mount('#commandline-host')
13. LaserCAD.ui.statusbar.mount('#statusbar-host')
14. LaserCAD.ui.dialogs.init()
15. LaserCAD.tools.toolManager.init()
16. LaserCAD.tools.toolManager.register('select', LaserCAD.tools.selectTool)
17. LaserCAD.app.shortcuts.attach(window)     // listeners globais de teclado
18. wireViewportInput(svgRoot)                // pointer events do viewport — SÓ AQUI
19. wireResizeObserver(svgRoot)               // viewport:resized — SÓ AQUI
20. LaserCAD.bus.emit('app:ready', {})
```

Passos 1–17 são síncronos e não tocam input. Wire-up de input (passos 18–19) só ocorre depois que `svg-root` está mounted **e** câmera está attached.

### 2.4 Consequências

- `getScreenCTM()` é chamado apenas quando garantido válido. Coordenadas world↔screen são confiáveis desde o primeiro frame.
- Se qualquer um dos passos 1–17 lançar, `bootstrap` reporta erro no DOM (banner sobre o body) e **não** registra listeners — o app fica "morto" mas inspecionável, em vez de "vivo mas com coords inválidas".
- `viewport:resized` jamais dispara antes de `app:ready` (passo 19 vem depois). Race eliminada.
- Esta ordem é fixa e versionada em `specs/app/bootstrap.md` §1. Mudanças exigem novo ADR.
- Para teste manual: `console.log(LaserCAD.render.camera.worldFromScreen({x:100,y:100}, LaserCAD.app.state.camera))` retorna ponto finito após `app:ready`, nunca antes.

---

## 3. Hosts DOM como contrato público entre HTML e bootstrap

### 3.1 Status

Aceito — 2026-05-12 — Decisores: Equipe LaserCAD.

### 3.2 Contexto

WS-C sinalizou que `bootstrap.md` monta os componentes via IDs (`#viewport-host`, `#menubar-host`, `#toolbar-host`, `#commandline-host`, `#statusbar-host`) que precisam estar presentes no `index.html` produzido por WS-B. Renomear silenciosamente um host quebra o bootstrap sem erro óbvio (apenas o componente correspondente "some").

A `design.md` L80–L112 desenha o layout em 4 regiões + menubar; `specs/index-html.md` materializa esses IDs; `specs/app/bootstrap.md` consome.

### 3.3 Decisão

Os cinco IDs abaixo são **contrato público** do projeto. Renomear exige ADR. Adicionar novo host exige ADR. Bootstrap valida presença antes de qualquer mount.

| ID                  | Função                                     | Spec proprietária     | Consumidor                 |
| ------------------- | ------------------------------------------ | --------------------- | -------------------------- |
| `#menubar-host`     | container do menubar 28px                  | `specs/index-html.md` | `specs/ui/menubar.md`      |
| `#toolbar-host`     | container da toolbar vertical 40px         | `specs/index-html.md` | `specs/ui/toolbar.md`      |
| `#viewport-host`    | container do `<svg>` raiz (área principal) | `specs/index-html.md` | `specs/render/svg-root.md` |
| `#commandline-host` | container da command line (3 linhas, 66px) | `specs/index-html.md` | `specs/ui/command-line.md` |
| `#statusbar-host`   | container da status bar (24px)             | `specs/index-html.md` | `specs/ui/statusbar.md`    |

`validateDomHosts()` em `bootstrap` itera essa lista e, se algum estiver ausente, escreve no `document.body` um banner com `[LaserCAD] Missing DOM host: <id>` (`color: var(--status-error)`) e lança no `console.error`. Os hosts não montados ficam inertes — o erro nunca é silencioso.

### 3.4 Consequências

- `index-html.md` e `bootstrap.md` referenciam esta tabela como fonte de verdade. Renomeio só é válido após ADR.
- Erro de ausência aparece visualmente em `< 100ms` após o carregamento, no próprio app, com mensagem precisa.
- Testes manuais podem mudar IDs no DevTools e observar o comportamento da validação (apagar `#viewport-host`, recarregar, ver banner).
- Adicionar uma sexta região no futuro (ex.: painel de propriedades) exige: (a) novo host no `index.html`, (b) atualização desta tabela, (c) menção em `bootstrap.validateDomHosts()`. Inconsistência reportada na revisão de PR.

---

## Relação com ADR 0001

ADR 0001 estabeleceu a base (SVG-first, mm, sem ES modules). ADR 0002 cobre as três fronteiras críticas onde a regra geral encontra exceções operacionais:

1. **Pureza do kernel × precisão centralizada** → resolvida com lazy lookup (§1)
2. **Sem build × ordem implícita** → resolvida com sequência obrigatória de bootstrap (§2)
3. **Sem framework × DOM compartilhado** → resolvida com hosts como contrato (§3)

Cada decisão preserva a simplicidade da ADR 0001 sem mascarar erros de integração.
