# LCV-199 — Plan

## Approach

The panel gets an `Attach image…` button and a removable chip. The button only sets
`agent.attach_requested`. The frame wiring, after the panel is drawn, consumes the flag through
`io/dialogs.rs::pick_image_dialog` (ADR 0005) and feeds the path to `agent_attach::attach_image`.
Tests call `attach_image` directly: it is the seam the dialog result feeds. A kernel-pure
`agent/attachment.rs` checks the file: magic bytes PNG/JPEG and a 2 MB cap. At send time
`start_turn` re-reads and re-checks the file. On failure it sends nothing, writes a transcript
row and restores the prompt (AC5). On success the bytes ride `TurnConfig.image` and
`drive_turn` builds the user message as `[text, image]` parts. Base64 stays in `wire.rs`
(ADR 0011). The existing `send_images` elision drops the image after its one request. The
memory record keeps `image elided` in its place (AC6). With no attachment every path is today's,
which gives AC7.

## Touches

- `src/io/dialogs.rs::pick_image_dialog` (new) — `require_armed`, filter `png`/`jpg`/`jpeg`;
  disarmed-panic test like its siblings.
- `src/agent/attachment.rs` (new, kernel-pure) — `ImageKind { Png, Jpeg }` with `mime()`,
  `sniff(bytes)`, `MAX_IMAGE_BYTES = 2 * 1024 * 1024`, `check(name, bytes) -> Result<ImageKind,
  String>`; `src/agent/mod.rs` re-export. AGENTS.md purity list gains `attachment.rs`.
- `src/agent/wire.rs::ContentPart::image(mime, bytes)` (new; `png` delegates to it).
- `src/agent/loop_/images.rs::send_images` — the `AuthorizeUpload` ask and the `Replied`
  capture count cover only images after the turn's user message. The attached image is
  consented by the attach (spec decision) and elided like any image.
- `src/agent/memory.rs::turn_record` — the user entry becomes `[text, "image elided"]` parts
  when the turn carried an image.
- `src/app/agent_worker.rs` — `TurnConfig.image: Option<UserImage>` (`Debug` prints name and
  byte count only); `drive_turn` builds the parts user message when it is `Some`.
- `src/app/agent_attach.rs` (new) — `Attachment { path, name, kind }`, `attach_image(app, path)`,
  `poll_attach_request(app)`, `take_for_send(app) -> Result<Option<UserImage>, String>`.
- `src/app/agent_state.rs::AgentState` — `attachment: Option<Attachment>`, `attach_requested`.
- `src/app/agent_turn.rs::start_turn` — `take_for_send`; on `Err`: a transcript row, the draft is
  restored, nothing is armed; on `Some`: the row `Image: <name>`, and the attachment is cleared.
  `src/app/agent_memory.rs::record` passes the image flag.
- `src/agent/panel.rs::draw_agent_panel` — chip (name + `×`) above the input row; button enabled
  only while `settings.agent_model_supports_vision`, with the AC3 tooltip.
- `src/app/panels.rs` — call `poll_attach_request` after `draw_agent_panel`.
- ADRs: none. ADR 0005 and ADR 0011 are followed as written. The consent rule is a spec
  decision, recorded in the `send_images` doc comment.

## Decisions (self-approved per user goal)

- Status messages go through `App::say(Severity::Warning, …)`:
  - `Image not attached: <name> is not a PNG or JPEG.`
  - `Image not attached: <name> is over 2 MB.`
- The AC5 transcript row (role `error`) reads `Image <name> could not be read: <io error>.
  Nothing was sent.`
- A turn started from the command line also takes the attachment; it is taken once, in
  `start_turn`.
- If `Model supports images` was turned off after attaching, `take_for_send` refuses like AC5:
  `"Model supports images" is off. Nothing was sent.`

## Risks

- LOC cap: `wire.rs` 286 (+~6 → ~292). If it passes 300, the seam is to move
  `AssistantMessage`/`Choice`/`ChatResponse` to `wire/response.rs` (kernel-pure, AGENTS.md list).
  base64 stays in `wire.rs`. `panel.rs` 256 (+~22 → ~278): the chip + button go in one
  `attach_row(ui, app)` fn. If it passes 290, move `footer_reserve` and the busy block to
  `panel/busy.rs`. `agent_turn.rs` 250 (+~10), `agent_worker.rs` 230 (+~12).
- Mutation testing: **yes**, `src/agent/`. Targets: `sniff` (the magic-byte boundaries), the 2 MB
  `>` vs `>=`, and the `send_images` split index (a canvas capture must still ask; the attached
  image must not).
- AC7 byte-identity: a test serialises a no-image request before and after the change against a
  pinned JSON string, the same pattern as `wire.rs::text_only_messages_serialise_byte_identically`.
- The dialog must not run in tests: the AC1 test sets `attach_requested` only through the
  painted button. It asserts the flag and never calls `poll_attach_request`. `dialogs_disarmed`
  scans cover the new wrapper.

## Seam note (post-review)

`src/app/agent_turn.rs` ends at 286 implementation lines; the next growth moves the turn-start
attachment and config assembly into `src/app/agent_turn/start.rs`.
