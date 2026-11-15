# Browser DOM exception register — initial source audit

16 November 2026. Owner: browser host adapter. These are actual source dependencies, not a claim that the current DOM-minimization goal is achieved. Measure live counts/mutations in RP-004. Normal component layout/text remains Rust-owned.

| ID / source | Capability and owner/lifetime | Cardinality / updates | Alternative and remaining evidence |
| --- | --- | --- | --- |
| DOM-01 `web/index.html`, `web/host.js` | Document, surface canvas, bootstrap script/style and semantic root; page lifetime | Fixed shell per current singleton host; canvas replaced on recovery | Required browser shell; multiple roots/disposal unverified |
| DOM-02 `web/semantics.js` | Transparent inputs/textareas/buttons/anchors/spans from Rust semantics; projection owns nodes/listeners | One per projected semantic node; geometry/properties updated at sync | Cannot remove without AT/input evidence; pooling not yet justified; RP-002 fixes disposal, RP-004 measures mutation cost |
| DOM-03 `web/index.html`, `web/host.js` | Visible status badge and fatal reload UI; page/failed runtime lifetime | One status region, fatal message/button | Status is visible DOM outside canvas; migrate normal status to Rust or devtools-only. Fatal recovery needs an explicit host exception when Rust cannot run |
| DOM-04 `web/services.js` | File input, download anchor/object URLs, printable popup | Operation lifetime; verify cancellation/cleanup | Browser picker/download/print capabilities; test actual permission/print behavior |
| DOM-05 `web/playground.js`, `playground/preview.html` | Opaque sandbox iframe runs compiled canvas app | At most current preview; replaced/removed on navigation/pagehide | Security isolation, not normal DOM widget rendering; audit inner host separately |
| DOM-06 `web/inspector.html`, `web/inspector.js`, `web/devtools.js` | Developer inspection UI | Development-only; separate tool lifetime | Tooling exception; prove production defaults and listeners |
| DOM-07 generated bundle head/service worker | Resource hints, metadata, script loading | Build-time and host lifecycle | Partial: no general typed metadata/resource API |
| DOM-08 future semantic/server/print exports | Not yet implemented full content projection | No approved count budget until design | Must share Rust content model, avoid duplicate announcements; full browser parity blocker |

Required budget before optimization: zero ordinary visible application controls rendered by DOM; enumerate host emergency/tooling/embedded exceptions separately. Current status badge is a recorded gap, not an accepted ordinary-widget exception. Semantic-node budgets scale with accessible content; preserve focused virtual rows. RP-004 must freeze counts for each reference app, mutations per semantic change and 100-cycle teardown baselines before optimizing. Do not disable accessibility to meet budgets.

RP-002 removes projection-owned elements and event listeners on disposal, clears composition/focus references, preserves unrelated root children, and rejects recreation after disposal. Real-browser adapter checks use a stub Rust receiver, so they prove DOM lifecycle only. Full host disposal and BFCache restoration remain separate work.
