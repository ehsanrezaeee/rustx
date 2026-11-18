# RP-008 native interaction

## Windows keyboard and resize

Run `scripts/test-windows-interaction.ps1 -Executable <extracted native-demo.exe>` in Windows PowerShell. The fixture is the unmodified default counter scaffold from RP-007. A fresh instance on the CPU renderer must expose “Count: 0”, reach “Count: 1” through Tab and Enter, retain it after resizing the window to 640 × 480, then reach “Count: 2” through Space without another Tab. The process must close normally. The driver posts key messages only to the counter's window and reads back the application state through Windows UI Automation.

The first replay on an actual Windows desktop used the [RP-007 Windows package](https://github.com/ehsanrezaeee/rustx/actions/runs/44352793428/job/134972697816) from `324c1e4`. Executable SHA-256 `b8f799312ffc56b99f58e5b88e0f7c911a5ea553437bbb96aadfd47b33438c3b`. It timed out at the first assertion. UI Automation returned the heading “Hello from RustX” and the button “Increase”, but unnamed static text elements. The Windows AccessKit adapter reads a label node's name from its value. RustX had supplied only its label. The native projection now supplies the semantic text as the AccessKit label node's value.

## Remaining interaction tasks

Use the same task expectations on Windows and macOS. Record OS, package source/hash, backend and input method for each run.

| Task | Observable result |
| --- | --- |
| Keyboard and focus | Tab and Shift+Tab traverse a multi-control form in logical order. Closing an overlay returns focus to its opener. |
| Pointer capture | Press and drag beyond the original target, then release. Capture ends once and the next click reaches its own target. |
| Wheel and scroll | Scroll a long list in both directions, hit each bound, then activate the row under the pointer. |
| Clipboard and files | Round-trip a Unicode selection. Exercise file dialog cancel and a successful file selection. |
| Window size and DPI | Retain edited state and focus through resize and a change of display scale. Verify hit coordinates on each display. |
| Multiple windows | Alternate input between two windows in one process and close one. The other keeps its state and focus behavior. |

Physical keyboard input, mixed-DPI hardware, GPU interaction, macOS interaction, OS IME and human screen-reader tasks require separate runs.
