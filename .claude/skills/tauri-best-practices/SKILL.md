---
name: tauri-best-practices
description: Guia de best practices do Tauri 2.x aplicado ao LaserCAD R14 — capabilities/permissions, IPC (commands/events), plugins oficiais (fs/dialog/store), CSP, bundling e distribuição. Use ao mexer em src-tauri/, tauri.conf.json, capabilities/*.json, comandos Rust, ponte web↔nativa (src/tauri-bridge.ts) ou release multi-plataforma.
---

# Tauri 2.x — Best Practices (LaserCAD R14)

Skill local para guiar mudanças em `src-tauri/`, no bridge runtime (`src/tauri-bridge.ts`) e nos artefatos de empacotamento. Toda referência aponta para a documentação oficial **Tauri 2** (`v2.tauri.app`). **Não usar `v1.tauri.app`** — a API e o modelo de segurança mudaram entre v1 e v2 (allowlist → capabilities).

Stack atual confirmado (`src-tauri/Cargo.toml`): `tauri = "2"`, `tauri-plugin-dialog = "2"`, `tauri-plugin-fs = "2"`, `tauri-plugin-store = "2"`. Rust mínimo `1.77.2`.

---

## 1. Modelo mental

- **Core (Rust)** roda fora do sandbox do browser, com acesso total ao SO. Lógica sensível mora aqui.
- **Webview do sistema** (não bundled) renderiza o frontend TS/SVG. Recebe patches de segurança via OS, não via release do app.
- **IPC** é a fronteira de confiança. Tudo que cruza precisa de tipo, validação e permissão explícita.
- **Plugins** são unidades de funcionalidade com permissões próprias (declaradas em `capabilities/*.json`).

Aprofundamento: <https://v2.tauri.app/concept/>

---

## 2. Segurança (capabilities + permissions + scopes)

Em v2 **não existe mais `allowlist`**. O modelo é granular e fica em `src-tauri/capabilities/<arquivo>.json`.

### Princípios
1. **Menor privilégio**: cada janela só recebe as permissões que de fato usa.
2. **Permissões específicas** em vez de `:default` quando der (ex.: `fs:allow-write-text-file` em vez de `fs:default`).
3. **Scopes restringem alvos** (caminhos para `fs`, comandos para `shell`, URLs para `http`). Evite curingas amplos.
4. **Per-window / per-platform**: use `windows: ["main"]` e `platforms: ["linux","macOS","windows"]` para isolar capacidades.
5. **Mesma window em múltiplos capability files mescla as permissões** — cuidado com vazamento por união.
6. **Window labels, não títulos**, definem fronteiras.

### Estrutura mínima
```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "main",
  "description": "LaserCAD main window",
  "windows": ["main"],
  "platforms": ["linux", "macOS", "windows"],
  "permissions": [
    "core:default",
    "dialog:allow-open",
    "dialog:allow-save",
    "fs:allow-write-text-file",
    "fs:allow-read-text-file",
    "store:default"
  ]
}
```

Docs:
- Capabilities — <https://v2.tauri.app/security/capabilities/>
- Permissions — <https://v2.tauri.app/security/permissions/>
- Scopes — <https://v2.tauri.app/security/scope/>
- Migração v1 → v2 — <https://v2.tauri.app/start/migrate/from-tauri-1/>

### CSP
LaserCAD já define CSP estrita em `tauri.conf.json` (`app.security.csp`). Regras:
- Manter `script-src 'self'`. **Nunca** adicionar `'unsafe-inline'` para scripts.
- Estilos inline são tolerados (`style-src 'self' 'unsafe-inline'`) porque SVG runtime depende disso; ainda assim, prefira CSS estático.
- `connect-src` deve incluir `ipc:` e `http://ipc.localhost` (necessário para o bridge IPC).
- Imagens locais e blob para export: `img-src 'self' data: blob:`.

Docs: <https://v2.tauri.app/security/csp/>

### Padrão de isolamento
Para apps com lógica frontend de terceiros, considerar o Isolation Pattern como camada intermediária no IPC: <https://v2.tauri.app/concept/inter-process-communication/isolation/>

---

## 3. IPC — Commands vs Events

| Use **Command** quando | Use **Event** quando |
|---|---|
| Precisa de retorno tipado | É fire-and-forget |
| Erro estruturado importa | Vai broadcastar para múltiplas janelas |
| Operação tem semântica de RPC | É notificação de ciclo de vida |

### Command idiomático (Rust)
```rust
use serde::Serialize;
use tauri::State;

#[derive(Debug, thiserror::Error)]
enum CmdError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Serialize for CmdError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

#[tauri::command]
async fn save_drawing(path: String, svg: String) -> Result<(), CmdError> {
    tokio::fs::write(path, svg).await?;
    Ok(())
}
```

Registrar **uma única vez** em `lib.rs`:
```rust
tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![save_drawing])
    .run(tauri::generate_context!())
```

> Chamadas repetidas a `.invoke_handler(...)` **sobrescrevem** as anteriores — sempre lista única em `generate_handler!`.

### Regras práticas
- `async fn` não aceita `&str` nem `State<'_, T>` por referência — use `String` e clone state.
- Retorne `Result<T, E>` com `E: Serialize` (`thiserror` + `impl Serialize`).
- Args JS chegam em camelCase. Para snake_case use `#[tauri::command(rename_all = "snake_case")]`.
- Payloads binários grandes: `tauri::ipc::Response::new(bytes)` evita custo JSON.
- Streaming: `tauri::ipc::Channel<&[u8]>` em vez de retorno único.

### Frontend
```ts
import { invoke } from '@tauri-apps/api/core';
await invoke('save_drawing', { path, svg }); // chaves em camelCase
```

Docs:
- IPC — <https://v2.tauri.app/concept/inter-process-communication/>
- Calling Rust from Frontend — <https://v2.tauri.app/develop/calling-rust/>
- Calling Frontend from Rust — <https://v2.tauri.app/develop/calling-frontend/>
- State management — <https://v2.tauri.app/develop/state-management/>

---

## 4. Plugins oficiais

Lista completa: <https://v2.tauri.app/plugin/>

Em uso pelo LaserCAD:

| Plugin | Doc | Uso atual |
|---|---|---|
| `tauri-plugin-dialog` | <https://v2.tauri.app/plugin/dialog/> | Open/Save SVG nativos |
| `tauri-plugin-fs` | <https://v2.tauri.app/plugin/file-system/> | Leitura/escrita do arquivo escolhido |
| `tauri-plugin-store` | <https://v2.tauri.app/plugin/store/> | Autosave key-value (substitui localStorage no nativo) |

Candidatos úteis no futuro:
- `tauri-plugin-window-state` — persistir tamanho/posição da janela: <https://v2.tauri.app/plugin/window-state/>
- `tauri-plugin-single-instance` — evitar duas janelas LaserCAD abertas: <https://v2.tauri.app/plugin/single-instance/>
- `tauri-plugin-updater` — atualização in-app: <https://v2.tauri.app/plugin/updater/>
- `tauri-plugin-log` — logging configurável: <https://v2.tauri.app/plugin/logging/>
- `tauri-plugin-deep-linking` — abrir `.svg`/`.lc` via URL/file association: <https://v2.tauri.app/plugin/deep-linking/>

### Cada plugin exige 3 coisas
1. Dependência em `Cargo.toml`.
2. `.plugin(tauri_plugin_xxx::init())` no builder.
3. Permissões correspondentes em `capabilities/*.json`.

---

## 5. Padrão do LaserCAD: runtime split

`src/tauri-bridge.ts` é a **única** ponte web↔nativo. Regras:

- Plugins Tauri NUNCA são importados estaticamente. Sempre dentro de `if (isTauri())` com `await import('@tauri-apps/plugin-xxx')`. Isso mantém o bundle web leve e funcional em `npm run dev`.
- Em browser puro, cada função do bridge retorna `null`/`undefined` e o fluxo cai para `localStorage` + download via Blob.
- Detector canônico: `typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window`.

Não duplicar essa lógica em outros módulos — sempre passar pelo bridge.

---

## 6. Configuração (`tauri.conf.json`)

- `productName`, `identifier` (reverse-DNS), `version` — versão aqui tem prioridade sobre `Cargo.toml`.
- `build.devUrl` aponta para o Vite (porta 1420 no projeto).
- `build.frontendDist` aponta para `../dist` após `npm run build`.
- `app.windows[].label` é o que capabilities referenciam (não o `title`).
- `app.security.csp` — manter restritiva (ver §2).
- `bundle.targets: "all"` faz `tauri build` gerar todos os formatos suportados pelo SO host.

Schema oficial: `https://schema.tauri.app/config/2` (já referenciado no projeto).

Referência completa: <https://v2.tauri.app/reference/config/>

---

## 7. Desenvolvimento e debug

- `npm run tauri:dev` — janela nativa com HMR; rebuilda Rust quando `src-tauri/` muda.
- DevTools: clique direito → Inspect, ou `Ctrl+Shift+I` (Linux/Win), `Cmd+Opt+I` (macOS).
- Commit `Cargo.lock`. Nunca commitar `src-tauri/target/`.
- `.taurignore` controla quais paths o watcher ignora.

Guias por IDE:
- VS Code — <https://v2.tauri.app/develop/debug/vscode/>
- RustRover — <https://v2.tauri.app/develop/debug/rustrover/>
- Neovim — <https://v2.tauri.app/develop/debug/neovim/>

Overview: <https://v2.tauri.app/develop/>

---

## 8. Distribuição

`tauri build` gera todos os bundles do SO host por padrão. Para granularidade: `tauri build --no-bundle` seguido de `tauri bundle --bundles deb,appimage`.

Por plataforma (apontar para o doc certo antes de configurar):

- **Linux**:
  - AppImage — <https://v2.tauri.app/distribute/appimage/>
  - Debian — <https://v2.tauri.app/distribute/debian/>
  - RPM — <https://v2.tauri.app/distribute/rpm/>
  - Snap — <https://v2.tauri.app/distribute/snapcraft/>
- **macOS** (assinatura + notarização obrigatórias fora da App Store):
  - DMG — <https://v2.tauri.app/distribute/dmg/>
  - App Bundle — <https://v2.tauri.app/distribute/macos-application-bundle/>
- **Windows**:
  - MSI/NSIS — <https://v2.tauri.app/distribute/windows-installer/>

Assinatura (obrigatória na maioria dos targets):
- macOS — <https://v2.tauri.app/distribute/sign/macos/>
- Windows — <https://v2.tauri.app/distribute/sign/windows/>
- Linux — <https://v2.tauri.app/distribute/sign/linux/>

CI:
- GitHub Actions pipeline oficial — <https://v2.tauri.app/distribute/pipelines/github/>

Auto-update: plugin Updater — <https://v2.tauri.app/plugin/updater/>

---

## 9. Checklist antes de fazer release

- [ ] `version` em `tauri.conf.json` bate com `package.json` e CHANGELOG.
- [ ] `capabilities/*.json` revisada: sem permissões não usadas, sem scopes amplos.
- [ ] CSP em `app.security.csp` ainda sem `'unsafe-inline'` em `script-src`.
- [ ] `npm run typecheck && npm run lint && npm test` verdes.
- [ ] `npm run tauri:build` localmente em pelo menos um target; abrir o binário e exercitar Open/Save/Autosave.
- [ ] Ícones presentes em `src-tauri/icons/` para todos os formatos listados em `bundle.icon`.
- [ ] Se publicar updates: chave de assinatura do Updater configurada e endpoint público.

---

## 10. Quando não usar Tauri / quando subir um issue

- Se um requisito exige API que não existe em plugin oficial nem comunitário, prefira **escrever um command** em vez de wrapper genérico via `shell` (este último amplia muito a superfície).
- Bug de plugin oficial → repo correspondente em `https://github.com/tauri-apps/plugins-workspace`.
- Vulnerabilidade → `security@tauri.app` ou GitHub Private Disclosure. **Não abrir issue público.**

---

## Referências rápidas

- Home v2 — <https://v2.tauri.app/>
- Conceitos — <https://v2.tauri.app/concept/>
- Segurança — <https://v2.tauri.app/security/>
- Desenvolvimento — <https://v2.tauri.app/develop/>
- Distribuição — <https://v2.tauri.app/distribute/>
- Plugins — <https://v2.tauri.app/plugin/>
- Reference (config, ACL, CLI) — <https://v2.tauri.app/reference/>
- Migração v1 → v2 — <https://v2.tauri.app/start/migrate/from-tauri-1/>
