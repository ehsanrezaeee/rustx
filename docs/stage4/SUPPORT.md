# Target evidence matrix

This table describes observed paths, not a blanket v1 support promise. Exact artifacts/commands/results are in ACCEPTANCE.md and the evidence JSON. The desktop host is macOS 27.0.1 (26A434), Apple M3 arm64, 16 GiB. Browser version strings are recorded by the actual running engine; bundled WebKit is not branded Safari.

| Target | Rendering / fallback | Input / IME / accessibility | Services / autofill | State |
| --- | --- | --- | --- | --- |
| Installed Chrome 157.0.8103.40, Mac | Actual Rust CPU; physical Metal GPU/recovery suite recorded separately | Automated pointer/keyboard, Unicode/selection/focus; OS IME and human VoiceOver unverified | Real loopback and public JSONPlaceholder, IndexedDB/files/clipboard tasks where evidence names them; browser autofill unverified | Tested task subset; complete support gate open |
| Playwright WebKit 27.2, Mac | Actual Rust CPU browser presentation | Automated keyboard/semantics and playground; no actual Safari or human AT/IME claim | Actual loopback/live API/offline shell tasks where recorded; OS autofill unverified | Tested task subset |
| Firefox 135/157 on this Mac | Previous launches failed profile/sandbox initialization; retain failure logs; current attempt reported separately | Unverified | Unverified | No passing target claim |
| Actual Safari current/previous major | No branded Safari session exercised | Real VoiceOver/IME required | Clipboard/files/autofill required | Unverified |
| Physical Android Chrome, midrange device | No device available; GPU/CPU and mobile budget open | Touch/virtual keyboard/IME/TalkBack required | Mobile clipboard/files/autofill required | Unverified; desktop 390px emulation is only responsive layout evidence |
| Physical iPhone/iPad Safari | No device available; fallback/performance open | Touch/keyboard/VoiceOver required | Permission/autofill behavior required | Unverified |
| Native macOS 27.0.1 M3 | Actual window startup/CPU/Metal probes and loopback services; not complete app matrix | AccessKit adapter source/trees, automated engine input; human native AT/IME tasks open | Detected native utilities; full dialogs/permissions/printer matrix open | Partial native evidence |
| Windows x64/MSVC | Build/package script and manual CI workflow prepared; not executed here | Real UIA/NVDA, OS IME, DPI/resize required | Native services/permissions and browser tasks required | Blocked verification: no actual Windows session/package |
| Linux compiler provider | Bubblewrap isolation source exists | Not run on Linux | No compiled service claim | Unverified |

Previous browser major versions are not all available. No supported row is inferred from compilation, mock services, headless pixels or native Mac startup. Use PLATFORM_CHECKS.md for actual manual tasks. A CPU fallback path can provide functionality when GPU acceleration is unavailable, but it must itself pass the target's tasks.

Shared-source application matrix: homepage, counter/keyed lab, Applications forms/services, People Explorer, engine/virtual collection/drawing, presets and editable-source playground share the Rust workspace. Browser suites identify each exercised route. Native Windows must run the same app source; a browser screenshot is not equivalent. The compiler sandbox is a separate host tool; Windows native app support does not imply Windows compiler-service availability.
