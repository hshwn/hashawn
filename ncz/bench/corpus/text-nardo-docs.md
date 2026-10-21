# Availability and developer tools

[Feature index](../../FEATURES.md) · [Source coverage map](source-map.md)

The audited application is the Surface package plus the Nardo Search target. Source presence, build inclusion and a reachable interface are separate facts. This page keeps capabilities with missing entry points visible for future release work.

## Availability ledger

| ID | Capability | Current status and entry/dependency |
| --- | --- | --- |
| A01 | Surface editor, layer and command shelf | **Integrated.** Surface executable launches embedded companion; their feature references describe current wiring. |
| A02 | Browser, search and utilities | **Integrated.** Browser is configured by `EdgeBrowserWindow`; search companion has its own accessory-app entry. Core behavior behind the FFI is a prebuilt library, not inspectable Rust source here. |
| A03 | Option Notes | **External.** Search/capture/open routes launch installed Option Notes. Editor source is present in companion but the normal route hands off; build does not embed the separate app. Its installed version was not audited. |
| A04 | Games library | **Integrated with external dependencies.** Native Games window/search work from companion. Steam, compatibility layers, console emulators and Android SDK/AVD are separate dependencies. Hydra requires reachable feeds. |
| A05 | File search | **External index.** Read-only lexical lookup works against existing `~/fssearch/index.db`; this app does not crawl/build that index. Older dense-retrieval experiments are not its current search path. |
| A06 | Voice recognition | **Integrated Browser entry, external runtime.** Option-Space in Browser; local Parakeet executable/model/cache required. No recognizer installation is performed by this build script. |
| A07 | Global dictation / other-app insertion | **Integrated (October 16), partly verified.** Caps+D inserts into the field captured at the start of the take in any app: Accessibility (read back), paste with the clipboard saved and restored and kept out of history, or typed keys, chosen per app family and field ([Dictation](dictation.md)). Verified headless and with real Accessibility into a throwaway window; not yet run live in third-party apps. Option-Space hotkey still unregistered in Edge. |
| A08 | Voice endpoint polish | **Integrated, local only.** Edge sends the final pass to on-device s1-mini (llama.cpp on :8093) when its model is installed, else baseline cleanup; with a per-app tone ([Dictation](dictation.md)). The AI polish switch is not consulted by this wiring. Remote endpoints remain standalone-entry only. |
| A09 | Current Settings scene | **Integrated, limited.** Open Settings from `>` or site tools opens the small Spotlight information/Open Spotlight scene. Its wording about default-browser continuation predates Edge's integrated Browser. |
| A10 | General settings pane | **Source only route.** DuckDuckGo/Google/Bing/Kagi/Brave/custom `{query}` search templates, new-tab URL, default-browser status and Make Default. Full `SettingsView` is only presented by excluded `OptionApp.swift`. |
| A11 | Download settings pane | **Source only route.** Folder picker and Ask for each download; core settings/download behavior still exist. |
| A12 | Password settings pane | **Source only route.** Search saved credentials, reveal/copy passwords, delete accounts. Site-level save/fill remains integrated. |
| A13 | Shortcut settings pane | **Source only route.** Record/remove/reset bindings and reset all; conflicting assignments unbind previous owner. Overrides persist and are honored. Voice chord is fixed. Browser cheat sheet is integrated. |
| A14 | Extension settings pane | **Source only route.** Install/toggle/remove management panel. Store-page install banner, extension toolbar and site tools provide integrated extension routes. |
| A15 | Voice settings pane | **Source only route.** Recognizer command/model/cache, Test, context cleanup, AI cleanup and restore-clipboard switches. A toggle's presence does not wire an absent callback. |
| A16 | Privacy settings pane | **Source only route.** Ad blocker, clear history/cache and confirmed clear cookies/local storage/IndexedDB; privacy information. `>` clear history/cache/blocker controls remain integrated. |
| A17 | Advanced / About panes | **Source only route.** Microphone/Accessibility status and settings links, maximum live webviews, version/edition/system/browser engine and license information. |
| A18 | Browser carryover/onboarding | **Source only entry.** Browser/file import picker; supported cookie formats and Chromium password machinery exist. Standalone import menu/onboarding presentation is excluded. Do not promise automatic Safari/iCloud password import. |
| A19 | Print/screenshot advertised keys | **Entry gap.** Cmd-P / Shift-Cmd-S labels exist, but their menu bindings are in excluded standalone entry. Use site tools or `> Print Page` / `> Copy Page Screenshot`. |
| A20 | Calculator scope variables | **Source only integration.** `CalculatorScope` supports `x`/`ans` and scope evaluation; current Spotlight calculation calls stateless evaluator. Ordinary calculations/conversions are integrated. |
| A21 | Journal | **Excluded by build flag.** Native registration requires `PERSONAL_EDITION`, absent from current project. Source records reverse-chron film/TV entries from Option Stream, posters, progress/watched state, five-star rating/reset, Continue, Notes review boards and removal. |
| A22 | Option Stream / older essentials | **Not supplied as current app features.** Catalog entries may refer to HTML apps/resources not included here. Journal's capture source is unavailable in current edition. Source catalog labels alone are not evidence of a shipped surface. |
| A23 | Debug native surface | **Debug only.** `option://debugsurface` demonstrates native tab mounting/title/deep-link plumbing under `DEBUG`; release target does not register it. |
| A24 | Free VPN | **Integrated, approval-dependent.** Panel/helper/resources shipped. Administrator approval and Connect required; privileged routing/leak/crash testing boundaries are recorded in networking notes. |
| A25 | VPN federation | **Proposed/isolated probes.** Provider transport tests and proposal; no current federation selection, volunteer relays or replacement Free VPN backend. |
| A26 | Native model-driven editor adapter | **Experimental.** Explicit `--native-agent-session` route, isolated test workspace and file-backed requests; no autonomous agent started by ordinary commands. |
| A27 | Prototype/legacy catalogs | **Historical.** `work/`, old UI/Essentials catalogs and standalone app notes can contain removed shortcuts, absent resources or old routing. Prefer this audit and current handlers for Edge. |
| A28 | Clipboard shelves/editing/retention controls | **Source only UI.** Store supports named colored shelves, add/remove membership, rename/delete shelves, item editing and configurable retention. Current Edge clipboard feed exposes search/copy/pin/delete/drag, not those management controls. |

Sources: [build script](../../outputs/surface-edge/build.sh), [companion target/exclusions](../../outputs/option-edge-search/project.yml), [actual companion entry](../../outputs/option-edge-search/Spotlight/App/SpotlightApp.swift), [Browser configuration](../../outputs/option-edge-search/Option/UI/EdgeBrowserWindow.swift), [excluded standalone entry](../../outputs/option-edge-search/Option/App/OptionApp.swift), [full SettingsView](../../outputs/option-edge-search/Option/UI/SettingsView.swift), [native registration](../../outputs/option-edge-search/Option/WebKit/NativeSurface.swift), [Journal](../../outputs/option-edge-search/Option/Essentials/Journal/JournalView.swift), [calculator scope](../../outputs/option-edge-search/Option/UI/CalculatorScope.swift), [essential catalog](../../outputs/option-edge-search/Option/WebKit/Essentials.swift).

## Developer entry points

These are commands on the Surface executable, not buttons for ordinary users. `OUTPUT` and `DIRECTORY` below mean explicit absolute paths for disposable verification/session output. Verification features are documented here; they were not executed as part of this documentation-only audit.

| ID | Entry | Purpose / limits |
| --- | --- | --- |
| DEV01 | `Surface --verify OUTPUT` | Native geometry, text/shape hit routing, controls, editing, command plans/undo and stale-context checks; writes reports/artifacts. |
| DEV02 | `Surface --benchmark-canvas OUTPUT` | Large boards/text/nested previews, first/warm CPU render timing, preview invalidation and gesture undo checks. Measures CPU rendering, not display FPS. |
| DEV03 | `Surface --verify-cpu OUTPUT` | Grid rendering, interaction batching and CPU accounting checks. |
| DEV04 | `Surface --verify-performance-ui OUTPUT` | Performance panel/UI verification and output. |
| DEV05 | `Surface --native-agent-session DIRECTORY` | Isolated editor/recovery, typed `request.json` operations and `state.json` with objects/selection/visible bounds/execution feedback. Uses normal capability validation/atomic edits/undo while bypassing parser inference. |
| DEV06 | Native session snapshots | Save editable `.surface` plus rendered PNG; `fixed_canvas: true` uses canvas coordinates rather than fit-artwork framing. UUID-targeted placement/anchor operations support model experiments; choice scores are not calibrated parser confidence. |
| DEV07 | `Surface --proxy-status` | Read service/helper status. |
| DEV08 | `Surface --install-proxy-helper` / `--reload-proxy-helper` | Mutate helper registration for development; separate from connecting networking. Administrator/service approval can be needed. |
| DEV09 | `Surface --verify-proxy-ui OUTPUT` | Networking panel verification; does not establish successful privileged tunnel/leak testing. |
| DEV10 | `swift test` in Surface package | Core document/history/relationship and networking/process policy tests. |
| DEV11 | Companion Tests | Caps chord/layer state and semantic/tool-prefix checks; harnesses/scripts rather than a single app release test suite. |
| DEV12 | Networking probe scripts | `secure_proxy_transport.py` and `vpn_federation_probe.py` isolate transport/provider experiments. Read policy/proposal for what their outcomes do and do not verify. |
| DEV13 | Semantic training/export tools | Training scripts, datasets/model experiments and structured shipping artifacts. Action training assets remain for research. Normal builds omit the old action models and structured parser resources; DeskNet research packaging remains separate. |
| DEV14 | Build/install script | `outputs/surface-edge/build.sh`: requires Apple Silicon, macOS 26, Xcode, XcodeGen and existing Apple signing identity. Builds both processes/helper, validates networking resource hashes, embeds resources, signs and replaces `/Applications/Nardo.app`. Stops previous Surface before replacement. |
| DEV15 | Build overrides | Default optimized release; `SURFACE_BUILD_CONFIGURATION=debug`, `SURFACE_COMPANION_CONFIGURATION=Debug`, and `SURFACE_SIGNING_IDENTITY` overrides. Source builds are not notarized public-release packaging by themselves. |

Sources: [Surface launch modes](../../outputs/surface-edge/Sources/SurfaceApp/main.swift), [verification](../../outputs/surface-edge/Sources/SurfaceApp/Verification.swift), [CPU checks](../../outputs/surface-edge/Sources/SurfaceApp/CPUVerification.swift), [benchmark](../../outputs/surface-edge/Sources/SurfaceApp/CanvasPerformance.swift), [native session](../../outputs/surface-edge/Sources/SurfaceApp/NativeAgentSession.swift), [Surface package](../../outputs/surface-edge/Package.swift), [companion tests](../../outputs/option-edge-search/Tests), [proxy transport probe](../../outputs/surface-edge/Tests/secure_proxy_transport.py), [federation probe](../../outputs/surface-edge/Tests/vpn_federation_probe.py), [structured model notes](../../outputs/option-edge-search/Semantics/structured/README.md).
# Bar words

[Feature index](../../FEATURES.md) · [Search and utilities](search.md) · [Surface Lite](lite.md) · [Citizens and guests](citizenship.md)

Added **October 17, 2026**. Small words the bar understands, each one a card. From the r/macapps scout ([macapps-2026-10.md](../scout/macapps-2026-10.md), "Bar words"). Both editions.

Each word belongs to an existing module, so switching the module off removes its card (no new tiles in the Settings grid):

| Module | Words |
| --- | --- |
| Calendar | `timer`, `pomodoro` (off also cancels running timers) |
| Media | `mute`, `unmute`, `mic`, `airpods`, `speakers`, `output` |
| Answers | `define`, `translate`, `color`, `:emoji` |
| Terminal | `kill :port` |

Cards are tile mosaics: rows of up to four, a short last row widens to fill, 8pt between and around. ←/→ walks tiles, ↵ acts. Tiles use the Settings card's squash-and-tip press. Icons: Solar bold-duotone (`solar-stopwatch`, `-cup-hot`, `-airbuds`, `-speaker`, `-headphones-round-sound`, `-bluetooth-wave`, `-usb`, `-monitor`, `-screencast-2`, `-soundwave`, `-book-bookmark`, `-dialog-2`, `-pipette`, `-emoji-funny-circle`, `-cpu-bolt`, `-close-circle`, `-check-circle`, `-copy`). The muted mic is `solar-microphone-3` with a drawn slash (Solar has no muted mic).

## Words

| ID | Typed | Card and ↵ |
| --- | --- | --- |
| BW01 | `timer 25m`, `timer 1h30m tea`, `tea timer 4m`, `timer 5 min`, `timer 1:30`, `timer 25` (minutes), `set a timer for 2 hours` | First tile: the new timer (dashed, clock, label from the leftover words). ↵ starts it and the bar goes away. Running timers follow as live tiles: countdown, a ring that empties, label, ✕ to cancel (↵ on a running tile cancels it). Max 24 h. |
| BW02 | `pomodoro`, `pomodoro 50 10` | 25 min focus (orange), then a 5 min break (green) starts by itself. `pomodoro 50 10` = 50 focus, 10 break. |
| BW03 | `timer`, `timers`; empty bar | Presets (5m, 10m, 25m pomodoro, 50/10 pomodoro), then running timers. With timers running, the empty bar shows the timer card above clipboard. |
| BW04 | `mute`, `unmute`, `mic`, `mic off` | One tile: input device name, state (Live green / Muted red), what ↵ does. ↵ sets it, toasts, and the bar goes away. Tap toggles in place. |
| BW05 | `airpods`, `speakers`, `headphones`, `output`, `output <name>`, `sound` | Every output device, matching names first. The current default has a filled media-pink icon; non-matches dim. ↵ switches the default output (system sounds follow if they were on the old default). |
| BW06 | `define <word>`, `what does <word> mean`, `meaning of …` | Headword, pronunciation, up to six senses (part of speech in indigo, examples and sub-senses dropped), two per row. ↵ opens Dictionary.app at the word. |
| BW07 | `translate <text>`, `… to spanish`, `… into ja`, `translate to french: <text>`; with no text (`translate`, `translate to french`, `translate clipboard into ja`, `translate this`) the **clipboard's text** (first 1,000 characters), so a text grab (Caps+S, T) then `translate` reads a sign, a subtitle or a game menu (added October 18; a small copy icon marks the source tile; nothing on the clipboard → a hint tile) | Source tile (detected language), result tile (teal), and a row of language chips; ←/→ changes the target and translates again. ↵ copies the result. Default target: what you typed; else your language when the text isn't in it; else the last target you copied. A language pair not downloaded yet shows `↵ download`, which asks the system to fetch it. |
| BW08 | `color`, `pick color`, `eyedropper` | First tile Pick, then the last seven picks as swatches (↵ or click copies the hex). Pick: the bar slides away, the system loupe samples anywhere, the hex goes on the clipboard and the bar comes back on that hex (the existing color card: swatch, rgb, hsl). |
| BW09 | `:fire`, `emoji thumbs up`, `emoji` | 12-across grid, up to 48, whole rows only (a ragged last row is dropped once there are two rows). Names are the system's own (CoreEmoji `AppleName.strings`, 1,900+, with flags and ZWJ sequences), plus everyday words (`lol`, `ok`, `party`, `shrug` …). Every typed word must start a word of the name. ↵ copies. Empty term: the common ones. |
| BW10 | `kill :3000`, `:3000`, `port 3000` | Each process listening on the port (TCP listen, or UDP bound): app icon or process tile, name, pid, protocol. ↵ arms (red outline, `↵ stop`), ↵ again sends SIGTERM and waits 1.5 s; still running → `↵ force` sends SIGKILL. Nothing there: a green `:3000 free` tile. Never stops itself, its parent or pid ≤ 1. Other users' processes aren't visible (no root). |

An explicit word wins over the calculator (`timer 25m` isn't 25 meters). `25m` alone is still a conversion.

## Implementation

- [EdgeBarWords.swift](../../outputs/option-edge-search/Option/UI/EdgeBarWords.swift): `BarWord.parse` (pure Foundation): duration grammar, port, emoji term, define, translate target (`BarWordLanguage`, English and native language names, codes, a few aliases like `mandarin`, `farsi`, `brazilian portuguese`).
- [EdgeBarWordCards.swift](../../outputs/option-edge-search/Option/UI/EdgeBarWordCards.swift): `BarWordCards` (module, rank, badge, tile count, ↵), the cards, `BarTranslator`, `ColorPick`, `EdgePortModel`.
- [EdgeTimers.swift](../../outputs/option-edge-search/Option/UI/EdgeTimers.swift): timers in `edge.timers.v1` (restored at launch, ended ones dropped). Each start schedules a `UNTimeIntervalNotificationTrigger`, so the alert comes even if the app quit; the notification delegate shows banners while the bar is in front. Notifications denied: a toast and the Glass sound instead.
- [EdgeSound.swift](../../outputs/option-edge-search/Option/UI/EdgeSound.swift): CoreAudio. Mute = `kAudioDevicePropertyMute` (input scope) when the device has one; otherwise input volume to 0, the old volume kept in `edge.mic.volumeBeforeMute.<uid>`. Property listeners refresh the cards.
- [EdgeWordLookup.swift](../../outputs/option-edge-search/Option/UI/EdgeWordLookup.swift): `DCSCopyTextDefinition` and the parser for its text.
- [EdgeEmoji.swift](../../outputs/option-edge-search/Option/UI/EdgeEmoji.swift), [EdgePorts.swift](../../outputs/option-edge-search/Option/UI/EdgePorts.swift) (`proc_listpids` → `PROC_PIDLISTFDS` → `PROC_PIDFDSOCKETINFO`; `lsof` when libproc finds nothing).
- Hooks in `Spotlight.swift`: card kind `.word` (`SpotCard.word`), made in `cheapCards`, idle timer card, tile count, ←/→, ↵. `SpotlightApp.swift`: timer restore at launch; Calendar off cancels timers.

## Journal

`timer.start`, `timer.cancel`, `timer.finish`, `sound.mic`, `sound.output`, `words.define`, `words.translate`, `words.color_pick`, `words.emoji`, `process.kill_port` in `docs/actions/registry.json`. Params are durations, kinds, device kinds, language codes, lengths and positions; never the translated text, the emoji term, device names or process names.

## Tests

`tools/bar-words-test/run.sh` (100 checks, offline): timer durations and labels (16 forms, 8 rejects), pomodoro, clock text; the timers model with a recording notifier (order, cancel, pomodoro break chaining, persistence, journal); mic/output words and device ranking; translate target parsing (12 forms, including `to my friend` staying text; 6 clipboard forms); port parsing plus a real listener of its own found through libproc and lsof, then stopped with SIGTERM; emoji ranking (`fire`, `thumbs up`, `lol`, `red heart`, `flag egypt`, prefixes); dictionary parsing on a fixed string and the live dictionary; ordinary queries (`#ff6600`, `music`) produce no word. It ends with a read-only audio probe (outputs, default output, input and whether it has a mute switch). It changes no device.

**Not exercised headless:** notifications (permission prompt, banner, sound), actually muting or switching output (the probe only reads), Translation downloads and real translations, `NSColorSampler`, the cards inside the real bar (they were rendered offscreen with stubbed app types during development, icons absent there), ↵/←/→ routing in `Spotlight.swift`. Both targets build.

## Not yet

A global mic chord; `airpods` only lists connected outputs (connecting one is `connect airpods`, [System words](system-words.md)); pausing timers; tearing a timer off into its own window; ↑/↓ by row in the emoji grid; emoji skin tones; translate needs macOS's languages installed or downloaded.
# Browser

[Feature index](../../FEATURES.md) · [Shortcuts](shortcuts.md) · [Availability](availability.md)

Integrated Browser hosts the actual Option `RootView`/WebKit engine in an Edge workspace. Spaces inside Browser are independent of the shell's three pinned actions. Existing code catalogs are not authoritative for current shortcut bindings or build availability.

## Organization and navigation

| ID | Feature / entry point | Behavior and limits |
| --- | --- | --- |
| B01 | Browser spaces | Space selector creates, activates, renames and deletes workspaces. Command-1…9 selects spaces by position. Delete has confirmation and removes its tabs/containers. |
| B02 | Space sign-in isolation | Workspace menu or `>` isolation command switches independent/shared website-data profiles. Cookies and storage persist per profile. |
| B03 | Tab tree / megatabs | Hierarchical tabs displayed as domain/site groups and cards, with favicons, titles and site colors. |
| B04 | Tab drag/drop | Arrange tab hierarchy/order/container membership with drag/drop; snapshot/core commands persist structure. |
| B05 | Containers | Create named groups, rename/collapse them, move tabs in/out, ungroup container without treating it as another shell workspace. |
| B06 | Pin tab | Context menu Pin/Unpin; separate from shell pinned actions and cross-space site pins. |
| B07 | Tab context actions | New Tab Above/Below, Duplicate Tab, Copy Link, Close Tab, container assignment and Shield/Unshield. |
| B08 | New tab query | Command-T opens command field in new-tab mode; URL vs search resolved by browser core. |
| B09 | New tab in site | Option-T scopes query to focused megatab's domain; empty Enter goes to site root. |
| B10 | Edit address | Command-L seeds current address into bottom input and navigates the captured tab in place. |
| B11 | Back/forward/reload | Site strip buttons, defaults Command-[ / Command-] / Command-R. Load progress is visible. |
| B12 | Web search / bangs | Core search template; native bangs `!g`, `!ddg`, `!b`, `!brave`, `!kagi`, `!yt`, `!w`, `!gh`, `!r`, `!so`, `!mdn`, `!npm`, `!crates`, `!maps`, `!images`. Template configuration UI is source-only in current companion. |
| B13 | Tab/group keyboard switching | Option-Tab cycles megatabs. Hold Option and tap backtick to arm/step sub-tab switcher; Shift reverses, release commits, Escape cancels, typed filter supported. Live preview is debounced. |
| B14 | Glance | Hold Option and hover sidebar tab for 0.38 seconds to start live peek; an open session switches instantly when hovering other tabs. Enter commits, Escape/click-outside dismisses. Native handoff surfaces are not glanceable. |
| B15 | Split browsing | Command-click tab cards to add/focus two side-by-side web panes; draggable divider and close-split button. Native surfaces stay out of web split layout. |
| B16 | Sidebar | Command-S toggles; trailing drag handle resizes. Browser space chrome remains reachable when sidebar is hidden. |
| B17 | Page zoom | Command-plus/minus/0; web zoom retained on tab. |
| B18 | New-tab page / pins | Clock/greeting, pinned-site tiles, frequent-site/essential affordances. Pin model is shared across spaces in its profile; dedicated add/remove pin commands are in core/standalone menu. |
| B19 | History panel | Command-Y / Site tools → History: timeline groups, search and page reopen. `> Clear Browsing History` clears entries. |
| B20 | Tab/page text search | Command shelf searches tab titles/URLs/indexed page text; active match can jump/highlight text in page. |
| B21 | Native URLs | `option://notes` and `option://games` resolve to handoff cards; supported deep links route to board/game. `option://journal` is not registered in this build. |

Sources: [BentoSidebar](../../outputs/option-edge-search/Option/UI/BentoSidebar.swift), [RootView/SplitState](../../outputs/option-edge-search/Option/UI/RootView.swift), [Keyboard](../../outputs/option-edge-search/Option/UI/Keyboard.swift), [Switcher](../../outputs/option-edge-search/Option/UI/Switcher.swift), [Glance](../../outputs/option-edge-search/Option/UI/Glance.swift), [NewTabPage](../../outputs/option-edge-search/Option/UI/NewTabPage.swift), [core command contract](../../outputs/option-edge-search/Option/Bridge/Command.swift), [NativeSurface](../../outputs/option-edge-search/Option/WebKit/NativeSurface.swift). Browser reducer/search implementation is linked in `Core/liboption_ffi.a`, so details beyond its Swift contract are not re-audited here.

## Page tools and protection

| ID | Feature / entry point | Behavior and limits |
| --- | --- | --- |
| B22 | Site strip | Stable origin, navigation, site protection/information, password actions, extensions and site tools. |
| B23 | Reader mode | Site tools toggles distraction-reduced article view for extractable page content. |
| B24 | Find on page | Command-F / Site tools; live match highlighting, match count and next/previous traversal. |
| B25 | Print / Save as PDF | Site tools or `> Print Page`; native print flow. Menu advertises Command-P, but standalone app menu registering it is excluded here. |
| B26 | Copy page screenshot | Site tools or `> Copy Page Screenshot`; current visible page snapshot to clipboard. Menu advertises Shift-Command-S; do not assume standalone menu shortcut is installed here. |
| B27 | Web highlights | Select page text → Mark; save quote and prefix/suffix anchor in Markdown vault; re-paint matching marks on page revisit. Page markup churn can survive via normalized-text anchoring; missing quotes cannot repaint. |
| B28 | Global ad/tracker blocker | Content lists from EasyList/uBlock/AdGuard compiled to WebKit filtering and cosmetic rules; `>` enables/disables and reloads policy. |
| B29 | Per-site protection | Site shield toggle and tab/container Shield/Unshield controls. Site panel reports current-profile cookie count and protection components. |
| B30 | Block element / undo | Site protection panel starts picker, stores cosmetic rule, undo removes most recent element block for site. |
| B31 | Protection activity | Page/session counters, recent activity, clear activity ledger; these are observed app counters rather than a claim that every tracker is detected. |
| B32 | YouTube protection | Bundled page scripts supplement content blocking for YouTube; site compatibility depends on page changes. |
| B33 | Browser compatibility | Configuration/remount policy handles WebKit site compatibility; external links can dispatch via macOS scheme handlers. |

Sources: [site controls](../../outputs/option-edge-search/Option/UI/RootView.swift), [Reader](../../outputs/option-edge-search/Option/WebKit/Reader.swift), [Find](../../outputs/option-edge-search/Option/UI/Find.swift), [BrainCapture](../../outputs/option-edge-search/Option/WebKit/BrainCapture.swift), [ContentBlocker](../../outputs/option-edge-search/Option/WebKit/ContentBlocker.swift), [Shield](../../outputs/option-edge-search/Option/WebKit/Shield.swift), [WebTab](../../outputs/option-edge-search/Option/WebKit/WebTab.swift).

## Downloads and media

| ID | Feature / entry point | Behavior and limits |
| --- | --- | --- |
| B34 | Download interception | WebKit file navigation/download links enter managed downloads; sanitize/collision-resolve names. Default directory `~/Downloads/Option`. |
| B35 | Progress and tray | Sidebar strip plus expanded download tray: progress, received/total bytes, status, rate/ETA where available, completion attention. Ledger survives relaunch; interrupted downloads do not magically resume. |
| B36 | Download actions | Cancel active download; Open or Show in Finder after completion; Remove from List or Clear completed entries. Removing ledger entries is separate from deleting downloaded bytes. |
| B37 | Download drag | Drag completed file to Finder/Surface/other destinations. File preview hover/detail supported. |
| B38 | Download preferences | Folder picker and Ask for each download implemented in full SettingsView; source-only settings route in current companion. |
| B39 | Sidebar media widgets | Active/recently-paused media entries; pager across multiple tabs, playback controls, progress, tab activation and PiP action. |
| B40 | Picture-in-picture | Floating video window rehosts live web content; hover-video controls/media detection and sidebar request can invoke it. |
| B41 | PiP controls | Play/pause, seek/skip, rate, mute; aspect-ratio resize and persistent frame. Remembers dismissing a particular video instead of repeatedly reopening it. |

Sources: [Downloads](../../outputs/option-edge-search/Option/WebKit/Downloads.swift), [DownloadsTray](../../outputs/option-edge-search/Option/UI/DownloadsTray.swift), [SidebarWidgetStack](../../outputs/option-edge-search/Option/UI/SidebarWidgetStack.swift), [PiP](../../outputs/option-edge-search/Option/UI/PiP.swift), [WebContentManager](../../outputs/option-edge-search/Option/WebKit/WebContentManager.swift).

## Passwords and extensions

| ID | Feature / entry point | Behavior and limits |
| --- | --- | --- |
| B42 | Password save/update prompt | Detect supported login submissions, offer save/update in the app's own Keychain vault; prompts tied to active HTTPS origin. |
| B43 | Password fill | Site key menu offers matching exact-host credentials and fills supported page forms. Not Safari/iCloud autofill; cross-host fill is intentionally restricted. |
| B44 | Password management pane | Search, reveal/copy/delete credentials implemented in full SettingsView; current companion does not present that view. |
| B45 | Extension store-page install | Banner on Chrome Web Store / Firefox Add-ons page installs package after confirmation of requested sites/capabilities. |
| B46 | Extension settings install | Paste store URL/Chrome ID or choose unpacked manifest folder, enable/disable/uninstall; management pane exists but current Settings scene is not wired to it. |
| B47 | Extension toolbar / site tools | Enabled extension icon opens measured popup or action click; site tools exposes enabled toggles. |
| B48 | Extension content scripts | Inject matching scripts/CSS with host/frame/timing guards; packaged resources served through extension scheme. |
| B49 | Extension bridge APIs | Storage (local/sync emulation/session), runtime messaging/background hosts, supported tabs/scripting/context-menu APIs and i18n translation through shim. Not complete Chrome/Firefox compatibility or cloud sync. |
| B50 | Declarative network rules | Supported static DNR block/allow/upgrade rules translated into WebKit rules; unsupported filters/actions can be dropped. `webRequest` interception is not implemented. |
| B51 | Extension context menus | Supported `chrome.contextMenus` items appear in native page right-click menu. |
| B52 | Runtime/memory handling | Budget live tabs, hibernate least-used inactive webviews with screenshot/session state, wake on demand; respond to memory pressure/content-process termination and surface issues via toasts. |

Sources: [PasswordManager](../../outputs/option-edge-search/Option/WebKit/PasswordManager.swift), [PasswordStore](../../outputs/option-edge-search/Option/WebKit/PasswordStore.swift), [PasswordsUI](../../outputs/option-edge-search/Option/UI/PasswordsUI.swift), [ExtensionsUI](../../outputs/option-edge-search/Option/Extensions/ExtensionsUI.swift), [ExtensionManager](../../outputs/option-edge-search/Option/Extensions/ExtensionManager.swift), [ExtensionShim](../../outputs/option-edge-search/Option/Extensions/ExtensionShim.swift), [DNR translator](../../outputs/option-edge-search/Option/Extensions/ExtensionAdblock.swift), [RuntimeHealth](../../outputs/option-edge-search/Option/App/RuntimeHealth.swift).
# Calendar

[Feature index](../../FEATURES.md) · [Citizens and guests](citizenship.md) · [Search](search.md) · [Surface Lite](lite.md)

The calendar card reads every account Calendar.app has (EventKit). It is in Lite. v2 (**October 17, 2026**) added meeting join, live countdowns, reminders and time-zone answers.

## Entry points

| ID | Entry point | Behavior and limits |
| --- | --- | --- |
| CAL01 | `cal`, `calendar`, `agenda`, `schedule`, `events`, `today`, `tomorrow`, `friday`, `next week`, `cal oct 20` | The card: the week as day chips (dots in each calendar's color), the chosen day's events as tiles, today's reminders under them. Click a day chip to show that day. Torn off or pinned tall, the week becomes timelines. |
| CAL02 | `add lunch with sam tomorrow 1pm`, `event …`, `new event …` | Dashed tile first; ↵ adds it to the calendar set in Settings → Calendar (default length there too). No clock time = all-day. `in 20 minutes` / `in an hour` now count as a time (they were read as all-day). A trailing `by` is dropped like `at`/`on`. |
| CAL03 | Countdown | On today's tiles: events under way show a red `now`; the next event and the meeting to join show `in 12m`, `in 1h 5m`, `in 4h` (red within 5 minutes). Minutes round up. The tile row redraws only when a label changes (each minute mark before the next event, starts, ends, reminder due times) through a `TimelineView` schedule; nothing ticks while the card is off screen. Ended events dim. |
| CAL04 | Meeting join | The meeting under way, or the next one today, with a conference link gets a green Join block (the service's name under the icon). ↵ on that tile, or a click on the block, joins. Choice: under way beats next, unless the next starts within 10 minutes and the current one has run 10+ minutes. All-day, declined and canceled events never count. Other events with a link show a small camera after their time. |
| CAL05 | `join`, `join meeting`, `join call`, `join next`; `join zoom` / `meet` / `teams` / `webex` / `facetime` / … | The card with the meeting selected; ↵ joins. A named service only picks that service's meetings. Nothing to join: ↵ does nothing. |
| CAL06 | Link extraction | URL field, then location, then notes. Plain text or HTML (`href`s, `&amp;` and numeric entities), links in `<…>` or parentheses, schemeless hosts (`zoom.us/my/x`). Unwraps Outlook Safe Links, `google.com/url?q=`, Proofpoint v2/v3, Barracuda and any redirector with the real URL in `url`/`q`/`u`/`a`/`target`. Services: Zoom (incl. ZoomGov, `zoommtg:`), Google Meet, Teams (`meetup-join`, `/meet/`, teams.live.com, `msteams:`), Webex, FaceTime, Slack huddles, Whereby, GoTo, Chime, Jitsi/8x8, Skype, BlueJeans, RingCentral, Gather, Around, Lifesize. Help pages, dial-in lists (`zoom.us/u/`, `tel.meet`) and bare hosts are skipped. |
| CAL07 | Joining | Zoom `/j/` links open as `zoommtg://…` and Teams `meetup-join` as `msteams:…` when an app handles that scheme (no browser hop); everything else opens the link. |
| CAL08 | Reminders row | Under today's events: incomplete reminders due today or earlier. Overdue first (red time or day), then by time, untimed last. A ring in the list's color; click or ↵ checks it off (squash-and-tip wobble, struck through, leaves after ~1.6 s); pressing again before it leaves un-checks. The bar stays open. No reminders and none being added: no row. |
| CAL09 | `reminders`, `todo`, `tasks`, `to do` | The card with selection on the first reminder; `Nothing due` when there are none. |
| CAL10 | `remind me to call mom at 5pm`, `remind me tomorrow to …`, `remind me in 20 minutes to …`, `reminder pay rent friday`, `todo renew passport by oct 30` | Dashed reminder chip first in the row; ↵ adds it to the default Reminders list. Same date reading as CAL02. A time adds an alarm at that time; no date or time = due today, no alarm. A time already past today means tomorrow. In the Calendar scope (the tool chip), `remind …` makes a reminder, anything else still an event. |
| CAL11 | Permissions | Calendars and Reminders are separate grants. The first ask is EventKit's own prompt; after a denial the permission guide opens System Settings at that list. The Calendar module tile shows the red dot when Calendars is missing. Reminders not yet asked: an `Allow reminders` chip in the row; denied: hidden except for `reminders` queries. Info.plist (both): `NSCalendarsFullAccessUsageDescription`, `NSRemindersFullAccessUsageDescription`. |

## Time zones (quick answers)

See [Search](search.md) Q76: `time in tokyo`, `tokyo time`, `what time is it in new york`, `3pm london in my time`, `3pm in tokyo`, `9:30am pst to cet`, `3pm london tomorrow in my time`, `1:30am new york on 2026-11-01 in utc`. ↵ copies the answer.

**Journal.** `calendar.add`, `calendar.open`, `calendar.join` (service, return/click, whether the app scheme was used, seconds from start), `reminders.add`, `reminders.complete` (done, overdue, timed, return/click). Registered in `docs/actions/registry.json`.

**Source.** `Option/UI/EdgeCalendar.swift` (card, tiles, countdown schedule), `EdgeCalendarCore.swift` (EventKit store, phrase parsing; shared with the iPhone app), `EdgeAgenda.swift` (values + `EdgeAgendaSource` protocol), `EdgeAgendaLive.swift` (EventKit reminders, join, journal), `EdgeMeetings.swift` (links, which meeting, countdown text), `EdgeTimeZones.swift`. Wiring: `Spotlight.cheapCards`, `Spotlight.startSub` (initial selection), `SpotlightAnswers.timeZone`.

**Tests.** `tools/calendar-test/run.sh`: the real sources against an in-memory agenda (no EventKit access): 36 link formats, meeting choice, countdown text and its redraw schedule (sweep), reminder/join/event phrases, time zones across US/EU/AU DST edges, tile order, selection and ↵ actions; renders the card to PNGs (`SHOTS=dir` keeps them).

**Needs a live session.** EventKit prompts and real accounts (Google/Exchange invites as EventKit actually delivers their notes), `zoommtg:`/`msteams:` handoff with the apps installed, alarms firing, the reminder linger timing on screen, the week window in tall mode.

**Not yet.** Google two-way sync (needs OAuth). Reminders in the tall week window. Undo for added events/reminders. Recurring reminders shown as such. Time-zone answers don't know the date for weekday names (`3pm london friday`); use `tomorrow` or a date.
# Citizens and guests

[Feature index](../../FEATURES.md) · [Layer and workspaces](layer.md) · [Availability](availability.md)

Sorted **October 13, 2026**, from the September 30 audit plus later work (terminal, calendar, media, workspaces, tiling, tear-off, swipe keyboard, co-typist).

Everything fits under Caps Lock, so "does it fit?" is never the question. The question is **does it get better by being here?**

1. **Does it compose?** It trades both ways with at least two existing layers.
2. **Does the brain get smarter?** It adds real actions and choices to the journal (`docs/actions/registry.json`), feeding the one intelligence.

**Citizens** pass both and are the product. **Guests** live in the stack but trade with nothing; they become optional, downloadable/uninstallable modules. Guests are not bad features, only optional ones.

## The three pipes

Every citizen plugs into at least one:

- **Text in:** dictation, co-typist, swipe keyboard. Text enters any field in any layer.
- **Things out:** clipboard, drag, tear-off. Content leaves one layer and lands in another.
- **The bar:** scoping and discovery through Spotlight.

Guests touch only the bar, and only to launch. **Quick test for a new idea:** which pipes does it plug into, and in which direction?

## Citizens

| Feature | What it trades with |
| --- | --- |
| Spotlight bar | It's how you reach everything; other layers get scoped through it; choices and abandons are the brain's richest data. |
| File search | Sends files into canvas, browser and clipboard; receives web captures, downloads and canvas docs; feeds the brain choice data. |
| Clipboard cards | Takes in from everything (browser, terminal, files, canvas) and drags out into everything. The connective tissue. |
| Canvas (Surface) | Takes in pastes, PDFs, web captures, images and files; gives back wiki-links, Spotlight discovery and capture targets. Biggest source of actions. |
| Browser | Shares Spotlight (tabs, history, live web preview); downloads go to the file index; highlights and captures go to the canvas; Glance follows the same glance→card→window path. |
| Terminal | Blocks drag out to clipboard and canvas; the peek card is native to the stack; it's where you supervise agents. |
| Dictation strip | Speech in, text into any field: browser, canvas, Spotlight, other apps ([Dictation](dictation.md)). |
| Calendar card | Takes natural language from the bar; would be fed by email and canvas; gives time context to everything. Reminders are things on it (checked off in place, added from the bar); meetings join from it ([Calendar](calendar.md)). |
| Workspaces, tiling, tear-off | The glue in space rather than data: every card can become a window, every window can tile. |
| Journal + action registry | The citizenship test itself; anything logged here feeds the one brain. |
| Co-typist | Ghost text in every text field across every layer, trained on your own writing. |
| [Notch island](notch.md) | Hosts dictation, the terminal's waiting notice, now playing and Caps chords instead of widgets of its own; its buttons are those layers' actions, and what you open it for lands in the journal. |
| [Menu search](menu-search.md) | The bar reaches into every other app: its menu items become typed commands (`menus.perform`) the brain can run, and the cheat sheet teaches their keys. |

## Borderline (one link away)

| Feature | Missing link |
| --- | --- |
| Swipe keyboard | Just another way to type today. Becomes a citizen once co-typist LM rescoring lands, sharing the brain with dictation and co-typist. |
| Media card | Takes from the browser (web media, PiP) but gives nothing back. Fix: transcript or timestamp captures dropped onto canvas or clipboard. |
| Passwords, extensions | Talk only to the browser. Fine as browser internals rather than layers of their own. |
| DeskNet desk cam | A citizen if drawings land on the canvas as ink or nodes, a guest if it's just a viewer. |
| Calculator and converters (Q33–Q61) | Words the bar understands, not layers. Useful ones copy a result to the clipboard; toys (8-ball, coin, text transforms) are pure guests that cost nothing. |

## Guests (optional modules)

| Feature | Why |
| --- | --- |
| Free VPN, Tor, HTTPS proxy (P01–P15) | A system service with no links to other layers. The clearest example of a module. |
| Games (G01–G21) | Library, Hydra search, Steam/ROM/APK/Windows launch, parties. The bar launches them, nothing flows back. |
| Windows apps via bottles | Same as games: launched from the bar, self-contained after that. |

## In progress

| Feature | Verdict |
| --- | --- |
| [Mail](mail.md) (needs-you card → thread window → Gmail) | Strong citizen. Built: the bar and its triage actions in the journal. To come: replies (dictation and co-typist in), attachments into files and clipboard, threads into calendar events. |
# Workspace commands

AI action interpretation was removed from the normal Spotlight input on October 4, 2026. Surface and Browser use ordinary search, explicit feature tags, menus and keyboard shortcuts. Spotlight no longer rewrites requests into action scopes, previews inferred edits or waits for command inference on Enter. Action models are omitted from the app bundle.

The typed executor, training assets and native experimental adapters remain available for model development. The operations below describe that retained implementation, not current Spotlight functionality.

## Retained experimental operations

[Feature index](../../FEATURES.md) · [Detailed implementation notes](../../outputs/surface-edge/WORKSPACE-COMMANDS.md)

The removed input-bar integration previously accepted requests against the focused Surface or Browser workspace. These are bounded English operations inferred by a local structured model, then checked by a typed executor. Examples describe intended supported requests; exact wording acceptance depends on the model's confidence and current context.

## Surface operations

| ID | Operation | Entry/example and semantics |
| --- | --- | --- |
| C01 | Create shapes | `draw two red rectangles`; squares/rectangles, circles/ellipses/ovals, rings and lines; count, dimensions, position, fill, outlines, layout and gap. |
| C02 | Add text | `add text: Hello, world`; inserts literal text on canvas/image, optionally coordinates/color/font size. In text note, append literal text separated by paragraphs. |
| C03 | Replace text | Replace selected text objects' literal content. Non-text targets are rejected. |
| C04 | Select/filter | Select drawn objects by type/color, select all or clear selection. Child placements are not included in shape filters. |
| C05 | Fill/outline styling | `make them blue`, `add an outline`; add/remove outline, set color/width. Preserve unspecified fill/outline properties. Color-only edits update all outline layers without replacing their identities/order/scaling/enabled states. |
| C06 | Move by distance | `move them 20 pixels right`; left/right/up/down movement. Movement/property relationships can affect linked objects; preview discloses linked effects. |
| C07 | Position coordinates | Set objects to canvas x/y coordinates. |
| C08 | Place relative to anchor | Place above/below/left/right or centered on an identified anchor with gap and alignment; uses visible bounds. Exact UUID targeting is also exposed to native test adapter. |
| C09 | Resize / scale | `resize them to 120 by 80`; explicit dimensions or named bigger/smaller/double/half/wider/narrower/taller/shorter changes. Unsupported scale names/missing dimensions reject rather than inventing a size. |
| C10 | Arrange | `arrange them in a column`; row/column/grid with explicit/default gap. |
| C11 | Align | Left/right/top/bottom/center against selected object geometry. Separate from persistent inspector alignment relationships. |
| C12 | Duplicate | Copy selected drawn objects with new IDs, offsets and bounded copy count. |
| C13 | Delete | Remove selected drawn objects and clean their relationship membership. |
| C14 | Group / ungroup | Group creates Move together relationship; ungroup removes selected members from movement/association rules. |
| C15 | Stack order | Send selected objects to front or back. |
| C16 | Rename active surface | Extract literal name and rename current content. Executor only accepts workspace target; selected-object rename is not currently supported despite the older command summary. |
| C17 | Add notebook page | Add canvas/text page in notebook context. Must be a standalone request. |
| C18 | Undo / redo | Standalone workspace request; current context/selection restored appropriately. |
| C19 | Follow-up references | Selection or recent command-created/selected objects on the same surface can satisfy “them”/recent references. Missing/stale target asks for selection. |
| C20 | Compound requests | `add an outline, make them blue`; stage complete edit and commit one undo moment. Unsupported clauses reject the full request. |

## Preview, context and limits

| ID | Capability | Behavior |
| --- | --- | --- |
| C21 | Non-mutating preview | Show title/detail/defaults before Enter; no document edit during interpretation. |
| C22 | Atomic commit | Store before/after plan and check token against active content, selection, note draft and history moment. Stale plan refreshes; never applies to a different context silently. |
| C23 | Capability validation | Fixed operation/argument schema, valid type/color/numbers, confidence gate and locked-object checks. Low-confidence, unsupported or malformed requests stay blocked. |
| C24 | Read-only protection | History preview rejects edits until Return to present or Branch here. |
| C25 | Surface-kind routing | Drawing commands require canvas/image; text notes accept appropriate text actions. Notebook overview requires entering a page. |
| C26 | Local inference | 33.6-million-parameter encoder compiled through Core ML; token/span extraction and executor are local. No generated code or remote model-service document upload in this path. |

Current bounded values include 1–64 created objects; shape dimensions/coordinates up to 10,000; inserted text font size 1–500; rename length up to 128. Semantic creation bridge limits title to 128 and body to 4096 characters. Other operations enforce their own ranges in the executor. Named fill colors are blue/red/green/yellow/orange/purple/pink/teal/cyan/white/black/gray/grey or six-digit hex. Rotation, gradients, shadows and arbitrary command execution are unsupported.

## Browser operations

| ID | Operation | Behavior |
| --- | --- | --- |
| C27 | Back / forward | Preview active Browser navigation; direction requires available history. |
| C28 | Reload | Refresh captured active tab. |
| C29 | Close tab | Close current tab through Browser core command. |
| C30 | New tab | Create Browser tab through its core command. |
| C31 | Browser context check | Capture tab ID and URL; verify both and Browser focus before execution. Surface edits do not target a background canvas while Browser is focused. |

Normal global search and explicit feature tags remain available. Workspace edits are experimental only. Local model scores do not establish general language understanding; the native experimental adapter's choice scores are not calibrated parser confidence.

Sources: [Surface executor](../../outputs/surface-edge/Sources/SurfaceApp/WorkspaceCommands.swift), [shared structured parser](../../outputs/surface-edge/Sources/SurfaceApp/SurfaceCommandParser.swift), [companion routing/commit](../../outputs/option-edge-search/Option/UI/Spotlight.swift), [workspace language adapter](../../outputs/option-edge-search/Option/Semantics/WorkspaceLanguage.swift), [semantic creation](../../outputs/surface-edge/Sources/SurfaceApp/SemanticActions.swift), [model training/results](../../outputs/option-edge-search/Semantics/structured/README.md).
# Notes, Games and voice

[Feature index](../../FEATURES.md) · [Availability](availability.md) · [Shortcuts](shortcuts.md)

## Option Notes

**External handoff.** `@notes`, Notes results, `> Open Notes` and Notes creation/capture launch installed **Option Notes** via Launch Services / `option-notes://…`. It is not embedded by Surface's build script. This checkout contains the Notes editor implementation, but the separately installed app's version is not verified by this audit. These rows describe that source and preserve the full capability inventory.

| ID | Feature / entry point | Behavior and limits |
| --- | --- | --- |
| N01 | Global discovery | Spotlight owns search, recents, switching and creation. Results include type, page/card counts, preview text, freshness and backlink count. |
| N02 | Canvas / notebook creation | Scoped creation card or editor plus menu; create named canvas/notebook and open active document. |
| N03 | Quick capture | `note …`, creation body or Option-Enter into selected board. External capture defaults to day's session board; daily spine is append-only Markdown. |
| N04 | Quiet document path | Parent crumbs orient/navigate; double-click current title to rename. No separate Notes library route in this implementation. |
| N05 | Infinite canvas/camera | Pan/zoom with saved per-board camera; fit/back-to-content affordance when contents leave viewport. `CameraSurface` means viewport control, **not photo capture**. |
| N06 | Selection/manipulation | Single/additive/marquee selection, drag, arrows/Shift-arrows nudge, resize handles, rotation, corner-radius and stroke endpoint handles. Locks protect edits. |
| N07 | Text editing | Text elements with font/weight/size/color/wrap; selection resize supports proportional text or deliberate Shift deformation via independent text scaling. |
| N08 | Shape revolver | Scroll/drag/click chamber immediately arms Rectangle, Ellipse, Polygon, Diamond or Star. |
| N09 | Configurable polygons/stars | Vertex count; star inner ratio, rounding and inversion. Filled shape geometry follows those settings. |
| N10 | Lines/arrows/ink | Dedicated Line/Arrow/Pen tools; pen stays armed between strokes. |
| N11 | Images | Drop/paste supported images, downscale into image store, place at canvas center with initial display size; image corner radius styling. |
| N12 | Math elements | Finish editing text wrapped in `$…$` or `$$…$$` to convert it to math. LaTeX source renders through KaTeX with measured height; no dedicated math toolbar button is present in current Topbar. |
| N13 | Style inspector | Fill, stroke/color/width, radius, opacity, font/weight/size, rotation, shape-specific controls and stack order. |
| N14 | Layers rail | Element/group list with ordering, visibility, locking and deletion; contextual rail participates in layout rather than covering canvas. |
| N15 | Groups | Group/ungroup selection, manipulate groups, flatten/unflatten group appearance. |
| N16 | Turn item/group into board | Inspector creates a navigable board association; embed cards/board references enter nested content. |
| N17 | Wiki-links/backlinks | `[[Board Name]]` rendering; Command-click follows, dangling links can create a board. Backlink index rebuilt with persistence/search. |
| N18 | Copy/paste/duplicate/delete | Preserve grouping; duplicate/paste deep-copies referenced board/notebook subtrees so copies edit independently. Deleting linked elements prunes owned descendants/PDF/cache/history; z-order front/back or one step forward/back. |
| N19 | Undo/redo | Per-context board/page history, coalesced style changes and no-op handling. Separate from Surface's persistent history tree. |
| N20 | Bounded notebook pages | Page-local elements/groups, configurable sheet slots/scales and active page. Add, insert blank after, delete page. |
| N21 | PDF notebooks | Drop PDF to create annotatable notebook with PDF backgrounds; sidecar preserves original PDF. Canvas/PDF coordinates stay page-local. |
| N22 | Notebook cover preview | PDF/first-page cover rendering for embedded notebook card. |
| N23 | Content indexing | Search board names, text, math and notebook/page data, plus web capture highlights; recents tracked separately. |
| N24 | Web capture files | Browser Mark writes URL/title/quote anchors to vault page Markdown; page marks rehydrate on revisit. |
| N25 | Vault persistence | Manifest plus one JSON per board, page/daily Markdown and PDF sidecars; images are downscaled and embedded as data URLs in board data. 2-second autosave and close/quit flush. |
| N26 | Migration and backups | Additive migration from CanvasNotes, legacy blob backup, old taxonomy migration; launch-time vault/PDF backup with bounded retention. |
| N27 | Legacy native links | `option://notes/board/…` and legacy subject/concept/library paths route to board IDs; frontmost tab hands off, background restore does not pop windows. |

Sources: [NotesRuntime/handoff](../../outputs/option-edge-search/Option/Essentials/Notes/NotesRuntime.swift), [suite launcher](../../outputs/option-edge-search/Shared/OptionSuiteLauncher.swift), [NotesRootView](../../outputs/option-edge-search/Option/Essentials/Notes/NotesRootView.swift), [NotesStore](../../outputs/option-edge-search/Option/Essentials/Notes/NotesStore.swift), [Topbar](../../outputs/option-edge-search/Option/Essentials/Notes/Topbar.swift), [Inspector](../../outputs/option-edge-search/Option/Essentials/Notes/Inspector.swift), [Layers](../../outputs/option-edge-search/Option/Essentials/Notes/Layers.swift), [Notebook](../../outputs/option-edge-search/Option/Essentials/Notes/Notebook.swift), [Persistence](../../outputs/option-edge-search/Option/Essentials/Notes/NotesPersistence.swift), [WikiLink](../../outputs/option-edge-search/Option/Essentials/Notes/WikiLink.swift).

## Games

**Integrated library, external launch dependencies.** `@games`, a Games card/Option-F or `> Open Games` opens library owned by the companion. Game execution uses Steam, macOS applications, compatibility software, emulators or Android tools as appropriate. Format routing is not a guarantee that every game runs.

| ID | Feature / entry point | Behavior and limits |
| --- | --- | --- |
| G01 | Library storefront | Continue-playing shelf, updates, all-games grid and in-place game detail. Command-F searches; Escape backs out/clears search/hides window according to state. |
| G02 | Library discovery | Merge persisted Option/Berrry libraries, Steam library folders/manifests, Heroic installs, native game apps and discovered executable/ROM locations. Deduplicate paths and remember hidden entries. |
| G03 | Game metadata | Names, covers, versions/builds, installed/update state, launch target, recorded playtime/last played where available; fetch/cache metadata from sources/Steam. |
| G04 | Drop games | Accept `.app`, `.exe`, `.msi`, `.apk`, or folder with chosen executable. APK copied into library storage. Not every arbitrary file is a game importer. |
| G05 | Steam launch | Steam-managed installations launch via Steam protocol/runtime rather than running executable directly. |
| G06 | Native app launch | Open enclosing `.app` or registered path with macOS. |
| G07 | Windows launch | Discover GPTK/CrossOver/Wine/Whisky-compatible runtime and prefix; executable or installer launch with compatibility environment/logging. Missing runtime/errors surface through notifications. |
| G08 | ROM launch | Route supported console ROM extensions to available appropriate emulator (OpenEmu/RetroArch, Dolphin, Ryujinx, Azahar, PPSSPP, etc.), otherwise Launch Services. Emulators are external. |
| G09 | Android APK launch | Use adb on attached device/emulator or boot provisioned AVD; optionally mirror through scrcpy. Needs Android SDK/AVD/tools, not bundled by this Edge project. Background launch reports progress/failure. |
| G10 | Local launch feedback | Validate target existence, surface launch errors, record launches/playtime where observed. Android `.launched` return is optimistic; later failure can arrive. |
| G11 | Library removal | Result context menu / Delete confirmation removes or hides launcher entry, with reveal-in-Finder action. Removal is distinct from uninstalling bytes. |
| G12 | Hydra/source search | Games query searches cached/online indexed catalogs and hydrates source JSON, ranks/deduplicates candidates, shows source/version/artwork and resolves provisional matches. Requires reachable external feeds. |
| G13 | Source choice | Result controls choose among alternatives and navigate source/download page; existing local installation can be associated with result. |
| G14 | Download dispatch | Current activation opens source in integrated Browser; magnet URLs dispatch to external handler. This path does not itself download torrents or install a game. |
| G15 | Update discovery | Compare installed versions/builds to matching source candidates; `@games updates`/related update queries and library update checks show candidates. Source/version heuristics are not authoritative vendor update guarantees. |
| G16 | Update UI | Play-current/update choice, update-page action and result shelf. Initial activation opens the source in Browser; refreshing update sources can populate an embedded Hydra browser card. |
| G17 | Update installer (conditional flow) | The refreshed update shelf's Hydra mini-browser connects completed downloads to extraction/install, target selection, version registration and optional launch afterward. Entry-dependent; not every source/download result runs this installer, and end-to-end installation is not verified by this audit. |
| G18 | Game deep link | `option://games/<id>` or supported party link routes detail selection and library. External URL registration/forwarding must also exist; Edge's own URL handler only accepts search summon scheme. |
| G19 | Host nearby party | Detail → Host Party uses MultipeerConnectivity local discovery; advertise game/lobby, show participants/host. No central internet matchmaking service in this implementation. |
| G20 | Join / invite / leave | Nearby lobbies for matching game, Copy Invite, join retries and Leave. Invite transport requires receiving URL routed to GamesRuntime. |
| G21 | Launch party / solo | Coordinate participants' launches; Play Solo bypasses party. This synchronizes lobby/launch messages, not arbitrary game's gameplay/netcode. |

Sources: [GamesStore](../../outputs/option-edge-search/Option/Essentials/Games/GamesStore.swift), [GamesLibraryView](../../outputs/option-edge-search/Option/Essentials/Games/GamesLibraryView.swift), [GameDetailView](../../outputs/option-edge-search/Option/Essentials/Games/GameDetailView.swift), [GamesRuntime](../../outputs/option-edge-search/Option/Essentials/Games/GamesRuntime.swift), [GamesMultiplayer](../../outputs/option-edge-search/Option/Essentials/Games/GamesMultiplayer.swift), [AndroidRuntime](../../outputs/option-edge-search/Option/Essentials/Games/AndroidRuntime.swift), [Spotlight source activation/retained mini-browser](../../outputs/option-edge-search/Option/UI/Spotlight.swift).

## Voice and dictation

Caps+D dictation (Apple's on-device recognizer by default, Parakeet-MLX where installed), other-app insertion and per-app tone: [Dictation](dictation.md). The rows below are the older Browser entry.

**Integrated Browser entry, external recognizer.** Fixed Option-Space toggles recording while Browser owns keyboard context. `parakeet-mlx`, its model/cache and supporting executables must be available. The global hotkey, other-app insertion and endpoint polish are wired by the excluded standalone app entry, not Edge's Browser configuration. The full Voice settings pane exists in source but is not presented by this companion's Settings scene.

| ID | Feature / entry point | Behavior and limits |
| --- | --- | --- |
| V01 | Recording overlay | Separate temporary voice panel, duration/waveform, recording/processing/inserted/failed state. Microphone permission required. |
| V02 | Toggle / stop / cancel | Option-Space starts/stops; Enter in panel stops/transcribes; Escape cancels and cleans recording/session. Fixed hotkey is not remappable. |
| V03 | Local streaming recognition | Parakeet MLX subprocess/sidecar streams audio/transcript, reconciles stable/final results and supports CLI fallback. Model may require initial download. |
| V04 | Edit live transcript | Editable transcript remains available while recording; handles auto-scroll/user edits and authoritative recognizer update reconciliation. |
| V05 | Transcript cleanup/context | Edge falls back to local baseline cleanup. It captures WebKit field/selection context for insertion. Rich correction/context-aware rewriting belongs to the optional endpoint polisher below; spoken words do not become workspace tool commands. |
| V06 | Optional AI polish (source only wiring) | Standalone entry connects configured OpenAI-compatible endpoint/model/key to rewriting with four-second timeout and fallback checks. Edge leaves this callback unset and uses local cleanup. Endpoint may be local or remote; separate from Surface command inference. |
| V07 | Browser insertion | Insert transcript into supported active WebKit input/contenteditable, respecting captured selection/context. |
| V08 | Other-app insertion | Caps+D: captured field in any app via Accessibility, paste (clipboard restored, not recorded) or typed keys; see [Dictation](dictation.md). |
| V09 | Recognizer configuration (source UI) | Command path, model name, cache directory, Test/version probe, cleanup/AI/restore-clipboard toggles in VoicePane. Defaults stored independently of document. |
| V10 | Permission status | Microphone/Accessibility Allowed/Needs approval/Blocked model and settings links; throttled prompts. Settings UI route gap applies. |

Sources: [VoiceFlow](../../outputs/option-edge-search/Option/UI/VoiceFlow.swift), [Browser configuration](../../outputs/option-edge-search/Option/UI/EdgeBrowserWindow.swift), [VoicePane](../../outputs/option-edge-search/Option/UI/SettingsView.swift), [Permissions](../../outputs/option-edge-search/Option/UI/Permissions.swift), [AI client](../../outputs/option-edge-search/Option/AI/AIClient.swift), [AI key store](../../outputs/option-edge-search/Option/AI/AIKeyStore.swift).
# Networking, data and recovery

[Feature index](../../FEATURES.md) · [Full networking policy](../../outputs/surface-edge/SECURE_PROXY.md)

## Free VPN and advanced proxy

These rows describe implemented policy and controls. The existing networking notes explicitly distinguish isolated probes from unverified privileged routing/leak/crash testing. Do not treat this inventory as a fresh security verification.

| ID | Feature / entry point | Behavior and limits |
| --- | --- | --- |
| P01 | Free VPN panel | Surface menu → Free VPN…; status, helper approval/install, country selection, Connect, Refresh Status, Disconnect & Restore Network. Networking stays off until Connect. |
| P02 | Privileged helper approval | Signed helper installed via macOS service management; administrator approval in General → Login Items & Extensions. Installing alone changes no routes/DNS/firewall. |
| P03 | Account-free Tor transport | Bundled Tor/runtime libraries and sing-box engine. No account/subscription or Homebrew prerequisite for this transport. |
| P04 | Exit country | Bounded HTTPS Onionoo inventory offers countries with exits; refresh availability, restrict ExitNodes and verify tunnel-observed exit country. Unavailable country fails without silent alternate country/transport. |
| P05 | TCP tunnel / encrypted DNS | System tunnel forwards TCP, sends DNS over HTTPS through transport and blocks UDP/LAN/unsupported traffic. Ordinary TCP can carry plaintext; no Tor Browser fingerprint protection. |
| P06 | Independent firewall guard | Allows defined loopback/exact-tunnel/bootstrap/relay exceptions; denies other physical traffic including IPv6. Validate installed rules/table; privileged software can change them. |
| P07 | Startup/health checks | Connect returns promptly with Disconnect available; 3-minute startup deadline, health/country checks every 20 seconds. Connected shown only after real tunnel-bound probe. |
| P08 | Fail-closed behavior | Tunnel/engine/helper failure or Surface quit leaves runtime deny guard armed. No automatic direct reconnect. Runtime guard does not survive reboot. |
| P09 | Disconnect/recovery | Stop identified private processes, remove only Surface anchor/PF enable reference and restore ordinary network. Reopen app after failure and Disconnect; precise emergency recovery in policy doc. |
| P10 | Resource integrity and XPC identity | Hash-pin engines/libraries/data, private root runtime copies, authenticate app/helper by team + bundle ID; accept typed requests rather than arbitrary privileged executable/config. |
| P11 | Advanced manual HTTPS proxy | Enter endpoint, Verify, Connect accepted result. TLS 1.3 to proxy with standard CA/hostname validation; HTTPS destination probes. Only TLS-classified TCP forwarded in this stricter mode. |
| P12 | Public-proxy discovery | Find Public Proxies / Cancel scan; sample up to 96, four probes concurrently, at most eight verified results. Reject private/reserved/malformed addresses; feed's HTTPS label isn't evidence of encryption. |
| P13 | Expiring verification | Proxy verification expires after five minutes; activation performs real routing verification. DNS via encrypted proxy; endpoint IP resolved/pinned before activation. |
| P14 | No traffic logging | Does not retain traffic, proxy credentials, cached DNS answers or exit-IP logs. Control/status data and Tor private runtime state remain. Operators can still observe destinations/timing; applications own their end-to-end TLS validation. |
| P15 | Federation experiment | Isolated OpenVPN UDP / OpenVPN TCP-over-Tor provider probes in Tests and proposal document. **Not current Free VPN UI/backend**, no volunteer relay infrastructure shipped. |

Tor mode prevents UDP calls/games/QUIC/BitTorrent and local-network access while active, including local game parties. Advanced HTTPS proxy has a different TLS-only TCP restriction. Country counts/provider availability are live data, so this reference intentionally avoids fixed availability promises.

Sources: [SecureProxy panel/controller](../../outputs/surface-edge/Sources/SurfaceApp/SecureProxy.swift), [privileged helper](../../outputs/surface-edge/Sources/SurfaceProxyHelper/main.swift), [proxy policy](../../outputs/surface-edge/Sources/SurfaceCore/SecureProxyPolicy.swift), [Tor policy](../../outputs/surface-edge/Sources/SurfaceCore/SecureTorPolicy.swift), [integrity](../../outputs/surface-edge/Sources/SurfaceCore/TorIntegrity.swift), [SECURE_PROXY](../../outputs/surface-edge/SECURE_PROXY.md), [federation proposal/probe results](../../outputs/surface-edge/VPN_FEDERATION.md).

## Local data locations

`~` is the current macOS user's home. Locations come from current source; actual files depend on which feature has run. Surface's own stores are separate from the shared Option suite stores.

| ID | Data / behavior | Location or boundary |
| --- | --- | --- |
| D01 | Surface current recovery | `~/Library/Application Support/Nardo/Recovery.surface` |
| D02 | Surface replacement archives | `~/Library/Application Support/Nardo/Archive/*.surface`; archive edited workspace before New/Open replaces it. |
| D03 | Saved workspace | User-selected `.surface`; all branches and deduplicated embedded image assets. |
| D04 | Surface navigation/cameras/layout | UserDefaults per root/content IDs; Edge layout keys per workspace. Preferences are not the `.surface` artwork. |
| D05 | Layer diagnostics | `~/Library/Caches/com.hashawn.nardo/layer-events.json`; bounded/content-free. Semantic content lookup separately writes `semantic-catalog.json` in this cache directory. |
| D06 | Browser core database | `~/Library/Application Support/option/option.sqlite`; intentionally shared with existing Option suite. |
| D07 | Browser history | `~/Library/Application Support/option/history.db`; independently clearable. |
| D08 | Website data | Persistent WebKit profile stores; isolated spaces use their own cookie/storage profile. Current-site cookie count comes from active profile. |
| D09 | Clipboard | `~/Library/Application Support/Raycastish/Clipboard/clipboard.db` and `blobs/`; shared store, pin/retention/pruning. Ordinary recorded text is locally persisted, not a credential vault. |
| D10 | File search index | `~/fssearch/index.db` read-only; `OPTION_FILESEARCH_DB` / `OPTION_SURCH_DB` override. External crawler supplies index. |
| D11 | Option Notes vault | `~/Library/Application Support/option/essentials/notes/vault/`: manifest, `boards/`, `pages/`, `daily/`; inline image data in board JSON, PDF sidecars in sibling `pdf/`, backups in sibling `backups/`. |
| D12 | Notes legacy migration | Copy from `~/Library/Application Support/CanvasNotes` when no current store; preserve legacy data backups and existing vault. External app launch version can affect behavior. |
| D13 | Download ledger/files | `~/Library/Application Support/option/downloads.json`; bytes default to `~/Downloads/Option`, overridable core preference. |
| D14 | Games catalog/library/cache | Persisted Option catalog under `option/essentials/games`; read legacy Berrry library candidates, caches/covers, version/playtime UserDefaults, game launch logs. Installed/extracted files include `~/Downloads/Option/Games`. |
| D15 | Extensions | `~/Library/Application Support/option/extensions`: packages, ledger, per-extension storage. `storage.sync` is compatibility emulation, not a hosted account sync service. |
| D16 | Passwords | macOS Keychain service `com.hashawn.option.passwords`; exact host/account and this-device-when-unlocked accessibility. Passwords never placed in the app's plaintext data files. |
| D17 | AI key/config | AIKeyStore + core endpoint/model settings; separate from browser website password vault. |
| D18 | Search/shortcut preferences | UserDefaults for ranking, shortcut overrides, currency cache, pinned actions, voice and awake preferences. |
| D19 | Runtime-health marker | `~/Library/Application Support/option/edge-search-runtime-health.json`; watchdog/diagnostic health state and issue toasts. |

Sources: [StudioStore](../../outputs/surface-edge/Sources/SurfaceApp/StudioStore.swift), [companion AppPaths](../../outputs/option-edge-search/Spotlight/App/SpotlightApp.swift), [ClipboardStore](../../outputs/option-edge-search/Option/WebKit/ClipboardStore.swift), [NotesPersistence](../../outputs/option-edge-search/Option/Essentials/Notes/NotesPersistence.swift), [Downloads](../../outputs/option-edge-search/Option/WebKit/Downloads.swift), [GamesStore](../../outputs/option-edge-search/Option/Essentials/Games/GamesStore.swift), [ExtensionManager](../../outputs/option-edge-search/Option/Extensions/ExtensionManager.swift), [PasswordStore](../../outputs/option-edge-search/Option/WebKit/PasswordStore.swift).

## Permissions, network use and maintenance

| ID | Feature/boundary | Behavior / availability |
| --- | --- | --- |
| D20 | Microphone | Needed for dictation; state/request/settings links implemented. |
| D21 | Accessibility | Other-app context/paste insertion implementation needs it; Edge's current voice callback uses WebKit only. Status and prompt throttling exist in source. |
| D22 | Network helper privilege | Administrator approves networking helper; Connect arms routes/firewall, Disconnect releases them. |
| D23 | Browser clearing | `>` clears history/cache now; full PrivacyPane can clear cookies/cache/local storage/IndexedDB across stores but UI is not presented here. Saved Keychain credentials/open tabs are separate. |
| D24 | Sign-in import (source only entry) | Explicit browser/file picker imports supported cookies into active profile; Chromium/Safari/Firefox parsing and Chromium password import machinery exist. The standalone import menu/onboarding that presents this flow is excluded. |
| D25 | Actual online services | Browsing/previews, filter lists, currency rates, extension stores, game metadata/Hydra sources and Tor inventories contact their respective sources. Local-first does not mean all features are offline. |
| D26 | Local vs endpoint models | Surface command inference/search embeddings are local; voice recognition external local CLI. Standalone voice-polish wiring can send transcript/context to a configured endpoint; that callback is unset in Edge. |
| D27 | In-app feedback | Toasts and notification ledger record download/game/runtime errors/completion. A standalone menu-bar notification controller is not present in current target source inventory. |

Sources: [Permissions](../../outputs/option-edge-search/Option/UI/Permissions.swift), [PrivacyPane](../../outputs/option-edge-search/Option/UI/SettingsView.swift), [CookieImporter](../../outputs/option-edge-search/Option/WebKit/CookieImporter.swift), [Carryover](../../outputs/option-edge-search/Option/WebKit/Carryover.swift), [AppNotifications](../../outputs/option-edge-search/Option/UI/AppNotifications.swift), [Toasts](../../outputs/option-edge-search/Option/UI/Toasts.swift).

## Carry-over from Surface Edge (renamed 2026-10-19)

| ID | Feature | Behavior |
| --- | --- | --- |
| D27 | First-launch migration | `NardoMigration` (SurfaceApp and the companion) copies, never moves: `Application Support/SurfaceEdge` → `Application Support/Nardo` (APFS clone, staged then renamed so the two processes can't both copy), the `studio.surface.edge` and `studio.surface.edge.search` preference domains into `com.hashawn.nardo` / `com.hashawn.nardo.search` when the new domain is empty, and the companion's WebKit / HTTPStorages website data so the browser stays signed in. Marker: `Application Support/Nardo/.migrated-from-surface-edge`. |
| D28 | Mail password fallback | If `com.hashawn.nardo.mail` has no item, Mail reads `studio.surface.edge.mail` and re-saves it under the new service. The old item is left in place. |
| D29 | Not carried over | macOS permissions (Accessibility, Microphone, Input Monitoring), the Free VPN helper approval and Launch at Login are tied to the bundle ID and must be granted again. The old `surface-edge-search://` URL scheme is still accepted. |
# Dictation (Caps+D)

[Feature index](../../FEATURES.md) · [Citizens and guests](citizenship.md) · [Surface Lite](lite.md) · [Notes, Games and voice](companions.md#voice-and-dictation)

Updated **October 18, 2026**. Caps+D starts a take, Caps+D again ends it, Esc cancels. Apple's on-device SpeechAnalyzer hears by default (every macOS 26 Mac has it, nothing to install); Parakeet-MLX only where its CLI is installed. The s1-mini editing pass runs only where its model and `llama-server` are installed; everywhere else the rule-based cleanup is the editing pass. In both Surface Edge and Lite.

## Engine

| Settings → Dictation → Engine | Lite | Full |
| --- | --- | --- |
| Automatic (default) | Apple. Never looks for Parakeet | Parakeet when installed, else Apple |
| Apple | Apple | Apple |
| Parakeet | Parakeet when installed, else Apple | same |

"Installed" means the `parakeet-mlx` command resolves to a Python environment and the bundled sidecar exists (detected per take, never assumed). With Parakeet installed but its weights missing, Apple hears while they download. The Engine row offers the choice only where Parakeet is installed; otherwise it reads "Apple, on this Mac".

**Language.** The first that Apple supports of: Settings → Language; the keyboard's language, when it isn't English and differs from the system's (a French layout says something, U.S./ABC doesn't); the system locale; the preferred languages; en_US. **SpeechTranscriber** (the long-form model, 30 locales) hears when it has the language; **DictationTranscriber** (54 locales) covers the rest: Arabic, Hebrew, Russian, Hindi, Turkish and others. A region it lacks maps to one it has (en_EG → en_GB or en_US). The Language menu lists both modules' locales.

**First take in a language.** The language model is a system asset. The take reserves it for the app; when macOS already has it (keyboard dictation or another app fetched it), it's ready at once. Otherwise the strip shows `Getting English · 42%` with a filling ring, then starts listening (a green ring fills; a spinner until the first percent). If the download took over 20 s, or you cancelled meanwhile, it doesn't open the mic: a toast says the language is ready. The download continues after Esc. At most 5 reserved languages: the oldest unused one is released.

**Permissions.** Only the microphone. SpeechAnalyzer asks for no speech-recognition permission: the harness transcribed while `SFSpeechRecognizer.authorizationStatus()` stayed not determined.

**Errors** go to the strip in orange: no language Apple supports, download failed (offline), no usable audio format, microphone refused (the permission guide opens; a generic permission card will replace this hook, marked `MIC PERMISSION HOOK` in `VoiceFlow.swift`).

**Measured** (`tools/dictation-engine-test`, 8 clips, 32 s of `say` voices fed at real-time pace in the recorder's format; 5 runs on October 18 with the M4 Pro under load from other builds, load average up to 99, so ranges):

| Engine | Word errors | Stop → final text (median) | Caps+D → listening | First live words |
| --- | --- | --- | --- | --- |
| SpeechTranscriber + fastResults (**default**) | 1 / 108 | 47–158 ms | 8–69 ms (first take of a launch 115–745 ms) | ~1.1 s |
| SpeechTranscriber, no fastResults | 1 / 108 | 45–155 ms | similar | ~3.3 s |
| DictationTranscriber | 5 / 108 | 35–93 ms | similar | ~0.55 s |
| Parakeet-MLX final pass (CLI, beam 5) | 0 / 108 | 2.0–18 s (load-bound) | — | — |

Parakeet's wait is a fresh Python process loading the model and re-reading the whole take on every stop. Apple's recognizer has already heard it and only finalizes. Synthetic voices are clean, so the error counts flatter every engine; numbers are normalized (7:30 = seven thirty). DictationTranscriber stays the fallback module: its live words come sooner, but it made more errors ("coffee" for "copy", "on boarding").

## Where the text goes

The field you were in when the take started (A07). The strip and bar never activate the app, so that is normally the app in front.

| Start of take | Destination |
| --- | --- |
| Another app in front | Its focused window and field, captured at Caps+D |
| Our browser page has the keyboard | The page's field (WebKit insertion) |
| Our terminal has the keyboard | The shell, as typed text; newlines become spaces |
| One of our cards or windows in front, no field of ours focused | The last other app you used |

At the end: the app is activated (handing over activation if we're active), the captured window raised, the captured field focused, then the text goes in. If the app won't come forward within 0.6 s, only Accessibility insertion is allowed, so nothing is pasted or typed into the wrong app.

## How it's inserted

Per app family and field, in order; only a failed Accessibility write falls through.

| App / field | Methods |
| --- | --- |
| AppKit field (`AXTextField`, `AXTextArea`, `AXComboBox`, `AXSearchField`) with settable selected text and readable value + range | Accessibility, then paste |
| Chromium, Electron (detected from the bundle), JetBrains, Firefox, Slack, VS Code, Cursor, Zed and others listed in `DictationInsertPolicy.web` | Paste |
| Terminals (Terminal, iTerm2, Ghostty, kitty, Alacritty, Warp, WezTerm, Hyper, Tabby, Rio) | Paste (bracketed by the terminal) |
| Screen Sharing, Remote Desktop, VMs, VNC, TeamViewer, AnyDesk | Typed keys; multi-line text pastes instead |
| Secure field (`AXSecureTextField`) | Typed keys only; multi-line refused |
| Clipboard holds a password manager's secret (`ConcealedType`) | Typed keys before paste (single line) |

- **Accessibility:** sets `kAXSelectedText`, reads the value back and requires it to equal the old value with the selection replaced (normalized quotes/spacing accepted). Unchanged → paste. Changed some other way → the old value is written back and paste follows; if it can't be written back, nothing more is inserted. Undo works in `NSTextView`.
- **Paste:** the previous clipboard (every item, every type, bytes) is saved; the dictated text goes on marked `org.nspasteboard.TransientType` + `AutoGeneratedType`; ⌘V; 1.2 s later the previous contents come back unless something else was copied meanwhile. Both changes are noted in `PasteboardOwnWrites`, so the Clipboard module records neither. With **Keep clipboard after pasting** off, the text stays on the clipboard as a normal copy and is recorded.
- **Typed keys:** unicode key events, 20 UTF-16 units each, no modifiers. Never used for text with a newline (it would send a chat message or run a command).

Journaled as `voice.insert` (bundle, method, failure, length).

## Tone per app

Settings → Dictation → **Tone**. Chips for installed apps with a tone; tap cycles plain → chat → email → code, × goes back to the built-in, + adds an app. Stored in `voice.tone.apps` (`[bundle id: tone]`). Journaled as `voice.tone`.

| Tone | Built in for | Effect |
| --- | --- | --- |
| plain | everything else | — |
| chat | Messages, Slack, Discord, WhatsApp, Telegram, Signal, Messenger, Teams, Beeper | A one-sentence message loses its final period |
| email | Mail, Outlook, Spark, Mimestream, Superhuman, Airmail, MailMate | Model instruction: complete sentences, greeting and sign-off on their own lines |
| code | Terminals, Xcode, VS Code, Cursor, Zed, Sublime, Nova, BBEdit, JetBrains | No sentence capitals, no final period |

The editing pass now always gets a `Target app:` line. Chosen by measurement on s1-mini (`tools/dictation-test/tone-eval.py`, same prompt with and without each line): the email line formats greetings and sign-offs with no regressions; a code line gained nothing over the app line and lost one case; a casual line made the model keep fillers and drop capitals and apostrophes. Small sample (12 transcripts).

## Not done

- **Command mode** ("make this shorter" with text selected). Probed on s1-mini with a rewrite prompt: 1 of 5 instructions applied (bullets); shorter, translate and grammar came back unchanged. Needs a model that follows instructions; the selection is already read into the context.
- No live run in other apps yet: AX insertion is verified against a throwaway AppKit window; paste, typed keys, focus return across apps and Electron detection are tested against mocks only.

## Sources and checks

[AppleDictation](../../outputs/option-edge-search/Option/UI/AppleDictation.swift) (engine and language pick, assets, SpeechAnalyzer session), [DictationInsertPolicy](../../outputs/option-edge-search/Option/UI/DictationInsertPolicy.swift) (policy, focus machine, clipboard borrowing, tone, engine), [DictationInserter](../../outputs/option-edge-search/Option/UI/DictationInserter.swift) (Accessibility, events, pasteboard, last app), [VoiceFlow](../../outputs/option-edge-search/Option/UI/VoiceFlow.swift), [wiring](../../outputs/option-edge-search/Option/UI/EdgeBrowserWindow.swift), [settings row](../../outputs/option-edge-search/Option/UI/EdgeSettingsPages.swift), [clipboard history skip](../../outputs/option-edge-search/Option/WebKit/ClipboardStore.swift).

- `tools/dictation-test/run.sh`: headless, mocks + a private named pasteboard (114 checks).
- `tools/dictation-engine-test/run.sh`: engine and language pick, then `say` clips through the real SpeechAnalyzer session; WER and latency. Skips the audio part when the language isn't installed (`DOWNLOAD=1` fetches it); `VARIANTS=1` adds the other modules, `PARAKEET=1` the Parakeet CLI.
- `tools/dictation-test/live.sh`: real Accessibility into a throwaway window; keys and clipboard mocked. Needs Accessibility for the terminal.
- `tools/dictation-test/tone-eval.py [--command]`: against the running polish server on :8093.
# File actions

[Feature index](../../FEATURES.md) · [Held items](held.md) · [Search](search.md) · [Surface Lite](lite.md)

Added **October 17, 2026**. Typed actions on files: convert, resize, zip, PDF, rename. Apple frameworks only (ImageIO, AVFoundation, PDFKit, `ditto`); no ffmpeg, no network, no model. Part of the **Files** module (off = no file actions); in Lite.

They are [held](held.md) verbs. Hold a file result (⌘↑), drop files on the bar, or, with nothing held and Finder in front, type the action and it applies to Finder's selection.

## Using it

| ID | Entry point | Behavior and limits |
| --- | --- | --- |
| F01 | `to png`, `to jpg`, `to heic`, `to avif`, `to tiff` (also `as …`, `convert to …`, `convert …`) | Images. A new file next to each source: `Photo.png` (`Photo 2.png` when taken). Pixels and metadata kept, orientation kept as a tag. JPEG has no alpha, so transparency lands on white. Already that format: skipped. Multi-frame sources (GIF, HEICS): first frame only, the chip says so. `to webp` is refused: macOS has no WebP encoder. |
| F02 | `resize 50%`, `resize 1080` / `1080px` (longest side), `resize 1920x1080` (fit inside), `scale half` | Images. `Photo (50%).jpg`. Keeps the format when ImageIO can write it (else PNG). Orientation is baked in. Never upscales (skipped, says the current size). |
| F03 | `to mp4`, `to mov` | Video, through AVAssetExportSession. H.264/HEVC with AAC goes into the new container untouched (remux, no quality loss, seconds). Anything else (ProRes, other audio) is re-encoded with the highest-quality preset. AVFoundation can't read webm/mkv: refused. |
| F04 | `to m4a` | Audio from audio or video files: AAC. An AAC-only source is remuxed. No audio track: skipped. |
| F05 | `pdf`, `merge`, `merge pdf`, `combine`, `to pdf` | Held images and PDFs, in held order, into one PDF: `First (merged).pdf` (one image: `First.pdf`). PDF pages keep their text, links and annotations. Each image is one page at its own size (72 dpi unless the file says otherwise), upright. JPEGs are embedded byte for byte; other photos as JPEG q0.92; images with transparency stay lossless. A locked PDF stops it. |
| F06 | `zip` (also `archive`, `zip these`) | Everything held into one zip: one item → `name.ext.zip` (a folder keeps its name inside), several → `Archive.zip`, side by side at the root (same names get ` 2`). `ditto`, no `__MACOSX` folder or resource forks. Several items are staged as APFS clones on the source volume first. |
| F07 | `rename trip`, `rename trip-##`, `rename lowercase`, `rename uppercase`, `rename date`, `rename replace IMG_ with Trip ` | Renames each held file's current file in place. A name with several files numbers them in held order (`trip 1`, `trip 2`; padded at 10+; `##` sets where and how wide). `lowercase` includes the extension; `uppercase` leaves it. `date` prefixes the file's creation date (`2026-10-17 name.jpg`), skipping ones already prefixed. `replace` is case-insensitive, extension untouched. Extensions are kept; a taken name gets ` 2`; a change of case only is allowed. |
| F08 | Typing an action | The held card previews it per file before ↵: `800×1200 → Photo.png`, `1200×800 → 600×400`, the exact new names for rename, one row for zip/pdf (inputs, size or page count, the output name). Files it doesn't apply to stay dim with the reason (`already png`, `not a video`); `to webp` shows in orange. |
| F09 | ↵ | Runs it and clears the bar. Each held file presses in while it works (progress fills it), as with squeeze. Convert/resize/mp4/m4a add a **version** to each file (the chip's chain, drag-out and mail then use it). Zip and PDF turn their inputs into **one new held file** in their place. |
| F10 | Empty query, something held | The held card (instead of no card): running jobs with progress in the family color and percent, then finished ones as **result tiles**: thumbnail, name, size, how it was made; drag it out (copies, never moves), reveal in Finder, copy (the file, pasteable in Finder). Under them, **action tiles** for what's held (images: to png/jpg/heic, resize 50%, pdf; video: to mp4, to m4a; zip; rename). A tile runs its phrase; rename types `rename ` and waits. Up to four jobs show; finished ones clear when the next action starts, all of them on done. |
| F11 | Drop files on the bar or the card sheet | They're held, and the card above (F10) offers the actions. SwiftUI drop targets in the panel (the held slot, the bar, compose) take drops first; elsewhere on the panel the hosting view takes file drops, only over drawn parts (`BarDropTarget.swift`). |
| F12 | Nothing held, Finder in front: `to png`, `to mp4`, `resize 50%`, or `zip these`, `pdf these`, `rename these trip` | The card shows Finder's selection (Finder badge on each icon) with the preview. ↵ holds the selection and runs the action on it. Words that are also searches (`zip`, `pdf`, `merge`, `rename …`) need `these` / `this` / `selection` / `files` to mean Finder; conversions and resizes don't. Finder's selection is read once (Apple Events) when the typed text first spells an action, and again only after it stops spelling one. With things held, actions apply to what's held, not Finder. |
| F13 | ⌘Z / chip ↶ / `undo` | One step back, as for squeeze: a version goes to the Trash; a rename moves the name back (refused if something took the old name since; the chip says so); a zip or PDF goes to the Trash and the files it was made from come back held, versions and all. |

Half-typed or bare words are never taken as actions (`png`, `to pn`, `zip code`, `resize`): they search as usual. `compress`, `smaller` and `under 5mb` remain [squeeze](held.md) (size targeting, from the original).

## Additive, and how undo works

- **Nothing is overwritten.** Every output gets a free name (`name.ext`, then `name 2.ext`…); the final move fails rather than replace a file that appeared meanwhile. Sources are only read. Encodes run in a scratch folder on the source's volume; only finished files are moved into place.
- **Rename is the one exception**: it changes names, never bytes, and undo moves them back.
- **Where results go:** next to the source; if that folder isn't writable, `~/Library/Application Support/SurfaceEdge/Held/versions/` (as squeeze).
- **Undo never deletes:** versions and made files go to the Trash.

Colors (one per family, also the progress bar and result size): convert `#5ac8fa`, resize `#bf5af2`, pdf `#ff6b5a`, zip `#ffb340`, rename `#ffd60a`. Icons: Solar bold-duotone (`solar-refresh-square`, `-gallery`, `-clapperboard-play`, `-music-note-2`, `-maximize-square-minimalistic`, `-documents`, `-zip-file`, `-pen-new-square`, `-magnifer`, `-copy`).

## Journal

`files.convert`, `files.resize`, `files.merge_pdf`, `files.zip`, `files.rename`, `files.result_out` (drag/reveal/copy), registered in `docs/actions/registry.json`. Params: kinds, extensions, sizes, method, timing, result (`ok|already|unsupported|failed`), source (`key|tile|finder`); never paths or names. `held.add` gains source `finder`; `held.undo` gains `step` (`version|rename|made`).

## Debug hook

The [held debug hook](held.md#debug-hook) drives these too: `swift tools/held-test/post.swift path=~/Desktop/x.heic`, then `query="to png"` (the preview) or `query="to png" run=1`.

## Tests

- `tools/file-actions-test/run.sh`: the real `FileActions.swift` on generated files: parsing (and what must stay a search), free names, png/heic/avif/tiff (decoded: type, pixels, orientation tag, EXIF), webp refused, transparency on white for jpg and kept for heic, resize by percent/longest/box (orientation baked, never upscales), H.264 .mov → mp4 remux (brand, tracks, 640×360, 2 s, progress to 1), ProRes → mp4 re-encode, video and wav → m4a (AAC, duration), pdf merge (page count, order, text still text), images + pdf (upright page size), JPEG embedded as is (size ratio), zip one/folder/several (entries via `zipinfo`, same names apart, no `__MACOSX`, `Archive 2.zip`), every rename rule, renaming onto a taken name, case-only rename, rename back (and refusing when the old name is taken), the read-only folder fallback, suggestions, and every source's SHA-256 unchanged.
- `tools/file-actions-test/run-ui.sh`: the real EdgeHeld / verbs / EdgeHeldActions / cards / BarDropTarget with app stubs: verbs and module gating, strip chips unchanged, idle tiles, convert with a skipped file, undo, resize, renaming versions and originals and undoing both, zip and pdf collapsing and coming back on undo, a 6 s 720p ProRes → mp4 with progress, Finder selection (read once, not when Finder isn't in front, ↵ holds and runs), drop routing (file vs text drags, Files off, transparent vs drawn parts of the panel). Renders the cards offscreen (`SHOTS=dir`).
- `tools/held-test/run-ui.sh` still passes with file actions compiled in.

## Needs a live session

Not verified in the running app: real drags from Finder onto the bar and sheet; reading Finder's selection (the first time macOS asks for Automation permission for Finder; denied = no Finder card); the empty-query card appearing in the real stack; dragging a result tile into Finder/Mail.

**Known gap: a drop exactly on the typed text.** The text field's field editor takes it and inserts the path (as before). Replacing the field editor is not possible: SwiftUI's `TextField` force-casts the window's field editor to its private `_SystemTextFieldFieldEditor` and crashes on any other class (the UI test found this; never override `fieldEditor(_:for:)` on a window hosting SwiftUI text fields).

## Not yet

WebP and MP3 output (no Apple encoders), GIF from video, unzip, split PDF, rotate, rename by EXIF date (uses file creation date), keyboard selection of tiles, actions on folders' contents.

**Source.** [FileActions.swift](../../outputs/option-edge-search/Option/Files/FileActions.swift) (engine, no UI), [EdgeHeldActions.swift](../../outputs/option-edge-search/Option/UI/EdgeHeldActions.swift) (verbs, jobs, Finder selection, results out), [EdgeHeldActionCards.swift](../../outputs/option-edge-search/Option/UI/EdgeHeldActionCards.swift) (preview card, jobs/tiles card, result drag source), [BarDropTarget.swift](../../outputs/option-edge-search/Option/UI/BarDropTarget.swift). Model hooks in `EdgeHeld.swift` (renames, made items, undo), routing in `EdgeHeldCards.swift`, bar hooks in `Spotlight.swift` (empty-query card, Finder branch, ↵) and `SpotlightController.swift` (hosting view).
# Held items

[Feature index](../../FEATURES.md) · [Citizens and guests](citizenship.md) · [Mail](mail.md) · [Search](search.md)

Added **October 16, 2026**. First vertical slice: files, squeeze, mail.

**Hold the noun, type the verb.** Suites bundle apps; Nardo dissolves them. You don't open a compressor, then a mail client, then attach: you hold the thing, and the tools come to it.

## The idea

- **Nouns are held.** Pick a result in the bar (⌘↑, or drag it onto the slot that opens above the bar) and it becomes a chip in a strip directly above the bar, under the cards. It stays held across commands until you let go.
- **Verbs are typed.** With something held, the bar reads what you type as a command on it first: `under 100mb`, `mail susan`, `done`. Anything that isn't a verb searches as usual (so you can find and hold more).
- **Verbs are typed by noun.** Each verb says which kinds of held thing it accepts (`HeldNoun`: image, video, audio, pdf, document, folder today). The strip's verb chips are only the verbs that fit everything held.
- **Modules add verbs.** A verb is a small spec (`HeldVerbSpec`: id, icon, color, the kinds it accepts, a parser, whether its module is on) registered with `HeldVerbs.register`. Files contributes squeeze and the [file actions](file-actions.md) (convert, resize, pdf, zip, rename: offered as tiles on the held card, not as strip chips); Mail contributes compose. Switching Files off removes squeeze; Lite has no mail verb. Holding apps, clipboard items, tabs or threads later means a new `HeldNoun` case plus verbs that accept it; the strip, bar hooks and journal don't change.
- **Every step is a version, and versions undo.** Operations never touch the original. Each writes a new file; the chip shows the chain (`612 → 94 MB`) and the folder it went to, and ⌘Z (or the chip's ↶) steps back one version.

This is a citizen by both tests: it trades between files, mail and the bar in both directions, and every hold, verb and undo lands in the journal as a typed action the one brain can learn from (which verbs follow which nouns).

## Using it

| ID | Entry point | Behavior and limits |
| --- | --- | --- |
| H01 | ⌘↑ on a file result | Holds the selected file (file search, files and media cards, also while browsing a folder). The query is selected so the next word you type replaces it and the results stay for holding more. Holding the same file (or its version) twice does nothing. |
| H02 | Drag a file onto the slot or the bar | Dragging a file row out of a card opens the strip's slot above the bar as a dashed drop target (a hand icon); drop to hold. Files dragged in from Finder can be dropped on the strip or the bar (the bar outlines in mint). Elsewhere on the bar's panel (the card sheet) file drops are held too ([F11](file-actions.md)). A drop exactly on the bar's typed text still goes to the text field's own editor ([known gap](file-actions.md#needs-a-live-session)). |
| H03 | The strip | Directly above the bar, below the cards; when dictation is also up, dictation sits on top of it. It rises out of the bar and sinks back the way the dictation row does. One chip per held thing: icon, name, size chain, the folder the newest version is in, ↶ (step back), magnifier (show in Finder), ✕ (let go of this one, `held.remove`). Hover shows the full path, method and note. Drag a chip's icon or name out to use its current version anywhere: the drag offers **copy only**, so Finder copies even on the same volume and nothing held is ever moved. Then the verbs that fit (squeeze, mail) and ✓ (done). A long stack scrolls sideways. |
| H04 | `under 100mb`, `under 1.5 gb`, `< 500kb`, `for email` (25 MB), `smaller` / `compress` (half the original) | Squeeze card: each held file's size against the target (amber bar = work to do, green = already fits) and an icon for the method. ↵ runs it: the query clears, the chip presses in while it works (progress along its bottom edge) and springs back when the version lands. Already under target: no new version, the chip says so for a few seconds. Couldn't reach it: the chip says how close it got, in orange. |
| H05 | ⌘Z / chip ↶ / `undo` | One step back (the item changed last, or the chip's own). The version file goes to the Trash (never deleted outright); the original is never touched. ⌘Z only acts on held items while the query is empty or a held card is up; otherwise it is the field's own undo / mail undo. |
| H06 | `mail susan`, `mail susan work`, `email sam@x.com`, `send to sam` | Compose card: To resolved from your mail (everyone you've written to or who appears on your messages, display name they use most; prefix, last-name and one-typo matches; people you write to rank first; `work` / `personal` prefer company / webmail domains; a full address is used as is). Alternates sit next to the best match, one click to switch. Subject from the held file's name, body field (↵ in the bar jumps into it). Held files show as dashed one-click attach chips and can be dragged in (a held file's attachment follows its newest version). Send goes through Gmail SMTP. Over 25 MB: an amber `under 25mb` chip squeezes the held attachments to share what's left of the limit. Mail not connected: the send button is **Connect Gmail** (the existing setup sheet). After sending, the draft resets and the bar clears; things stay held. |
| H07 | `done` / `release` / `let go` / `clear` / ✓ in the strip / Esc on an empty query | Releases everything: chips slide away one after another, then the strip sinks. Versions stay on disk. A second Esc closes the bar as before. While things are held an empty query shows no idle cards; it shows the held card instead (file action jobs, results and tiles, [F10](file-actions.md)). |

## Squeeze (size targeting)

Targets are decimal (100 MB = 100,000,000 bytes, as Finder shows). Size verbs always encode **from the original**, not the current version, so repeated squeezes don't stack generation loss. A version is only kept if it measures under the target.

| Kind | Method |
| --- | --- |
| Images (jpg, png, heic, tiff, psd, bmp, webp, …) | ImageIO. Binary search on quality (0.45–0.92) at full size; if even 0.45 is too big, downscale by the measured ratio and search again. Writes JPEG, or HEIC when the image really uses transparency (checked on a thumbnail; opaque PNGs with an alpha channel still go JPEG). Metadata kept; orientation baked in when downscaled. |
| Video (mov, mp4, m4v, …) | AVFoundation re-encode (AVAssetReader → AVAssetWriter). Bitrate = target × 8 × 0.94 ÷ duration, minus audio (AAC 128/96/64 kbps by budget). Resolution steps down through standard heights when that's under ~0.06 bits per pixel per frame (0.04 for HEVC); never upscales. H.264 (HEVC if the source is HEVC), .mp4. Measured after encoding; retried up to 3× with a corrected bitrate if over. Files AVFoundation can't read (webm, mkv) use ffmpeg only if it's installed. |
| Audio (mp3, m4a, wav, aiff, …) | Same pipeline, AAC .m4a at the bitrate that fits. |
| PDF | First PDFKit's own image optimization (text stays text). If that isn't enough, pages are flattened to JPEG at the DPI that fits (the chip's note says text is no longer selectable). |
| Anything else, folders | Zip (`ditto`, no `__MACOSX` folder or resource forks, since zips usually go to other systems). Named with the whole original name: `server.log (under 1 MB).zip`. If the zip still doesn't fit, nothing is kept and the chip says how close it got. A format that can't shrink enough falls back to a zip, and the chip's note says so. |

**Where versions go.** Next to the original, as `name (under 100 MB).ext` (`name (under 100 MB) 2.ext` if taken), so the result is a normal file you can find, and the chip shows that folder. If the original's folder isn't writable, versions go to `~/Library/Application Support/Nardo/Held/versions/`. Encodes run in a temp folder and only a finished file is moved into place. Undo moves the version to the Trash. Alternative considered: a hidden versions store (no clutter, but files you can't find and drags of files that vanish); revisit if sibling files get annoying.

## Mail send

`Option/Mail/SMTPClient.swift`: Network.framework, `smtp.gmail.com:465` implicit TLS, EHLO, AUTH PLAIN with the same app password as IMAP (Keychain `com.hashawn.nardo.mail`), MIME multipart/mixed (text-only mail is a single text/plain part), base64 bodies and attachments wrapped at 76, RFC 2047 subject/names, RFC 2231 filenames (with an ASCII `filename=` for old clients when it fits a line), every line ≤ 78, Date/Message-ID/From/To/Subject/MIME-Version, dot-stuffed DATA sent in 256 KB chunks with progress. Plain TCP only for localhost (the fake server), mirroring IMAPClient; `SURFACE_SMTP_HOST=127.0.0.1:14465` points compose at one. Gmail files a copy in Sent by itself.

The password never reaches logs, errors or the journal: AUTH failures report a fixed message even though servers echo what they got (the test's fake server does, on purpose).

## Journal

`held.add`, `held.remove`, `held.done`, `held.squeeze`, `held.undo`, `held.drag_out`, `mail.compose`, `mail.attach`, `mail.send`, registered in `docs/actions/registry.json`. Params carry kinds, sizes, methods, results and timings, never message bodies or passwords; `mail.compose` and `mail.send` carry the recipient's domain and correspondence count, not the address.

## Debug hook

`com.hashawn.nardo.held` (distributed notification; full build, or a Debug build of Lite and its `--test-instance` copy; Release Lite ignores it, see `Option/App/DebugHooks.swift`). It opens the bar if it's closed. `userInfo`: `path` holds a file; `query` types into the bar; `run` = `1` performs the query as ↵ would; `done` = `1` releases.

```sh
swift tools/held-test/post.swift path=~/Movies/export.mov       # hold it, bar opens
swift tools/held-test/post.swift query="under 100mb"            # the squeeze card
swift tools/held-test/post.swift query="under 100mb" run=1      # squeeze it
swift tools/held-test/post.swift query="mail susan"             # compose card
swift tools/held-test/post.swift done=1                         # let go
```
`PREFIX=com.hashawn.nardo.test.` sends it to a `--test-instance` copy instead.

## Tests

- `tools/held-test/run.sh`: the real `FileSqueeze.swift` on generated files: a 36 MB noise PNG (to 5 MB, and to 600 KB which needs downscaling), a transparent PNG (→ HEIC), a 6 s 1080p video with audio (to 4 MB, and to 600 KB which steps resolution down), a 4-page PDF of photos (optimize, then flatten), a text log (zip), target parsing, "already under", "can't reach" leaving nothing behind, naming, and the read-only fallback. Each asserts the version is under target, decodes, is named and placed as documented, and the original's bytes are unchanged.
- `tools/held-test/run-ui.sh`: the real EdgeHeld, verbs, cards and compose (app types stubbed in `stubs.swift`) from hold to send: dedupe, verb parsing, module gating, squeeze to a version, already-under, undo (discards only the version), recipient resolution, not-connected → setup, the 25 MB squeeze from the compose card, a send to `fake_smtp.py` whose attachment is byte-identical to the version, the journal, done, and the drag slot. It renders the strip and cards offscreen to PNGs (`SHOTS=dir`). Not covered: the Spotlight.swift hooks themselves (⌘↑, ⌘Z, Esc, ↵, drops) and real drags.
- `tools/mail-test/run-send.sh`: the real SMTPClient + MIME builder against `fake_smtp.py`, checked through Python's own MIME parser (attachment bytes, RFC 2047/2231 round trips, headers, dot-stuffing, line lengths, no parser defects, the bad-password message) plus recipient resolution on a scratch mailbox.

## Not yet

Holding apps, clipboard items, tabs, mail threads; verbs beyond squeeze/mail/[file actions](file-actions.md) (share, move, to-canvas); OAuth instead of app passwords; replies (compose is new mail only, one recipient); a From display name; held items surviving a relaunch; Contacts (not read at all). The real Gmail send path is unverified until mail is connected.

**Source.** [EdgeHeld.swift](../../outputs/option-edge-search/Option/UI/EdgeHeld.swift) (model, strip, chips, copy-only drag source), [EdgeHeldVerbs.swift](../../outputs/option-edge-search/Option/UI/EdgeHeldVerbs.swift) (nouns, verb specs, registry), [EdgeHeldCards.swift](../../outputs/option-edge-search/Option/UI/EdgeHeldCards.swift) (squeeze/undo/done cards), [EdgeCompose.swift](../../outputs/option-edge-search/Option/UI/EdgeCompose.swift) (compose model + card), [FileSqueeze.swift](../../outputs/option-edge-search/Option/Files/FileSqueeze.swift), [SMTPClient.swift](../../outputs/option-edge-search/Option/Mail/SMTPClient.swift), [MailRecipients.swift](../../outputs/option-edge-search/Option/Mail/MailRecipients.swift). Hooks in `Spotlight.swift` (strip slot, `.held` card kind, ⌘↑, ⌘Z, Esc, ↵, bar drop, card-drag slot) and `SpotlightController.swift` (debug hook).
# Layer and workspaces

[Feature index](../../FEATURES.md) · [Shortcuts](shortcuts.md) · [Other apps' windows](windows.md)

Nardo is a desktop layer with two integrated workspaces, Surface and Browser, plus a transient command/results shelf. Other apps can be opened from that shelf. Workspace selection and whole-layer visibility are separate controls.

## Visibility and input

| ID | Feature / entry point | Behavior and limits |
| --- | --- | --- |
| L01 | Quick Caps Lock tap | A tap under 300 ms hides/restores the entire layer. An unfinished hold cancels on release. |
| L02 | Caps + 1 / 2 / 3 | Default actions: Surface, Browser, Clipboard. Number-down previews; number-up commits while Caps is still held. Multiple numbers can be used during one hold. |
| L03 | Cancel a chord | Release Caps before the number, or press Escape during the hold. Unassigned numbers do nothing. |
| L04 | Reveal pinned actions | Hold Caps for 350 ms while the command layer is visible, or right-click the leading command icon → Pinned actions. Hints appear above the input without a permanent workspace strip. |
| L05 | Reassign pinned slots | Right-click a hint and choose Surface, Browser, Clipboard, Notes or Games. Three assignments persist. Reassigning an already-pinned action swaps positions. Notes/Games seed scoped search. |
| L06 | Caps + 0 / input minus | Toggle the input alone while retaining workspaces. |
| L07 | Command-K | Expand and focus the command shelf from either integrated workspace. |
| L08 | Escape / field chevron | Collapse results/tools without closing the workspace; retain the query. |
| L09 | Enter after a request | Complete the selected result/action and slide input/results away. The next Caps tap can restore the input. Individual result semantics are in [Search](search.md). |
| L10 | Open or click a workspace | Focus its actual canvas/page and dismiss the command overlay. Workspace UI accepts the first click. Floating peers can remain open. |
| L11 | Select the open workspace again | Collapse it. Clicking a workspace updates the selected shortcut without closing other floating workspaces. |
| L12 | Command-W | Collapse the integrated workspace through Edge's window routing. Browser also has close-tab actions; see the context caveat in [Shortcuts](shortcuts.md). |
| L13 | Quit routing | Command-Q in Surface quits the app; in the search companion it hides the layer. |
| L14 | Reopen from macOS | Surface brings back the command interface; the companion can reopen Browser or search depending on visibility. |
| L15 | Summon URL | `nardo://search?query=…` seeds companion search. Duplicate companion launches forward to the existing instance. |

## Geometry and appearance

| ID | Feature / entry point | Behavior and limits |
| --- | --- | --- |
| L16 | Direct edge resize | Drag sides/top corners; a layout menu is not required. |
| L17 | Docked handle drag | The 28-point top handle changes height vertically. Horizontal movement is ignored until detached. Docked bottom corners stay square. |
| L18 | Floating handle drag | Moves the workspace freely. Snap near screen edges/corners; release at screen bottom to dock. |
| L19 | Detach/dock control | Explicit button or layout menu switches mode. Docking restores remembered docked geometry. |
| L20 | Downward flick | A short downward handle flick collapses a docked workspace. |
| L21 | Fill usable display | Double-click the handle, use its fill-screen button, or Control-Command-F. Restore the prior frame/mode afterward. This uses the current desktop rather than a separate full-screen Space. |
| L22 | Persisted layout | Each integrated workspace remembers size, position, floating/docked mode and fill-screen restore geometry across relaunches. Frames are constrained to an available display. Hidden slide positions are not saved. |
| L23 | Interruptible motion | Matching 220 ms positional open/close movement; rapid reversals cancel earlier motion. Full-size content slides rather than shrinking. Interrupted transitions restore intended geometry. |
| L24 | Reduce Motion | Workspace animation obeys macOS's Reduce Motion preference. |
| L25 | Native Liquid Glass | Workspace chrome, handle and command sheets use native material. Artwork/page colors remain their own. Active appearance persists while focus moves. |
| L26 | Content-sized result sheet | Results measure height; rear sheet rises from the display bottom behind the narrower command shelf. Shared spacing and concentric card corners apply across result types. |
| L27 | Contextual prompts | Input icon/prompt follows Surface or Browser. Animated eyes respond to query/attention; they are interface feedback, not an autonomous assistant. |
| L28 | Result-specific presentation | Compact calculator expression/answer, full-color rear sheet, square clipboard tray, larger file/web previews and keyboard-following result scroll. |

## Keyboard ownership and lifecycle

| ID | Feature / entry point | Behavior and limits |
| --- | --- | --- |
| L29 | Caps-to-F18 remapping | Companion owns the global shortcut using a temporary mapping, restores original mappings at shutdown and has a crash-recovery watchdog. |
| L30 | Competing launcher handling | Gracefully closes original Option Spotlight to avoid two shortcut owners, then relaunches it when Surface exits. If it refuses to quit, the new global shortcut is not registered. |
| L31 | Companion lifetime | Embedded companion exits when Surface exits; Surface and Browser communicate visibility/focus across process boundaries. |
| L32 | Visibility diagnostics | Bounded content-free layer events in the cache; separate from document recovery or browsing history. |
| L33 | Performance panel | Surface menu → Performance… shows CPU and memory for Surface, companion and attributed WebKit processes. Samples once per second only while open; 100% CPU means one core. |
| L34 | Bounded work during interaction | Workspace input keeps latest drag/resize per display interval and flushes on release. Equal frames and position-only moves avoid redundant layout; unchanged folder events/semantic catalogs are deduplicated. Canvas-specific caches are in [Surface](surface.md). |

Sources: [NardoController](../../outputs/surface-edge/Sources/SurfaceApp/EdgeController.swift), [EdgeCommands](../../outputs/surface-edge/Sources/SurfaceApp/EdgeCommands.swift), [WorkspacePlacement](../../outputs/surface-edge/Sources/SurfaceApp/WorkspacePlacement.swift), [PerformancePanel](../../outputs/surface-edge/Sources/SurfaceApp/PerformancePanel.swift), [companion SpotlightController](../../outputs/option-edge-search/Option/UI/SpotlightController.swift), [CapsWorkspaceChord](../../outputs/option-edge-search/Option/UI/CapsWorkspaceChord.swift), [pinned actions](../../outputs/option-edge-search/Option/UI/EdgeFolderShelf.swift), [Browser window](../../outputs/option-edge-search/Option/UI/EdgeBrowserWindow.swift), [eyes](../../outputs/option-edge-search/Option/Semantics/SurfaceEyesView.swift).
# Nardo Lite

[Feature index](../../FEATURES.md) · [Citizens and guests](citizenship.md) · [Layer and workspaces](layer.md)

Created **October 14, 2026**. Nardo Lite is the first public build: a smaller Nardo meant to ship early (r/macapps post), collect feedback, and grow. The name is a placeholder.

**Rule:** if it lives in the bar, it ships. If it replaces a whole app, it's promised for later.

## How it's built

Lite is **not a fork**. It is a second XcodeGen target, `Nardo Lite`, in `outputs/option-edge-search/project.yml`. It compiles the same sources as the `Nardo Search` companion, with one extra flag, `SURFACE_LITE`. The app layers are still compiled in but unreachable, so one tree builds both products.

| | Full (Nardo) | Lite |
| --- | --- | --- |
| Bundle | `/Applications/Nardo.app` (outer `Surface` + nested `Nardo Search` helper) | `/Applications/Nardo Lite.app`, standalone |
| Bundle ID | `com.hashawn.nardo` / `com.hashawn.nardo.search` | `com.hashawn.nardo.lite` |
| Info.plist | `Spotlight/Resources/Info.plist` | `Spotlight/Resources/Info-Lite.plist` |
| URL scheme | `nardo://` | `nardo-lite://` |
| Build | `outputs/surface-edge/build.sh` | `outputs/option-edge-search/build-lite.sh` |

`build-lite.sh` signs with a Developer ID identity when one exists (distributable, notarizable). Otherwise it falls back to Apple Development. That build has no provisioning profile and no restricted entitlements, so it should also run on other Macs after **Privacy & Security → Open Anyway**. Not yet confirmed on a second Mac. It replaces `/Applications/Nardo Lite.app`, keeping the previous copy under `work/nardo-lite-previous-*`. It never touches the full app.

## What's in and out

All gating goes through **`Option/App/Edition.swift`** (`Edition.lite`, `Edition.name`, `Edition.urlScheme`, `Edition.appCommandIDs`). Don't scatter `#if SURFACE_LITE`; branch on `Edition.lite`.

| In Lite | Out of Lite (promised later) |
| --- | --- |
| Caps Lock layer and Spotlight bar | Browser (Caps+B, `@browser`, our own tabs and history) |
| File search and previews, [file actions](file-actions.md) | Canvas / notes (Caps+N, `@notes`, Surface semantic results) |
| Clipboard cards, tear-off | Games, Hydra (Caps+G, `@games`) |
| Calculator, conversions, quick answers | Mail (Caps+E, mail cards, Gmail sync not started) |
| [Calendar](calendar.md) (meeting join, reminders) and media cards | `>` commands tied to the browser, canvas or games |
| [Bar words](bar-words.md): timer, mute, output switch, define, translate, color pick, emoji, kill port | |
| [System words](system-words.md): battery + Bluetooth devices (`connect sony`), `cpu`/`memory`/`quit slack`, `eject` | |
| Dictation (Caps+D), swipe keyboard (Caps+K) | VPN/Tor/proxy, desk cam (these live in the outer Surface app and aren't in Lite at all) |
| Terminal (Caps+T) | |
| Tiling and workspaces (Caps+W, Caps+1–9); [other apps' windows in workspaces](windows.md#workspaces-other-apps-windows-opt-in) (opt-in: Caps+Shift+digit sends the front window, switching parks the rest off a corner, everything comes back on quit/crash/launch; a switch re-tiles with that workspace's own splits); [snap keys and auto-tiling](windows.md) for any app (Caps+arrows, Return, Delete; Caps+Shift+W; drag a tile's edge or Caps+Option+arrows to resize splits), every move [animated](windows.md#animation) on the real window at the display's rate; the window switcher (Caps+Tab, Windows card) | |
| Capture (Caps+S; T inside it grabs text, also `ocr` in the bar), input-only (Caps+Space) | |
| [Bottom strip](strip.md) (Strip module, on, auto-hide): pinned + running apps, Dock badges, live sliver, workspace dots, window peeks, the Caps stack's floor; opt-in Replace Dock | |
| [Notch island](notch.md) (Notch module): now an opt-in mirror of the strip's live status, off by default | |
| Live web preview in the bar; Enter opens the **system default browser** | |
| Other browsers' open tabs (Safari/Chrome/Arc…) | |
| [Menu search and shortcut cheat sheet](menu-search.md) (Caps+/) | |
| [Settings](settings.md): modules, Caps key rebinding, open at login, Quit; permission asks in the bar | |

Where each gate lives:

- **Chords:** `SpotlightController.toolChordKeys` filters `allToolChordKeys`.
- **`@` tags:** `OptionAppRouter.command(in:)` returns nil.
- **Cards:** `Spotlight.liteCards`, called from `applyCardsNow`.
- **Commands:** `SpotlightCommands.matching`.
- **Web Enter:** `SpotlightController.expand` routes to `openInSystemDefaultBrowser`.
- **Mail sync:** skipped in `SpotlightApp.didFinishLaunching`.

## Modules (Settings card)

Full reference: [Settings, keys and first run](settings.md). Type `settings` (or `modules`, `preferences`; `settings mic` filters), press **Caps+,**, or open the app again while it runs. The card is a 4-across grid of every module (a short last row shares the full width) (`Option/App/EdgeModules.swift`, view `Option/UI/EdgeModulesCard.swift`). Tap or ↵ toggles; the tile wobbles and its color drains. A red dot means a permission is missing, and tapping it opens the permission guide.

Off means: no card in the bar, its Caps chord shows a toast instead (Caps+digit workspaces belong to Tiling), and background work stops where there is any (Clipboard stops recording, though its history stays; Files stops the FSEvents index; Shelf stops the edge shelf's monitors and hover poll and releases Finder's ⌘X/⌘V). State lives in `edge.modules.v1` (UserDefaults, `[module: Bool]`, missing = on). Turning a module back on restores its settings because they were never cleared.

Icons: **Solar bold-duotone** (480 Design, CC BY 4.0, credit required), template SVGs `solar-*` in the asset catalog. Before release, add the credit line to the About text.

**Pages:** typing a filter that narrows to one module (`settings dictation`, `settings mic`) or clicking a tile's › shows that module's settings in the same card; ‹ or ← goes back. Settings live in `Option/App/EdgeSettings.swift` (keys, defaults, typed accessors; pages write via `EdgeSettingsStore`), views in `Option/UI/EdgeSettingsPages.swift`:

| Page | Settings |
| --- | --- |
| General | Open at login (SMAppService), forget search learning, Quit, icon credit (Check now for updates once Sparkle is configured) |
| Keys | Every Caps chord: click, press a key to rebind; taken keys swap on a second press; digits/Esc/switcher keys refused ([Settings](settings.md#keys-rebind-caps-chords)) |
| Clipboard | Keep history (1d/1w/1m/3m/**forever**, default forever; pruned at launch, every 6h, on change; pinned never), never-record apps, clear unpinned |
| Files | Index size + reindex, folder access, leave-out folders |
| Answers | Currency rate freshness |
| Web | Search engine (DuckDuckGo/Google/Kagi/Brave/Bing/Startpage; URLs and !bangs still go through the core), live preview, other browsers' tabs (was its own module) |
| Calendar | Calendar for new events, default length |
| Dictation | Engine (offered only where Parakeet is installed; else "Apple, on this Mac"), language (SpeechTranscriber + DictationTranscriber locales), clean-up, keep clipboard, AI polish (only where s1-mini + llama-server exist), tone per app ([Dictation](dictation.md)) |
| Swipe keys | Forget learned words |
| Terminal | Font and size on top of the Ghostty config |
| Tiling | Workspaces include other apps (off; Bring back while any are parked), gaps, grid size (saved tiles clamp to a smaller grid), window thumbnails, windows in search, [auto-tile](windows.md) (Off/Main/All) and its layout (Split/Master), [animate windows](windows.md#animation) (on) and its feel (Quick/Snappy/Smooth), drag other apps to the grid, always-float apps |
| Capture | Save folder, PNG/JPEG/HEIC, show pointer. Text grab ([OCR](ocr.md)) has no settings; it is off with Capture. Reading clipboard images is off with Clipboard. |
| Strip | Show (at the edge / always), Mirror to notch (the island), Replace Dock / Restore Dock, Import from Dock ([Strip](strip.md)) |

Not yet: making "off" mean "not loaded" (today it means "not started / not shown"); keyboard navigation inside pages.

## Runtime behavior

- **Standalone.** Lite skips the full companion's "quit when the outer `com.hashawn.nardo` app is gone" watch.
- **One Caps Lock owner.** If the full companion (`com.hashawn.nardo.search`) is running, Lite starts without the Caps remap and shows a toast asking you to quit Nardo. The full app does not check for Lite yet, so don't run both.
- **Shared data on this Mac.** Lite uses the same App Support stores as the full app (`~/Library/Application Support/SurfaceEdge`, `option/option.sqlite`, the clipboard store, the `edge-ptyd` daemon), so terminal sessions and the clipboard carry over. On a fresh Mac it starts empty.
- **Debug notifications are off in Release.** Any process can post distributed notifications, and some hooks act with the app's permissions, so Release Lite answers only to `show`; a Release `--test-instance` copy answers look-only hooks (rank-eval, layer, hitdump). Debug builds answer everything. The shell-typing hooks aren't compiled into Release Lite, and the old `edge.debugHooks` preference no longer does anything ([table](settings.md#debug-hooks)). Run only one app when you script tests.
- **No developer services.** Release Lite never calls 127.0.0.1:8095 (EG2 ranking, tags, semantic file search) and never reads `~/fssearch`.
- **Dictation engine.** Apple's on-device SpeechAnalyzer, always, unless Settings → Dictation → Engine is set to Parakeet and its CLI is installed (Lite's Automatic never looks). The first take in a language reserves its model; if macOS doesn't have it yet, the strip shows the download (`Getting English · 42%`) and then listens. Polish only runs where s1-mini and llama-server exist. Details and measurements: [Dictation](dictation.md#engine).

## Rules for agents

1. **New user-visible feature?** Decide whether it belongs in Lite. If it doesn't, gate it with `Edition.lite`, add it to the table above, and make sure `build-lite.sh` still builds.
2. **Shared edits must compile both targets.** Check the full companion without touching the installed app:
   `xcodebuild -project outputs/option-edge-search/OptionEdgeSearch.xcodeproj -scheme 'Option Edge Search' -configuration Release -derivedDataPath work/edge-search-build-check build`
3. **Never rely on the outer Surface app being alive.** One example already bit us: ProMotion. The full build only reached 120Hz because the outer app's windows held the display high. Lite alone dropped to 60Hz and lagged until `MotionDriver` (`WorkspacePlacement.swift`) asked for `CAFrameRateRange(80, 120, 120)` itself. `EdgeTerminal` and `EdgeSwipeKeyboard` ask too (checked October 18: those are all the display links in the tree).
4. **Measure motion the same way for both apps.** Post a `com.hashawn.nardo.layer` distributed notification (Release Lite: to a `--test-instance` copy as `com.hashawn.nardo.test.layer`) to toggle the layer, then read the `frames=` lines from `log stream --level info` for subsystem `com.hashawn.nardo.perf`. Compare against the full app on the same machine state.

## Before a public release

- **Signing.** Needs a Developer ID identity and notarization (requires the paid Apple account).
- **First run.** Done October 18: each feature asks for its permission with a bar card when first used; nothing at launch, nothing lists Desktop/Documents/Downloads before you allow it; Input Monitoring isn't needed ([Settings](settings.md#first-run-asks-in-context)). Not yet: a first-launch hint that Caps Lock is the key.
- **Machine-specific paths.** Done October 14 and 18: dictation falls back to SpeechAnalyzer; Lite never reads `~/fssearch` (own index only, first-crawl count in the Files card and Settings → Files); the Ghostty config is only read when present (`tools/term-config-test` checks the defaults without it); the s1-mini polish model only when present; no 127.0.0.1:8095 calls in Release.
- **Updates.** Plumbing done October 18 (`EdgeUpdates`, empty `SUFeedURL`/`SUPublicEDKey`, off until configured). Still to do: add the Sparkle package, keys, an appcast. `build-lite.sh --no-install` builds a signed Release into `work/` without installing.
- **Startup hitches.** Partly investigated October 18 ([night2-bugs](../night2-bugs.md)). Fixed: the file index's rescan loop (it wrote 3.3 TB in 17 h and saturated the disk under every main-thread SQLite read), the clipboard card's unindexed sort (three per open, 5.4 → 0.09 ms each), and AppKit's password AutoFill setup on the first focus. Left: the very first `show()` still holds the main thread ~34 ms (first SwiftUI layout of the overlay) before the slide; in Lite that's the launch-time open. `shell show took N ms` in the motion log measures it.
- **Display links.** Terminal and swipe keyboard now ask for 80–120Hz themselves (October 14).
- **Hardened runtime.** `build-lite.sh` doesn't pass `--options runtime` yet; notarization requires it. Add and retest everything on the Developer ID day.
- **First file search on a fresh Mac** finds nothing until the first crawl completes (the engine opens the index once it's marked complete).
# Mail

[Feature index](../../FEATURES.md) · [Citizens and guests](citizenship.md) · [Shortcuts](shortcuts.md)

Mail is a layer, not an inbox. Gmail over IMAP (app password), synced into a local store. Added **October 13, 2026**.

| ID | Feature / entry point | Behavior and limits |
| --- | --- | --- |
| M01 | Needs-you card | With nothing typed, the bar shows threads where a person is waiting on you, and only when there are any. Rules today (`MailNeedsYou.score`): in the inbox, last message not from you, Primary, not bulk (List-Id/List-Unsubscribe/Precedence/Auto-Submitted/no-reply senders), from someone you've written to or addressed directly to you, and unread, asking something (`?`) or marked Important. Ranked by correspondence history, unread, Important, question, back-and-forth, halving every 3 days. |
| M02 | `mail`, `email`, `inbox`, `gmail` | Shows the needs-you card on purpose. `mail <words>` searches subject, sender and body (FTS5, prefix match). |
| M03 | Caps+E | Scopes the bar to mail (or brings the mail window forward). |
| M04 | Return / tear-off | In the bar, Return pulls the card off into its window, opened on the selected thread. The window is a thread list (needs-you first, then the rest of the inbox) plus a reader. Return in the window opens the thread in Gmail (replies live there until compose lands). |
| M05 | Reader | Older messages fold to one line each; the open message renders in a sealed WKWebView: no JavaScript, CSP `default-src 'none'`, a content rule blocking every http(s) load (tracking pixels never fire), non-persistent data store. Quoted replies hidden. Links open in the Surface browser. Attachments listed by name. Showing a thread for ~1.2s marks it read. |
| M06 | ⌘E / ⌘S / ⌘Z | Archive (removes Gmail's Inbox label), snooze until 8:00 tomorrow (local), undo the last of either. Work in the bar and in the mail window. Archive/snooze buttons also sit on the selected tile. |
| M07 | Setup | The card's setup tile opens a sheet for address + app password, verified against Gmail before saving. Password goes to the Keychain (`com.hashawn.nardo.mail`), never the database or journal. |
| M08 | Sync | One IMAP session on All Mail: the newest 45 days (max 2,500 messages), headers + Gmail thread/label metadata, CONDSTORE label changes, expunges, Primary via `X-GM-RAW`, bodies for inbox mail; then IDLE (push). Full pass every 10 minutes. Triage writes use their own short session. Reconnects with backoff. |

**Storage.** `~/Library/Application Support/Nardo/mail.db` (mode 600; override `SURFACE_MAIL_DB`).

**Journal.** `mail.connect`, `mail.open`, `mail.archive`, `mail.snooze`, `mail.undo`, `mail.read`, `mail.open_gmail`, each with thread id, sender, score and source, so the one brain can learn triage later.

**Source.** `Option/Mail/` (IMAPClient, MailMIME, MailStore, MailSync) and `Option/UI/EdgeMail.swift`. Tests: `tools/mail-test/run.sh` runs the real sources against a fake Gmail IMAP server (`fake_gmail.py`).

**Sending.** With a file held, `mail susan` opens a compose card and sends through Gmail SMTP with the same app password: see [Held items](held.md) (H06).

**Not yet.** Replies, compose without something held, multiple accounts, OAuth (needed to ship to other people), inline `cid:` images, attachment opening, snooze that survives other devices.
# Menu search and shortcut cheat sheet

[Feature index](../../FEATURES.md) · [Search and utilities](search.md) · [Surface Lite](lite.md) · [Citizens and guests](citizenship.md)

Added **October 17, 2026**. Both editions. Module: **Menus** (Settings card; needs Accessibility; chord Caps+/).

The bar remembers the app it was opened over and treats that app's menu bar as a list of typed commands: search them, see their shortcuts, perform them. The same executor takes (bundle id, menu path) from any caller, so menu items are a command surface for other apps that the brain can use later.

## Using it

| ID | Entry point | Behavior and limits |
| --- | --- | --- |
| MS01 | Type two or more letters | A menus card joins the results when the app in front has matching items: its icon as a tile, up to five rows (`Format › Font ›` dim, the item bright), check marks, shortcut key caps. Ranked on the bar's scale: exact item title 4 (behind an app you named and an open tab), title prefix / title words / acronym 9, words across the path (`font bold`) 16, substring or one typo 26, subsequence 38. Disabled items rank under enabled ones. |
| MS02 | ↵ or click on a row | The bar goes away, focus goes back to the app (activated if it isn't frontmost), the item is re-read live and pressed with `AXPress`. A row that is disabled shakes, the bar stays, a toast says it isn't available. Items that turn out disabled, gone, or refused at press time toast after the bar has left. |
| MS03 | `keys`, `shortcuts`, `hotkeys`, `cheatsheet`, `keyboard shortcuts` (prefixes of five letters count); `keys format` filters | Cheat sheet: every item with a shortcut, one tile per top-level menu in three balanced columns, the app's tile (icon, name, count) first; each column's last tile runs to the bottom. Submenu items show their sub-path. ↑↓ walk items column by column, ↵ performs. Taller than 420 pt: scrolls. Window-list and Open Recent items are left out. |
| MS04 | Caps+/ | Opens the bar on `keys` for the app in front. |
| MS05 | No app / no permission | `keys` with one of our windows in front: "No app in front". Without Accessibility: "Needs Accessibility" with the red dot that opens the permission guide. Menu search shows nothing in either case. |

## What gets read

- **Walk.** `AXMenuBar` of the app, one `AXUIElementCopyMultipleAttributeValues` call per element (role, title, enabled, command char, modifiers, virtual key, glyph, mark, children, identifier), on a serial background queue. Per-call timeout 0.5 s, whole walk 3 s, 4000 items, depth 6. Measured: Finder 203 items 40–200 ms, Notes 369 items ≈ 800 ms, the test app 49 items 90–120 ms.
- **Left out.** The Apple menu (sleep, restart, log out), the app menu's Services submenu (found by its items' `_processRequest:` / `_configureServicesMenu:` actions, so any language), separators, section headers (disabled, no action, no key), the Help menu's search field, submenu parents themselves.
- **Kept and marked.** Disabled items, check marks (`AXMenuItemMarkChar`), ⌥ alternates (same key or both keyless, ⌥ bit added: Close Window → Close All, Zoom → Zoom All), the dynamic Window list and Open Recent (`dynamic`: journaled without their titles).
- **Shortcut glyphs.** Order fn ⌃ ⌥ ⇧ ⌘ key. AX modifier mask: 1 ⇧, 2 ⌥, 4 ⌃, 8 no ⌘, 16 fn/🌐 (macOS 15 window tiling: Window › Fill = fn⌃F). Key from the menu glyph code if any (Carbon `kMenu*Glyph`: ⌫ ⌦ ⎋ ⇥ ↩ arrows F1–F19…), else the command character (letters upper-cased, control characters, AppKit `NSF1FunctionKey…` range), else the virtual key. An item with no key and only a mask (mods 8 is common) has no shortcut.

## Cache

Per bundle id (12 apps, least recently used out), stamped with pid, app version (`CFBundleShortVersionString+CFBundleVersion`) and walk time. On bar open: no snapshot or another version → walk; same version, other pid (relaunched) → show the cached items, walk again; dirty or older than 30 s → show, walk again; walked under 1.5 s ago → as is. Dirty comes from an `AXObserver` on the app for window created, focused / main window changed, title changed, element destroyed (the Window menu and most enabled states follow the window). Rows on screen are re-read live once per session (enabled, mark), and every press re-checks the item first: a stale cache can't press a disabled item. A rebuilt menu or relaunched app falls back to following the path by titles.

## Keystroke cost

Search runs on the main thread inside the bar's synchronous first pass, over keys precomputed during the walk (scalar arrays, a 64-bit letter mask that skips most items with one AND). Measured with `tools/menus-test` (Release `-O`, M4 Pro): 4000 items (the cap) median 123 µs, p95 224 µs, max 263 µs per keystroke; Finder's 203 items median 8 µs. `barOpened` costs 0.4 ms on the main thread (bundle version read). Walks never run on the main thread. In-app: `log stream --predicate 'subsystem == "studio.surface.edge.perf" AND category == "menus"' --info` prints `search … us=` per keystroke and `walk … ms=` per walk.

## Journal and actions

`menus.perform` (bundle, path, menu, depth, shortcut, checked, alternate, source `bar|keys|brain|debug`, result `ok|sent|disabled|missing|failed|no_app|not_trusted`, ms) and `menus.cheatsheet` (bundle, shortcut count, source `word|chord`), in `docs/actions/registry.json`. Window-list and Open Recent items are logged as `<window>` / `<recent>`. Spotlight's own choice/abandon events record the card as kind `menus`.

Programmatic entry points (main actor): `AppMenus.shared.perform(bundleID:path:source:)` (brings the app forward, follows the path, live check, AXPress; returns the result) and `AppMenus.shared.commands(bundleID:)` (fresh walk, every item with path, shortcut, enabled, checked). Brain calls journal with source `model`.

`sent` means the app took the press but didn't answer in time (typically a modal sheet opened); the command ran.

## Debug hook

`studio.surface.edge.menus` (full build, or a Debug build of Lite and its `--test-instance` copy; Release Lite ignores it, see `Option/App/DebugHooks.swift`). `query` opens the bar with that text; `dump=<bundle>` writes that app's items to `Caches/studio.surface.edge/menus-dump.json` (read only); `bundle` + `path` (`File › New` or `File/New`) performs an item. See `tools/menus-test/post.swift`.

## Tests

- `tools/menus-test/run.sh`: the real MenuModel / AppMenus / MenuReader with app types stubbed. Glyph rendering from masks, glyph codes, virtual keys and function-key characters; flattening a mocked TextEdit-like tree (Apple menu, Services, separators, section headers, alternates, duplicates, depth and size caps); ranking cases; cheat-sheet words, groups and columns; the cache policy; keystroke latency. Then a read-only walk of Finder, and two throwaway apps it builds and launches (`dev.menus-test.a/.b`): real walk (alternates, F5 without ⌘, ⌃⌫, ⌥←, ⌃⇥, checked and disabled items, section header, Window list, Help), search → press with focus handed back (the app's window was key when the action ran), disabled refused, brain path into a submenu while the other app is in front, missing path, no app, press by path without a cached element, AX observer marking the cache dirty on a new window and the revalidated walk seeing it, module off, our own app in front. 158 checks.
- `tools/menus-test/run-ui.sh`: renders the search card, the cheat sheet (full and scrolling) and the notices offscreen to PNGs (`SHOTS=dir`) from the throwaway app and, read only, Finder.

Not covered by tests: the Spotlight.swift and SpotlightController.swift hooks themselves (card in the bar, ↵, Caps+/, the bar resigning key before the press), the press wobble in the real bar.

## Not yet

- Apps that build menus lazily (`menuNeedsUpdate:`; Safari's History/Bookmarks, some Open Recent menus) show what AX exposes with the menu closed, which can be empty or stale.
- Non-Cocoa menus (Electron, Qt, Java) are read the same way but untested; their disabled items without an identifier are treated as section headers and left out.
- Pressing an item in an app that isn't in front first activates it; apps that ignore activation get the press anyway.
- A torn-off answer window shows the menus card only while the bar remembers an app.

**Source.** [MenuModel.swift](../../outputs/option-edge-search/Option/Menus/MenuModel.swift) (tree, flattening, glyphs, search, cheat sheet, cache policy), [AppMenus.swift](../../outputs/option-edge-search/Option/Menus/AppMenus.swift) (AX reading, cache, observer, executor, journal, debug hook), [EdgeMenusCard.swift](../../outputs/option-edge-search/Option/UI/EdgeMenusCard.swift) (cards, key caps, press feedback). Hooks in `Spotlight.swift` (`.menus` card kind, `cheapCards`, ↵, ↑↓, re-search on `AppMenus.revision`), `SpotlightController.swift` (target capture in `show()`, Caps+/, debug hook), `EdgeModules.swift` (Menus module).
# Notch island

[Feature index](../../FEATURES.md) · [Citizens and guests](citizenship.md) · [Surface Lite](lite.md) · [Layer and workspaces](layer.md)

Added **October 16, 2026**. Both editions.

**Since October 18 (branch `strip/dock`) the [bottom strip](strip.md) is the primary home for live status on every Mac; the island is an opt-in mirror** (Settings → Strip → Mirror to notch, **off by default**; a Mac that had explicitly switched Notch on keeps it). Both read the same state through `EdgeLiveStatus.swift` (the reader that used to live in the island's controller).

A black body around the MacBook notch that shows what Surface is doing right now. It is a view onto existing state, not a widget shelf: it reads the dictation model, the terminal's waiting notice, now playing and the Caps chord in progress, and its buttons call the same actions the cards and chords do. It keeps no store of its own.

## What it shows

One item owns the collapsed island (an ear each side of the notch: icon left, live readout right); each other live item adds a 5pt dot in its color to the right ear (up to three). Expanded, every shown item is a card (four at most), in this order:

| Order | Item | Source (read, not copied) | Collapsed readout | Card and action |
| --- | --- | --- | --- | --- |
| 1 | Caps chord being held | `EdgeFolderShelfState.pending` (set by `SpotlightController.selectCapsWorkspace`) | `⇪T` keycap; left ear is that module's icon and color | Tile, keycap, chord title. No action (release Caps to run it). Only while a letter/digit is down, so it pulses briefly. |
| 2 | Dictation | `BrowserWindow.shared.dictation` (`VoiceFlowModel`: phase, level, transcript) | Live 4-bar waveform from `level` (the same `VoiceWaveformView`); spinner while transcribing; check when inserted; warning on failure | Tile, 9-bar waveform, `TranscriptTicker`, stop button → `BrowserWindow.toggleDictation` (Caps+D's action), journaled `voice.dictate {source: island}` |
| 3 | Terminal notice waiting for you | `EdgeTermPeek.shared.waiting`: the last peek notice, kept until that session is in front of you; finished commands stop waiting after 5 min | ✓ ✗ • in your palette's green/red/yellow | The peek's line in your terminal font and colors after a tinted tile; click → `EdgeTermPeek.open` (same route as clicking the peek), the island folds away |
| 4 | Timer (hook only) | `EdgeIsland.shared.addSource { [IslandTimer] }`; nothing on main registers one yet | Remaining time | Tile, title, progress bar, remaining |
| 5 | Now playing | `EdgeMedia.usesOwnPlayer` ? `EdgeMediaPlayer` : `EdgeNowPlaying` (the media card's own choice) | Progress ring in the media color; left ear is the artwork (else the app's icon) | Artwork, title, artist, progress, ⏮ ⏯ ⏭ → `EdgeMedia.previous/toggle/next(source: "island")` |

Live means: dictation in any visible phase, a waiting notice, a running timer, a Caps chord down, a track **playing**. A paused track is not live, so the island hides; but a track paused from the open island keeps its card until the island folds (it was pinned when it opened). Switched-off modules (Dictation, Terminal, Media) don't show.

## Using it

| ID | Entry point | Behavior and limits |
| --- | --- | --- |
| NI01 | Something becomes live | The ears grow out of the notch (spring). Appearing under a resting pointer doesn't open it; move off and back. |
| NI02 | Hover the island | Opens after 0.14s over it (a fly-by doesn't). The body drops out of the notch with a little overshoot; the cards fade in as it reveals them; the ears ride out to stand over the cards' tile column and trailing edge. |
| NI03 | Click the band (the notch-tall strip) | Collapsed: opens at once. Open: folds, and staying put doesn't hover it back open until the pointer leaves. |
| NI04 | Pointer leaves | Folds 0.35s later; coming back within that cancels. Never mid-press (a drag that left keeps it open until release). |
| NI05 | Nothing live any more | Folds into the notch and hides; the window is ordered out. |
| NI06 | Display without a notch | A top-center pill hanging from the menu bar (menu bar tall, 24pt if it auto-hides), only when the Notch module is on there. |
| NI07 | Notch module (Settings card, `settings notch`; or Settings → Strip → Mirror to notch) | Default **off** (was: on with a notch display, until October 18). Explicitly switching it sticks. Off: no window, no pointer monitors. No settings page of its own. |

## How it's built

| Part | File |
| --- | --- |
| Geometry, layout, what shows, state machine, click-through rect (pure, no AppKit windows) | [EdgeIsland.swift](../../outputs/option-edge-search/Option/UI/EdgeIsland.swift) |
| Drawing: silhouette, ears, cards | [EdgeIslandView.swift](../../outputs/option-edge-search/Option/UI/EdgeIslandView.swift) |
| Window, observation, pointer, actions | [EdgeIslandController.swift](../../outputs/option-edge-search/Option/UI/EdgeIslandController.swift) |
| Reading Surface's live state (shared with the strip) | [EdgeLiveStatus.swift](../../outputs/option-edge-search/Option/UI/EdgeLiveStatus.swift) |

- **Notch geometry.** Height `safeAreaInsets.top`; width = screen width − `auxiliaryTopLeftArea.width` − `auxiliaryTopRightArea.width`, starting at `frame.minX + left width` (widths only: the auxiliary rects' origins aren't documented as global or local). Lives on the first display with a notch, else the main display.
- **Shape.** Goo bodies (the island box, the notch box, and a strip just above the screen top) closed with an 8pt fillet by `GooShape`, so the body melts into the notch and both shoulders curve into the menu bar. The path is computed once per frame and used as fill, mask and hairline; level ticks (24Hz while dictating) don't rebuild it.
- **Motion.** Two `GooSpring`s on the display link: `presence` (ears out of the notch) and `open` (collapsed → expanded body, unclamped so it overshoots). No SwiftUI animation on geometry.
- **Window.** Borderless non-activating panel, never key, level main menu + 3 (over the menu bar, under its menus), all Spaces and full-screen auxiliary, top flush with the screen top. `ignoresMouseEvents` stays true except while the pointer is over the island's current rect (global + local mouse-moved monitors, installed only while something shows), so the menu bar and apps under the empty parts of the panel keep their clicks.
- **Observation.** `withObservationTracking` around one read of all sources → `IslandSnapshot` → items. Now playing starts streaming (`EdgeNowPlaying.start`, the adapter in perl) when the island starts with the Media module on, not only when the media card first shows.

## Journal

`island.expand {by: hover|click, showing, count}`, `media.toggle|next|previous {player, source: card|bar|island}`, `media.play {path}`, `edge.open_notice {kind, source: peek|island}`, `voice.dictate {source: island}` ([registry](../actions/registry.json)).

## Checks

- `tools/island-test/run.sh`: geometry from mocked displays (notched at origin, notched at an offset, notchless, auto-hidden menu bar, several at once), layout, click-through rects, priority and pinning, the hover/click machine.
- `tools/island-test/run-ui.sh`: renders the real view (goo engine, waveform, ticker) offscreen over a mock menu bar and notch: collapsed per kind, appearing, half-open, overshoot, expanded, notchless. `SHOTS=dir` keeps the PNGs.

## Not verified live

Built and checked headless only. Needs a real session: alignment against the hardware notch, hover and clicks over the menu bar (including with the menu bar's own menus open), full-screen apps, the panel on a second display, the Caps chord pulse timing, and whether the ears cover status items sitting right beside the notch on a crowded menu bar.
# Text from images (OCR)

[Feature index](../../FEATURES.md) · [Search and utilities](search.md) · [Surface Lite](lite.md) · [Citizens and guests](citizenship.md)

Added **October 16, 2026**. Text grab from the screen, and clipboard search that matches text inside images. Both use on-device Vision (`VNRecognizeTextRequest`, accurate, automatic language detection). Nothing leaves the Mac. In both Surface Edge and Lite.

Citizenship: things-out (screen text lands on the clipboard as a card) and the bar (image clips become findable by their contents). Both actions are in the journal.

## Text grab

| ID | Entry point | Behavior and limits |
| --- | --- | --- |
| O01 | Caps+S, then **T** | Toggles the capture overlay between region capture and text grab. A chip under the loupe shows the mode (camera / text icon, `T` keycap); text grab lights it and the selection outline in Capture's yellow. T again switches back. The overlay always opens in region mode from Caps+S. |
| O02 | `ocr`, `grab text`, `text from screen`, `screen text`, `read text` in the bar (prefixes of 4+ characters count) | Answer card (yellow text tile, `⇪S then T`). ↵ opens the overlay already in text grab. Also `>` → **Grab Text from Screen**. Shown only while the Capture module is on. |
| O03 | Drag a region (text grab) | Frozen frame, selection tracks the pointer 1:1 (the same overlay as capture). On release the region's text is recognized, put on the pasteboard and recorded as a clipboard text card credited to the app whose window was under the region (else the app you were in). The keyboard goes back to that app, so ⌘V works right away. Clipboard module off: the text is still copied, no card is recorded; ignored apps (password managers etc.) are not recorded either. |
| O04 | Confirmation card | Rises from the bottom edge like the capture card: yellow tile + spinner while reading, then the first 4 lines (`+N` for more) and a wobble when the text lands. Sized to the text (120–520 pt of text width). Leaves after 2.4 s (1.6 s for "No text"), held while hovered; click dismisses, drag pulls the text out. |
| O05 | Click without dragging (text grab) | Does nothing; drag again. (In region mode a click still picks a color.) |
| O06 | Esc | Cancels; in text grab the keyboard goes straight back to the app you were in. |

Layout of recognized text (shared with the region capture card's text):

- Fragments that share a row (overlap at least half their height) join into one line, left to right, or right to left when the row is mostly Arabic/Hebrew.
- A horizontal gap wider than ~3 average characters becomes a tab (tables paste as columns).
- A vertical gap taller than 1.2× the median line height leaves a blank line.

## Text inside clipboard images

| ID | Entry point | Behavior and limits |
| --- | --- | --- |
| O07 | Background reader | Image clips are read one at a time on a utility-priority queue with a 0.5 s pause between, newest first. Starts 20 s after launch, and on new image clips. Runs only while the Clipboard module is on; switching it off stops after the current image. Not run by the `--test-instance` copy. |
| O08 | Limits | Skipped (marked, not retried): over 40 MB, over 60 megapixels, or a side under 16 px. Images are decoded at a long side of at most 4096 px for recognition. |
| O09 | Cache | `image_text(blob_hash, text, read, version, read_at)` in the clipboard database, keyed by the image's content hash, so a re-copied image is read once. `EdgeOCR.version` bumps re-read everything read by an older pipeline. Text is dropped when no clip refers to its image anymore (delete, clear, never-record-from-app, retention prune). |
| O10 | Bar clipboard search | A clip matches when its own text matches (FTS, whole words, as before) **or** it is an image whose text contains every typed term: substring, case- and diacritic-insensitive, any script. Terms of 3+ characters use a trigram FTS index (`image_text_fts`); shorter terms (a 2-character Chinese word) use `LIKE` (ASCII case-folding only). Applies everywhere the clipboard is searched: universal results, the Caps+C card/window. |
| O11 | Matched snippet | An image clip found by its text shows the matched line over the bottom of its thumbnail (2 lines, black plate), the matched run in Clipboard's color. |
| O12 | Copy Text (right-click an image clip) | Copies the image's text as a new text clip. Reads it on the spot if the background reader hasn't reached it yet. |

## Image files

| ID | Entry point | Behavior and limits |
| --- | --- | --- |
| O13 | Text button in an image file's preview (file search, next to zoom) | Reads the image (decoded at a long side of 4096 px) and copies its text as a clipboard text card credited to Finder. The button shows a spinner, then a yellow ✓, or dims when there's no text. |

## Where it lives

- [EdgeOCR.swift](../../outputs/option-edge-search/Option/UI/EdgeOCR.swift): recognition, line layout, snippets, `ClipboardImageText` (the background reader).
- [Capture.swift](../../outputs/option-edge-search/Option/UI/Capture.swift): overlay mode, `grabText`, `TextGrabCard`.
- [ClipboardStore.swift](../../outputs/option-edge-search/Option/WebKit/ClipboardStore.swift): `image_text` tables, search, `copyAndRecord`.
- [RootView.swift](../../outputs/option-edge-search/Option/UI/RootView.swift) (`ClipCard`): snippet, Copy Text.
- [InteractiveFilePreview.swift](../../outputs/option-edge-search/Option/UI/InteractiveFilePreview.swift) (`imageAccessory` slot) + `ImageTextCopyButton` in Spotlight.swift: copy-text button. The slot keeps the preview file self-contained for `Tests/SpotlightStability`.
- [Spotlight.swift](../../outputs/option-edge-search/Option/UI/Spotlight.swift) (`grabTextCard`), [SpotlightCommands.swift](../../outputs/option-edge-search/Option/UI/SpotlightCommands.swift) (`cmd.grabText`).
- Journal: `capture.text` (w, h, lines, chars, via), `clipboard.copy_image_text`, `files.copy_image_text`.

## Verification

`tools/ocr-test/run.sh` builds the real `EdgeOCR.swift` and `ClipboardStore.swift` with stubs and runs offline against a throwaway store and a private pasteboard: rendered English (3 lines kept exactly, blank line between paragraphs), Chinese, Arabic (right to left), Japanese and Russian with automatic language detection; layout rules on synthetic fragments; snippets; background reading, caching, skip of oversized images, version bump, stop/restart with the module, orphan cleanup; search by image text (word, multi-word, substring, 2- and 3-character Chinese), no false matches, text clips unchanged; grab → pasteboard + clipboard card credited to the source app; module off → copied, not recorded. `SHOTS=dir` keeps the rendered images.

`tools/ocr-test/run-ui.sh` builds the real `Capture.swift` into a throwaway .app (for the asset catalog) and, without showing any window: renders the overlay in both modes from a synthetic frozen frame (selection follows the drag 1:1, yellow outline in text grab, white in region mode, outside dimmed), releases the drag and reads the boxed lines back exactly (and nothing outside the box), and renders the grab card while reading, with two lines, with 7 lines (4 + `+3`, width capped) and with no text.

Not covered headless: the live overlay (Screen Recording permission, T toggle, Esc), the keyboard hand-back to the previous app, which app a grab is credited to, the confirmation card, the clipboard-card snippet and the image-preview text button on screen, and reading time on a large real clipboard history.

The `solar-text-square` icon was drawn by hand in Solar bold-duotone's shape (rounded square at 50% + a T); swap in the official Solar `text-square` SVG if it differs.
# Search and utilities

[Feature index](../../FEATURES.md) · [Commands](commands.md) · [Shortcuts](shortcuts.md)

Summon with Caps Lock or Command-K. Tab/Shift-Tab cycles result categories; arrows navigate entries within them. Enter activates the selected result. The field retains its query when tools collapse. Recognition/ranking may offer more than one interpretation; the selected card determines the action.

## Discovery and result actions

| ID | Feature / entry point | Behavior and limits |
| --- | --- | --- |
| Q01 | Universal search | Combine answers, apps, files, clipboard, games/Hydra, tabs/history, notes and web previews in ranked categories. |
| Q02 | Empty search | Recent clipboard feed; scoped feature searches can show their own recents/library. |
| Q03 | Application search | Search installed app names, display icons, Enter launches through the launcher. App list refreshes when stale. |
| Q04 | File search | Search local index by keyword/filename with project/role-aware ranking; enrich with system Spotlight/filename fallbacks. Own FSEvents index (`FileIndexer.swift`, `files.db`: names and paths only); `~/fssearch/index.db` only as a fallback before the first crawl completes. Rescans (Settings → Reindex, a folder flagged MustScanSubDirs, dropped events) diff against existing rows, so an unchanged tree writes almost nothing and row ids survive. Dropped-event bursts collapse into one rescan 10 s later, at most every 5 min; `~/Library` is excluded from the stream ([night 2 fix](../night2-bugs.md)). Dense-vector search is not integrated. |
| Q05 | File result actions | Enter opens with macOS; result files can be dragged to other destinations, including supported Surface drops. ⌘↑ holds a result; then `to png`, `resize 50%`, `to mp4`, `zip`, `pdf`, `rename …` act on it ([file actions](file-actions.md)). |
| Q06 | Image/SVG preview | Fit, zoom buttons, pinch and drag-to-pan for decodable PNG/JPEG/GIF/WebP/TIFF/HEIC/BMP/SVG. A text button copies the text in the image ([OCR](ocr.md)). |
| Q07 | 3D preview | SceneKit attempts OBJ/DAE/SCN/USDZ/USD/USDC/PLY/STL. Orbit, zoom, Shift-drag pan; unsupported/undecodable files use Quick Look. |
| Q08 | Folder/other-file preview | Folder lists up to 80 visible children; other files use native Quick Look. Metadata includes kind, size, modification date and path. |
| Q09 | Open-tab search | Find real browser tabs; activation selects the actual tab and shows integrated Browser. Page text indexing also supports matching. |
| Q10 | Browsing history search | Find visited pages and reopen the selected URL. Combined tab/history results show both kinds. |
| Q11 | Live web preview | URL/search previews in the rear sheet; selected web result or Command-Enter promotes to Browser. Preview respects new-tab/address/site-scoped purpose. |
| Q12 | Browser-scoped field | `@b`, `@browser`, `@web` and fuzzy aliases; query routes to browsing/search. |
| Q13 | Notes-scoped field | `@n`, `@notes`, `@notebook`, `@crumbs`; search boards/text, recents and document-creation cards. Opens external Option Notes. |
| Q14 | Games-scoped field | `@g`, `@games`, `@play`, `@launcher`; local games, online source search and update queries. |
| Q15 | Fuzzy app aliases | Shared router accepts abbreviated/consonant aliases such as `@brwsr` and `@nts`. Feature tag can be removed to return to universal search. |
| Q16 | Natural-language search scope (removed) | Learned action/scope models are no longer connected or bundled. Ordinary queries remain literal. |
| Q17 | Edit/remove explicit feature tag | Explicit `@browser`, `@notes`, `@games` tags retain their remove/backspace controls. |
| Q18 | Surface discovery | Current-workspace content catalog plus local English sentence embeddings ranks title/excerpt matches; result opens identified Surface content. Catalog is bounded to 300 records. |
| Q19 | Natural-language Surface creation (removed) | Use Surface menus or shortcuts; Spotlight no longer infers notebook/note/canvas creation. |
| Q20 | Explicit Notes creation/capture | `new canvas …`, `new notebook …`, `note …` / `new note …` cards hand off to Option Notes, including initial text. |
| Q21 | Capture into selected note | Option-Enter on a Notes result sends the typed query into that board without opening it. External Notes dependency applies. |
| Q22 | Promote feature | Option-F on supported Games, Notes or web cards opens the fuller app/workspace. |
| Q23 | Learned local ranking | Query-specific usage memory breaks low-confidence ties; it cannot outrank explicit scope, URL or calculation intent. |
| Q24 | Stale-result handling | Debounced/asynchronous searches track generations; outdated results/preview targets are discarded. Keyboard selection scrolls into view. |

Sources: [Spotlight](../../outputs/option-edge-search/Option/UI/Spotlight.swift), [launcher](../../outputs/option-edge-search/Option/Bridge/SpotlightLauncher.swift), [FileSearchEngine](../../outputs/option-edge-search/Option/UI/FileSearchEngine.swift), [InteractiveFilePreview](../../outputs/option-edge-search/Option/UI/InteractiveFilePreview.swift), [semantic scopes](../../outputs/option-edge-search/Option/Semantics/SemanticScope.swift), [semantic search](../../outputs/option-edge-search/Option/Semantics/SemanticIntent.swift), [Surface bridge](../../outputs/option-edge-search/Option/Semantics/SurfaceSemanticBridge.swift).

## Clipboard

| ID | Feature / entry point | Behavior and limits |
| --- | --- | --- |
| Q25 | Clipboard recording | Poll pasteboard changes every 0.6 seconds; record images first, otherwise nonempty text up to 200,000 characters, with source application and capture time. |
| Q26 | Persistent history | SQLite full-text index and deduplicated image blobs; pinning protects items from default 60-day retention pruning. |
| Q27 | Search/filter | Search stored text and supported natural date/time phrases such as today, yesterday, last week, relative days or explicit date ranges. Image clips also match by the text inside them (read in the background, substring, any script); the matched line shows on the card. See [OCR](ocr.md). |
| Q28 | Preview and metadata | Image previews, readable text snippets and footer with type, pin, source/time metadata. Display truncation does not alter original payload. |
| Q29 | Copy original | Enter or context menu Copy restores stored payload to pasteboard; selecting a result does not itself promise paste into another app. |
| Q30 | Plain-text copy | Context menu Copy as Plain Text for text entries; Copy Text on image entries copies the text read from the image ([OCR](ocr.md)). |
| Q31 | Pin/unpin and delete | Context menu pin toggles retention protection; Delete removes chosen entry. |
| Q32 | Drag clipboard content | Drag provider exports supported text/image content. |
| Q75 | Large-text compression | Text/URL clips at least 16,384 UTF-8 bytes use lossless `ncz` compression only if smaller. Full value is restored when copied; only the first 4096 characters remain in searchable/display preview for compressed clips. |

Clipboard shelf CRUD, editing and retention setters exist in the store but lack controls in this Edge feed. They are listed as source-only in [Availability](availability.md).

Sources: [ClipboardStore](../../outputs/option-edge-search/Option/WebKit/ClipboardStore.swift), [clipboard card](../../outputs/option-edge-search/Option/UI/RootView.swift), [search/activation](../../outputs/option-edge-search/Option/UI/Spotlight.swift).

## Calculator and conversions

Enter copies the selected answer. Calculator expression and answer use separate compact panels. Rates are estimates from cache/network, not trade execution prices.

| ID | Feature | Examples / limits |
| --- | --- | --- |
| Q33 | Arithmetic | `2 + 3 * 4`, parentheses, unary signs, exponentiation, implicit multiplication and common word operators. Half-typed unit words do not summon a calculator. |
| Q34 | Percentages | Percent expressions and percent-of/add/subtract behavior. |
| Q35 | Scientific functions | `sqrt`, `cbrt`, `abs`, `sign`, floor/ceil/round, exp, ln/log/log2/log10, trig/inverse trig/hyperbolic trig, rad/deg conversions, factorial; constants include pi/e. |
| Q36 | Number base output | Expressions with hex/binary/octal output suffixes; base inputs also have a quick-answer card below. |
| Q37 | Duration conversion | Milliseconds through years; months/years in unit conversion use average fixed durations. Calendar date offsets are separate. |
| Q38 | Length conversion | Nanometers to kilometers, inches/feet/yards/miles/nautical miles, AU/light-years. `m` is minute in this registry; spell `meter` for meters. |
| Q39 | Mass conversion | Milligrams/grams/kilograms/tonnes, ounces/pounds/stone. |
| Q40 | Volume and area | Liter/milliliter/cubic meter, US fluid/cooking units; square metric/imperial units, acres/hectares. |
| Q41 | Speed/data conversion | m/s, km/h, mph, ft/s, knots, Mach; bits/bytes, decimal KB–PB and binary KiB–TiB. |
| Q42 | Energy/power/pressure/frequency | Joules/calories/Wh/eV/BTU and registered power, pressure and frequency units. |
| Q43 | Angle/temperature | Radians/degrees/gradians/turns; Celsius/Fahrenheit/Kelvin with offset-aware conversion. |
| Q44 | Currency | `100 USD to EGP`, aliases/symbols and unit expressions. Cached USD-relative rates refresh after 12 hours; offline fallback rates exist. Detail reports rate freshness/fallback. |
| Q45 | Relative calendar dates | `5 days from now`, `3 weeks ago`, `today + 5 days`, `tomorrow - 2 weeks`. |
| Q46 | Date distances | Detected dates with minutes/hours/days/weeks/months/years, including phrases about until/since/between. Uses local calendar/date detection. |

`CalculatorScope` implements named variables and `ans`, but the active answer builder calls the stateless evaluator. Those registers are **source only**, not promised command-field functionality.

Sources: [CalculatorEngine](../../outputs/option-edge-search/Option/UI/CalculatorEngine.swift), [unused scope adapter](../../outputs/option-edge-search/Option/UI/CalculatorScope.swift).

## Offline quick answers and actions

| ID | Feature | Input and result |
| --- | --- | --- |
| Q47 | Color conversion | `#f60`, `#ff6600`, `rgb(255,102,0)`; suffix `to/in/as hex`, `rgb` or `hsl` selects copied format. Actual color fills sheet with adaptive text. HSL is an output format, not an input parser here. |
| Q48 | Integer bases | `0xff`, `0b1010`, `0o17` or multi-digit decimal: decimal answer plus HEX/BIN/OCT metadata. This card copies hex, unlike calculator-base output. |
| Q49 | Base64 | `base64 hello` encodes UTF-8; `base64d aGVsbG8=` decodes valid UTF-8 payloads. |
| Q50 | URL encoding | `urlencode hello world`, `urldecode hello%20world`. |
| Q51 | Hashes | `sha256 hello`, `sha1 hello`, `md5 hello`; digest of literal UTF-8 text. |
| Q52 | UUID | `uuid` or `guid` produces a random lowercased UUID. |
| Q53 | Password generation | `password` / `pw` defaults to 20 characters; optional length clamped to 6–128. Copy the generated string. |
| Q54 | Unix time | `now` copies epoch seconds; 10-digit seconds/13-digit milliseconds display local formatted dates. |
| Q55 | Text counts | `count …` reports characters, words and lines; copies character count. |
| Q56 | Text casing/reversal | `upper`, `lower`, `title`, `reverse` followed by text. |
| Q57 | Playful text transforms | `mock` / `spongebob`, `clap`, `flip` / `upside`, `vapor` / `fullwidth` / `aesthetic` followed by text. |
| Q58 | ISO day countdown | `days until`, `days til`, `days to`, `days since` followed by `yyyy-MM-dd`; copies day number. |
| Q59 | Coin/dice | `coin`, `flip a coin`, `roll` (d6), `roll 2d6`; count is clamped to 1–50. |
| Q60 | Random integer/choice | `random 10`, `random 5 20`, `rand …`, `pick red, green, blue`. |
| Q61 | Magic 8-Ball | `8ball …` / `8 ball …` gives a random canned response. |
| Q62 | Daily log capture | `log …` appends to today's Markdown daily spine. Separate from the excluded Journal UI. |
| Q63 | Keep Mac Awake | `awake`, `stay awake`, `caffeine`, `caffeinate`, `insomnia`, `no sleep`, etc. toggles an idle-system-sleep assertion. Display/lid sleep is not disabled. State persists and re-arms when singleton initializes. |
| Q76 | Time zones | `time in tokyo`, `tokyo time`, `what time is it in new york`, `now in dubai`; conversions `3pm london in my time`, `3pm london` (to here), `3pm in tokyo` (from here), `9:30am pst to cet`, `noon utc in my time`, `3pm utc+5:30 in my time`, with `today`/`tomorrow`/`yesterday`, `2026-03-08` or `mar 8` anywhere. Places: every tz-database city and id, countries, US/EU/Asia abbreviations (meaning the region's clock, so EST in July is New York time), `utc±h[:mm]`; `my time`/`local`/`here`. The wall clock is resolved on the source zone's date: a skipped time (spring forward) lands after the gap and says so; a repeated one takes the first pass and says so. Copies the answer; drops the calculator card. Source: `EdgeTimeZones.swift`. |
| Q64 | Math display formatting | Basic LaTeX-to-display formatting for supported result text; not a general LaTeX document compiler. |

Source: [SpotlightAnswers and display formatting](../../outputs/option-edge-search/Option/UI/Spotlight.swift), [StayAwake](../../outputs/option-edge-search/Option/App/StayAwake.swift), [DailyLog](../../outputs/option-edge-search/Option/Essentials/Notes/DailyLog.swift).

## `>` command palette

Type `>` and optionally filter by title/keywords. Enter runs the chosen registry item.

| ID | Command | Effect / availability |
| --- | --- | --- |
| Q65 | Open Settings | Invokes Settings scene; **current companion scene shows only the small Spotlight panel**, not the full Browser preferences. |
| Q66 | New Workspace | Creates/activates a Browser space and shows Browser. |
| Q67 | Clear Browsing History | Clears history; leaves cookies/tabs. |
| Q68 | Clear Web Caches | Clears cached resources; sign-ins remain. |
| Q69 | Enable / Disable Ad Blocking | Global content-blocker toggle and protection reload. |
| Q70 | Open Downloads Folder | Opens current download directory in Finder. |
| Q71 | Copy Page Screenshot | Copy current page's snapshot to clipboard. |
| Q72 | Print Page | Print active page using macOS print flow. |
| Q73 | Open Notes / Open Games | Notes hands off externally; Games opens its companion-owned library window. |
| Q74 | Separate / Share Logins | Active-space isolation toggle switches between independent and shared website-data profile. |

Source: [SpotlightCommands registry](../../outputs/option-edge-search/Option/UI/SpotlightCommands.swift).
# Settings, keys and first run

[Feature index](../../FEATURES.md) · [Surface Lite](lite.md) · [Lite launch sort](../scout/lite-launch.md)

Updated **October 18, 2026** (night 2, ship-readiness). Both editions.

Settings live in the bar, not a window. Type `settings` (`modules`, `preferences`, `settings mic` filters), press **Caps+,**, open the app again while it runs (Finder, Launchpad, Spotlight), or open `surface-lite://settings`. If AppKit ever opens the SwiftUI Settings scene (⌘,), it closes itself and opens the same card.

## Modules grid

Every module is a tile, four across. Tap or ↵ switches it; › opens its page. Page-only tiles (General, Keys) don't switch. A red dot means a permission is missing; tapping it starts the guide.

State: `edge.modules.v1` = `[module: Bool]`, missing = on (except the notch island: off by default since October 18, now Settings → Strip → Mirror to notch). Settings survive switching off.

| Module | Off stops |
|---|---|
| Clipboard | Recording (pasteboard poll) and image-text reading; history stays |
| Files | File card, held-file verbs, bar drops, **the FSEvents index** (it saves its journal position and replays on return) |
| Shelf | The screen-edge drop shelf's global drag monitors and 30Hz hover poll; Finder's ⌘X/⌘V claim |
| Answers, Web, Media, Calendar, Capture | Their cards and words; Calendar also cancels timers |
| Dictation, Swipe keys, Terminal, Capture, Menus | Their Caps chord (a toast says it's off) |
| Tiling | Caps+W, Caps+digits, the switcher and Windows card |
| Strip | The bottom strip: its window, pointer monitors and badge polling; restores Apple's Dock if it was replaced ([Strip](strip.md)) |
| Notch | The island (also Strip page → Mirror to notch) |

Not yet: "off" means not started / not shown, never "not loaded". Tiling's auto-tiler of the layer's own windows still runs (another builder owns that area).

## Keys (rebind Caps chords)

`settings keys` (or `chords`, `caps`). Every chord as a tile: the cap shows the key it's on now, in its module's color. Click a tile, press a key:

- **Free key:** it moves; the tile wobbles.
- **Taken key:** the owner's cap turns red and a line says so. Press the same key again to swap the two.
- **Reserved:** digits (workspaces), Esc (cancel), Return/Delete/arrows (the window switcher), Caps itself. The tile shakes and says why.
- **↺** puts a moved chord back. **Reset all** appears when anything moved.

Model: `Option/App/EdgeChords.swift`. Each chord's id is its default letter in `SpotlightController.allToolChordKeys`, so a chord added there shows up here with no other registration. Overrides live in `edge.chords.v1` = `[id: keyCode]` (only moved chords). Resolve order: your bindings first (list order), then defaults on free keys. A default that lands on a key you gave to something else comes back unbound (`–`, red) instead of doubled. Carbon registration picks up changes on the next Caps press. Module gating looks chords up by id, so a rebound chord still belongs to its module.

## General

Open at login (`SMAppService.mainApp`), forget search learning, Quit, the icon credit. With Sparkle configured, **Check now** too.

## First run: asks in context

Nothing is asked at launch. A feature that needs a permission asks the moment you use it, with one card in the bar (`Option/UI/EdgePermissionAsk.swift`): its module's icon and color, **Allow Accessibility** (or the permission), and why. ↵ starts `PermissionGuide`: the right System Settings pane plus the docked helper that watches for the switch. Esc or typing moves on, and the card lapses after 45s.

| Feature | Asks for | Where |
|---|---|---|
| Swipe keys (Caps+K) | Accessibility | Instead of showing a keyboard that can't type |
| Caps+W on another app, window switcher focus | Accessibility | TileEngine `ForeignWindows.ensureTrusted` |
| Dictation typing into another app | Accessibility | Same, as Dictation |
| Menu actions | Accessibility | AppMenus |
| Capture (Caps+S) | Screen Recording | CaptureController |
| File search | Full Disk Access | A card under the file results while Desktop/Documents/Downloads are blocked |
| Dictation | Microphone | The system's own prompt first; the guide after a denial (VoiceFlow, unchanged) |
| Calendar | Calendars / Reminders | EventKit's own prompt first |

**Input Monitoring is not needed:** Caps is remapped with `hidutil` and chords are Carbon hot keys; nothing uses an event tap.

**Protected folders.** Listing Desktop, Documents or Downloads is what makes macOS ask "would like to access files in your Desktop folder". `FolderAccess` (in `PermissionGuide.swift`) keeps everything out of them until it may: with Full Disk Access (detected by opening the TCC database, which never prompts) or after you press Allow on a Files ask (`edge.files.protectedFolders`). Until then the index counts them blocked without listing them, the filename fallback skips them, and the permission dot doesn't look. Allow re-checks and crawls what became readable right away. An update from a build before this opt-in counts as allowed (`FolderAccess.carryOverEarlierConsent`, once, at launch): those builds listed the three folders at every launch, so macOS already has the answer, and without it the index froze them for someone who had allowed them long ago. A first launch records not yet.

**First crawl.** While it runs, a file search with no hits shows "Indexing your files · N so far", and Settings → Files shows the count. Lite uses only its own index (the `~/fssearch` fallback is full-edition only).

The guide's "Relaunch" restarts this app (it used to quit and open Surface Edge, even from Lite).

## Debug hooks

Scripted checks drive the app with distributed notifications, which any process can post. `Option/App/DebugHooks.swift` decides who answers:

| Build | Real app | `--test-instance` copy |
|---|---|---|
| Debug (either edition) | all | all |
| Release full edition | all | all |
| Release Lite | `show` only | look-only: `show`, `focus`, `working`, `shutdown`, `layer`, `hitdump`, `bartype`, `rankeval` |

The shell-typing hooks (`termkeys`, `termrun`, `termdebug`, `termdemo`, `termblocks`) are compiled out of Release Lite (`#if DEBUG || !SURFACE_LITE`). The `edge.debugHooks` preference is gone, because any unsandboxed process can write a preference. `tools/rank-eval` still works: it runs on a Release test copy. Its `--live` mode, and held/menus hooks against Lite, need a Debug build. Lite also ignores the outer app's navigator and tabs notifications. Window-geometry relays (TileEngine/AutoTiler/layout v1) still listen in Lite; they only move Surface's own windows.

Release Lite also never calls the developer's local model server (127.0.0.1:8095: EG2 ranking, query tags, semantic file search; `Edition.localModelServer`), which would send every keystroke to whatever listens on that port.

## Updates

`Option/App/EdgeUpdates.swift` reads Sparkle's own keys (`SUFeedURL`, `SUPublicEDKey`, both empty in `Info-Lite.plist`; `SUEnableAutomaticChecks` false). Without the Sparkle package it compiles to a no-op. To turn it on: add Sparkle (SPM) to the Surface Lite target, fill both keys (an https appcast, the EdDSA key from `generate_keys`), and ship notarized builds.

`build-lite.sh --no-install` builds and signs into `work/surface-lite-build` and stops: nothing is quit and `/Applications` is untouched.

## Checks

- `tools/settings-test/run.sh`: chords (72: labels, resolve, conflicts, swap, reset, a 3000-case collision fuzz, new chords, persistence), modules (83: persistence, onChange, page routing, chord ownership, source checks that "off" and first-run gating reach their code), updates (9).
- `tools/debug-hooks-test/run.sh`: the gating table (42) and a source check that shell hooks sit inside the `#if`.
- `tools/term-config-test/run.sh`: the terminal with no Ghostty config (built-in defaults), empty and junk files, and a real config with a missing include (11).

## Source

`Option/App/EdgeModules.swift`, `Option/App/EdgeChords.swift`, `Option/App/DebugHooks.swift`, `Option/App/EdgeUpdates.swift`, `Option/UI/EdgeModulesCard.swift`, `Option/UI/EdgeSettingsPages.swift`, `Option/UI/EdgeKeysPage.swift`, `Option/UI/EdgePermissionAsk.swift`, `Option/UI/PermissionGuide.swift` (`FolderAccess`), `Spotlight/App/SpotlightApp.swift` (reopen, module lifecycle).
# Shortcuts and gestures

[Feature index](../../FEATURES.md) · [Availability caveats](availability.md)

Defaults from current menu/keyboard registries, audited September 30, 2026. `⌘` Command, `⌥` Option, `⇧` Shift, `⌃` Control. Context matters: Surface and Browser have different owners for the same chord. Browser overrides persist, but its full remapping settings pane is not presented by Edge's current Settings scene.

## Layer and command shelf

| Input | Action |
| --- | --- |
| Quick Caps Lock tap (<300 ms) | Hide/restore whole layer. |
| Hold Caps (350 ms) | Show pinned hints when input is visible. |
| Caps + 1 / 2 / 3 | Surface / Browser / Clipboard by default. Release number while Caps remains held to commit; number-down previews. |
| Release Caps before number / Escape during hold | Cancel unfinished action. |
| Caps + 0 / input minus | Toggle only input. |
| Caps + Tab | Window switcher: Tab / Shift+Tab / arrows step, a hint letter jumps (raises on its key-up), release Caps to raise, Escape cancels. See [Window switcher](window-switcher.md). |
| Caps + ← / → / ↑ / ↓ | The front app's window to that half; again ⅔, then ⅓. Two arrows at once: a quarter (← + → middle third, ↑ + ↓ full height). See [Windows](windows.md). |
| Caps + Return / Shift+Return | Maximize (again: back to your frame) / center. |
| Caps + Shift + arrow | Window to the display that way. |
| Caps + Delete | Window back to your frame from before the first snap. Esc during the hold undoes that hold's moves. |
| Caps + W / Caps + Shift + W | Grid picker for the front window / auto-tiling on or off for its display (on for the main display until you choose; off flies the tiles back to where they were). Auto-tiled display: Caps+arrow focuses a neighbour, Shift swaps, Return floats, Shift+Return always floats the app. Drag a tile's edge to resize the split (neighbours follow live). |
| Caps + Option + arrow / Option + Delete | Auto-tiled display: the focused tile's border that way moves 5% (again: further) / the display's splits even again. |
| Caps + S, then T | Capture overlay: T toggles region capture ⇄ text grab (drag → text on the clipboard). Esc cancels. See [OCR](ocr.md). |
| Caps + / | Shortcut cheat sheet of the app in front (`keys`); [menu search](menu-search.md). |
| Right-click input icon / pinned hint | Reveal pinned actions / reassign slot. |
| ⌘K | Expand/focus shelf from either workspace. |
| Escape / field chevron | Collapse results, retain query. |
| Enter | Run selected result / copy answer, then dismiss input. |
| Tab / ⇧Tab | Cycle result categories forward/back. |
| Arrows | Select rows/items within result; selected result scrolls into view. |
| ⌘Enter | Promote web search/preview to Browser. |
| ⌥Enter | Capture query into selected existing Notes board without opening it. |
| ⌥F | Expand current supported Browser/Notes/Games result into its full workspace/app. |
| Backspace on empty scope / ⌥Delete | Remove explicit feature tag as supported by its handler. |
| ⌘W | Collapse current integrated workspace; keep tools. |
| ⌃⌘F / double-click workspace handle | Fill usable display / restore previous geometry. |
| ⌘Q | Surface quits; companion hides layer. |

## Surface

| Input | Action |
| --- | --- |
| ⌘N / ⌘O | New canvas / Open `.surface`. |
| ⌘S / ⇧⌘S | Save / Save As. |
| ⌘E | Export PNG from design/image. |
| ⌘Z / ⇧⌘Z | Undo / redo. |
| ⌘X / ⌘C / ⌘V / ⌘A | Cut / copy / paste / select all in active editor context. |
| ⌘D | Duplicate objects or placed live views. |
| ⌥⌘R | Rename current/selected surface. |
| ⌘0 | Fit canvas. |
| ⌃⌘N / toolbar stack | Toggle Canvas navigation. Navigator is also an assignable Caps action. |
| ← / → while navigating | Select sibling elements in their parent board. |
| ↓ / ↑ while navigating | Expand selected surface / collapse to its parent element. |
| Return / Escape while navigating | Open selected surface in Surface / close navigation. |
| ⇧⌘H | History tree. |
| ⌥⌘I | Properties. |
| ⇧⌘R / hold Control | Toggle / temporarily reveal relationships. Control also bypasses snapping during manipulation. |
| Arrows / ⇧Arrows | Nudge selection 1 / 10 pixels. Add Option for independent movement. |
| Delete / forward Delete | Remove selection. |
| Escape on canvas | Cancel draft ink, clear selection, Select tool. |
| Space-drag / middle-button drag / scroll | Pan. |
| Pinch / ⌥scroll / ⌘scroll | Zoom around pointer. |
| Shift-click / marquee | Additive selection; Multi-select toggle also exists. |
| ⌘click overlapping artwork | Cycle hit candidates. |
| ⌥drag | Move independently of movement relationship. |
| ⌥hover covered board | Peek live contents. |
| Double-click empty canvas / printable typing | Create text; tool letters are **not** Surface keyboard tool selectors. |
| Double-click text / note / image / nested surface | Edit text / inline note / annotate image / enter content. |
| ⌘Enter / Escape in inline editor | Finish edit. |

## Browser

| Input | Action |
| --- | --- |
| ⌘T | New-tab search shelf. |
| ⌥T | New tab scoped to focused site/group. |
| ⌥W | Close active tab. Registry also assigns ⌘W, but Edge intercepts it to hide Browser first. |
| ⌥Tab / ⇧⌥Tab | Cycle site megagroups forward/back. |
| Hold Option, tap/release backtick (`) | Sub-tab switcher advances on backtick release; release Option to commit. Shift reverses. |
| Hold ⌥backtick, press T | New tab in focused site without incidental switch. |
| Enter / Escape in switcher | Commit / cancel. Typing pivots to tab search. |
| ⌘1–⌘9 | Select Browser space by ordered position. |
| ⌘L | Edit current URL in bottom shelf; navigation replaces current tab. |
| ⌘R / ⌘[ / ⌘] | Reload / back / forward. |
| ⌘+ / ⌘− / ⌘0 | Zoom in / out / reset. |
| ⌘S | Toggle sidebar. |
| ⌘F / ⌘Y / ⌘/ | Find in page / history / live shortcut cheat sheet. |
| ⌥Space | Start/stop dictation in Browser keyboard context; fixed binding. |
| ⌥hover tab | Glance preview; Enter commits / Escape dismisses. |
| ⌘click tab card | Split/focus two-pane browsing. |
| Page top-edge hover | Reveal navigation/tools affordance. |

Print and screenshot are available from tools or `>` commands. Their advertised ⌘P / ⇧⌘S menu shortcuts are not wired by the current Edge entry. The global voice controller is also not started by Edge; Option-Space here is Browser-local.

## Option Notes editor source

These belong to the editor source in this checkout. Edge normally launches a separate Option Notes app, whose installed version may differ. Letter shortcuts are skipped while editing text.

| Input | Action |
| --- | --- |
| V / T / R / O / L / A / P | Select / Text / Rectangle / Ellipse / Line / Arrow / Pen. |
| ⌘J / ⌘↑ | Commit edit and request Notes search. |
| ⌘Z / ⇧⌘Z | Undo / redo. |
| ⌘A / ⌘C / ⌘V / ⌘D | Select all / copy / paste / duplicate. |
| ⌘G / ⇧⌘G | Group / ungroup. |
| ⇧⌘F / ⇧⌘B | Front / back; add Option for one step forward/back. |
| Arrows / ⇧Arrows | Nudge 1 / 10. |
| Delete / Escape | Delete selection / Select and deselect. Escape while editing commits/exits field. |
| ⌘click wiki-link | Follow/create linked board. |
| Shift-resize text | Independent text deformation instead of proportional resizing. |

Games uses ⌘F for search and state-dependent Escape to back out/clear/hide. In the voice panel, Enter stops/transcribes and Escape cancels.

## Query reminders

| Query | Result |
| --- | --- |
| `@browser`, `@notes`, `@games` | Explicit app scope. Files/clipboard/app discovery uses ordinary queries. |
| `>`, then feature name | Maintenance/feature command palette. |
| `!g query`, `!yt query`, `!gh query` | Explicit engine/site search bang; see [Search](search.md) for full inventory. |
| `draw two red rectangles`, `make them blue` | Ordinary search text; AI editing interpretation has been removed. |
| `2 + 3 * 4`, `20 c to f`, `1 meter to feet` | Calculator / unit conversion. |
| `#ff6600 to hsl`, `base64 hello`, `sha256 hello` | Color / encoding / hash answer. |
| `password 24`, `roll 2d6`, `days until 2026-12-31` | Generator / dice / date utility. |
| `log your text`, `awake` | Daily log / toggle idle-sleep prevention. |

Sources: [Surface menus](../../outputs/surface-edge/Sources/SurfaceApp/main.swift), [canvas keyboard](../../outputs/surface-edge/Sources/SurfaceApp/CanvasView.swift), [Browser defaults](../../outputs/option-edge-search/Option/UI/ShortcutMap.swift), [Browser routing](../../outputs/option-edge-search/Option/UI/Keyboard.swift), [shelf/results](../../outputs/option-edge-search/Option/UI/Spotlight.swift), [Caps chords](../../outputs/option-edge-search/Option/UI/CapsWorkspaceChord.swift), [Notes keyboard](../../outputs/option-edge-search/Option/Essentials/Notes/NotesRootView.swift), [Games view](../../outputs/option-edge-search/Option/Essentials/Games/GamesLibraryView.swift), [voice panel](../../outputs/option-edge-search/Option/UI/VoiceFlow.swift).
# Source coverage map

[Feature index](../../FEATURES.md)

This map accounts for **159 first-party Swift files** under Surface `Sources/` and companion `Shared/`, `Spotlight/`, `Option/`, including the explicitly excluded standalone entry. It supports finding implementations and re-auditing coverage; a listed file is not a claim that every path in it is reachable.

Generated products, upstream vendored code and old copies under `work/` are outside this application-source inventory. The Browser reducer is supplied as `Core/liboption_ffi.a`; its Swift command/snapshot interface is documented, its missing Rust source is not inferred.

## Surface layer and application

Menus, workspace geometry, companion launch, input and performance panel. [Feature reference](layer.md).

- [main.swift](../../outputs/surface-edge/Sources/SurfaceApp/main.swift)
- [EdgeController.swift](../../outputs/surface-edge/Sources/SurfaceApp/EdgeController.swift)
- [EdgeCommands.swift](../../outputs/surface-edge/Sources/SurfaceApp/EdgeCommands.swift)
- [WorkspacePlacement.swift](../../outputs/surface-edge/Sources/SurfaceApp/WorkspacePlacement.swift)
- [OptionSuiteLauncher.swift](../../outputs/surface-edge/Sources/SurfaceApp/OptionSuiteLauncher.swift)
- [PerformancePanel.swift](../../outputs/surface-edge/Sources/SurfaceApp/PerformancePanel.swift)

## Surface editor and document model

Rendering, canvas gestures, selection controls, notebook/image/board views, relationships, persistent history and recovery. [Feature reference](surface.md).

- [Document.swift](../../outputs/surface-edge/Sources/SurfaceCore/Document.swift)
- [History.swift](../../outputs/surface-edge/Sources/SurfaceCore/History.swift)
- [PropertyRelationships.swift](../../outputs/surface-edge/Sources/SurfaceCore/PropertyRelationships.swift)
- [CanvasGrid.swift](../../outputs/surface-edge/Sources/SurfaceApp/CanvasGrid.swift)
- [CanvasInteractions.swift](../../outputs/surface-edge/Sources/SurfaceApp/CanvasInteractions.swift)
- [CanvasView.swift](../../outputs/surface-edge/Sources/SurfaceApp/CanvasView.swift)
- [DirectControls.swift](../../outputs/surface-edge/Sources/SurfaceApp/DirectControls.swift)
- [Engines.swift](../../outputs/surface-edge/Sources/SurfaceApp/Engines.swift)
- [Inspector.swift](../../outputs/surface-edge/Sources/SurfaceApp/Inspector.swift)
- [Notebook.swift](../../outputs/surface-edge/Sources/SurfaceApp/Notebook.swift)
- [NotebookReader.swift](../../outputs/surface-edge/Sources/SurfaceApp/NotebookReader.swift)
- [PDFImport.swift](../../outputs/surface-edge/Sources/SurfaceApp/PDFImport.swift)
- [PaperGeometry.swift](../../outputs/surface-edge/Sources/SurfaceApp/PaperGeometry.swift)
- [RecoveryWriter.swift](../../outputs/surface-edge/Sources/SurfaceApp/RecoveryWriter.swift)
- [SelectionFollower.swift](../../outputs/surface-edge/Sources/SurfaceApp/SelectionFollower.swift)
- [SelectionModules.swift](../../outputs/surface-edge/Sources/SurfaceApp/SelectionModules.swift)
- [StudioStore.swift](../../outputs/surface-edge/Sources/SurfaceApp/StudioStore.swift)
- [StudioView.swift](../../outputs/surface-edge/Sources/SurfaceApp/StudioView.swift)
- [SurfacePreviewCache.swift](../../outputs/surface-edge/Sources/SurfaceApp/SurfacePreviewCache.swift)
- [ToolChrome.swift](../../outputs/surface-edge/Sources/SurfaceApp/ToolChrome.swift)
- [VisualStyle.swift](../../outputs/surface-edge/Sources/SurfaceApp/VisualStyle.swift)

## Surface language commands

Retained experimental parser and typed capability/atomic-plan executor; the normal Spotlight action integration has been removed. [Feature reference](commands.md).

- [SurfaceCommandParser.swift](../../outputs/surface-edge/Sources/SurfaceApp/SurfaceCommandParser.swift)
- [WorkspaceCommands.swift](../../outputs/surface-edge/Sources/SurfaceApp/WorkspaceCommands.swift)
- [SemanticActions.swift](../../outputs/surface-edge/Sources/SurfaceApp/SemanticActions.swift)

## Surface networking

Optional tunnel/proxy panel and privileged helper. Core policies are also linked under SurfaceCore below. [Feature reference](data-and-networking.md).

- [SecureProxy.swift](../../outputs/surface-edge/Sources/SurfaceApp/SecureProxy.swift)
- [main.swift](../../outputs/surface-edge/Sources/SurfaceProxyHelper/main.swift)
- [BoundedProcess.swift](../../outputs/surface-edge/Sources/SurfaceCore/BoundedProcess.swift)
- [SecureProxyPolicy.swift](../../outputs/surface-edge/Sources/SurfaceCore/SecureProxyPolicy.swift)
- [SecureProxyProbe.swift](../../outputs/surface-edge/Sources/SurfaceCore/SecureProxyProbe.swift)
- [SecureTorPolicy.swift](../../outputs/surface-edge/Sources/SurfaceCore/SecureTorPolicy.swift)
- [TorIntegrity.swift](../../outputs/surface-edge/Sources/SurfaceCore/TorIntegrity.swift)

## Surface developer tools

Explicit test/session modes; normal app launch does not invoke these harnesses. [Feature reference](availability.md).

- [NativeAgentSession.swift](../../outputs/surface-edge/Sources/SurfaceApp/NativeAgentSession.swift)
- [Verification.swift](../../outputs/surface-edge/Sources/SurfaceApp/Verification.swift)
- [PDFImportVerification.swift](../../outputs/surface-edge/Sources/SurfaceApp/PDFImportVerification.swift)
- [NotebookReaderVerification.swift](../../outputs/surface-edge/Sources/SurfaceApp/NotebookReaderVerification.swift)
- [NativeCommandVerification.swift](../../outputs/surface-edge/Sources/SurfaceApp/NativeCommandVerification.swift)
- [CPUVerification.swift](../../outputs/surface-edge/Sources/SurfaceApp/CPUVerification.swift)
- [CanvasPerformance.swift](../../outputs/surface-edge/Sources/SurfaceApp/CanvasPerformance.swift)

## Companion entry, lifecycle and core contract

Spotlight/App is the active entry. Option/App/OptionApp.swift is explicitly excluded; its menus/settings/onboarding/voice wiring are not inherited by Edge. The FFI contract wraps a prebuilt core. [Feature reference](availability.md).

- [SpotlightApp.swift](../../outputs/option-edge-search/Spotlight/App/SpotlightApp.swift)
- [OptionSuiteLauncher.swift](../../outputs/option-edge-search/Shared/OptionSuiteLauncher.swift)
- [DefaultBrowserManager.swift](../../outputs/option-edge-search/Option/App/DefaultBrowserManager.swift)
- [OptionApp.swift — excluded standalone entry](../../outputs/option-edge-search/Option/App/OptionApp.swift)
- [RuntimeHealth.swift](../../outputs/option-edge-search/Option/App/RuntimeHealth.swift)
- [StayAwake.swift](../../outputs/option-edge-search/Option/App/StayAwake.swift)
- [Command.swift](../../outputs/option-edge-search/Option/Bridge/Command.swift)
- [CoreEngine.swift](../../outputs/option-edge-search/Option/Bridge/CoreEngine.swift)
- [Snapshot.swift](../../outputs/option-edge-search/Option/Bridge/Snapshot.swift)
- [SpotlightLauncher.swift](../../outputs/option-edge-search/Option/Bridge/SpotlightLauncher.swift)

## Companion language and semantics

Surface catalog discovery remains active. Natural intent/scope recognition and the shared structured parser are retained as research sources and excluded from the companion build. See Search for discovery behavior. [Feature reference](commands.md).

- [SemanticIntent.swift](../../outputs/option-edge-search/Option/Semantics/SemanticIntent.swift)
- [SemanticScope.swift](../../outputs/option-edge-search/Option/Semantics/SemanticScope.swift)
- [SurfaceCommandParser.swift](../../outputs/option-edge-search/Option/Semantics/SurfaceCommandParser.swift)
- [SurfaceEyesView.swift](../../outputs/option-edge-search/Option/Semantics/SurfaceEyesView.swift)
- [SurfaceSemanticBridge.swift](../../outputs/option-edge-search/Option/Semantics/SurfaceSemanticBridge.swift)
- [ToolPrefix.swift](../../outputs/option-edge-search/Option/Semantics/ToolPrefix.swift)
- [WorkspaceLanguage.swift](../../outputs/option-edge-search/Option/Semantics/WorkspaceLanguage.swift)

## Companion UI

Browser, search/quick answers, file/clipboard/result previews, shell placement/chords, shortcuts, settings source, downloads/media and voice. References for Search, Layer and Companions describe those areas separately. [Feature reference](browser.md).

- [AppNotifications.swift](../../outputs/option-edge-search/Option/UI/AppNotifications.swift)
- [BentoSidebar.swift](../../outputs/option-edge-search/Option/UI/BentoSidebar.swift)
- [BentoTheme.swift](../../outputs/option-edge-search/Option/UI/BentoTheme.swift)
- [CalculatorEngine.swift](../../outputs/option-edge-search/Option/UI/CalculatorEngine.swift)
- [CalculatorScope.swift](../../outputs/option-edge-search/Option/UI/CalculatorScope.swift)
- [CapsWorkspaceChord.swift](../../outputs/option-edge-search/Option/UI/CapsWorkspaceChord.swift)
- [DownloadsTray.swift](../../outputs/option-edge-search/Option/UI/DownloadsTray.swift)
- [EdgeBrowserWindow.swift](../../outputs/option-edge-search/Option/UI/EdgeBrowserWindow.swift)
- [EdgeFolderShelf.swift](../../outputs/option-edge-search/Option/UI/EdgeFolderShelf.swift)
- [EdgeOCR.swift](../../outputs/option-edge-search/Option/UI/EdgeOCR.swift) — text recognition, clipboard image text ([OCR](ocr.md))
- [EdgeResultShape.swift](../../outputs/option-edge-search/Option/UI/EdgeResultShape.swift)
- [FaviconStore.swift](../../outputs/option-edge-search/Option/UI/FaviconStore.swift)
- [FileSearchEngine.swift](../../outputs/option-edge-search/Option/UI/FileSearchEngine.swift)
- [Find.swift](../../outputs/option-edge-search/Option/UI/Find.swift)
- [Glance.swift](../../outputs/option-edge-search/Option/UI/Glance.swift)
- [HistoryPanel.swift](../../outputs/option-edge-search/Option/UI/HistoryPanel.swift)
- [ImportUI.swift](../../outputs/option-edge-search/Option/UI/ImportUI.swift)
- [InteractiveFilePreview.swift](../../outputs/option-edge-search/Option/UI/InteractiveFilePreview.swift)
- [Keyboard.swift](../../outputs/option-edge-search/Option/UI/Keyboard.swift)
- [NewTabPage.swift](../../outputs/option-edge-search/Option/UI/NewTabPage.swift)
- [OnboardingView.swift](../../outputs/option-edge-search/Option/UI/OnboardingView.swift)
- [PasswordsUI.swift](../../outputs/option-edge-search/Option/UI/PasswordsUI.swift)
- [Permissions.swift](../../outputs/option-edge-search/Option/UI/Permissions.swift)
- [PiP.swift](../../outputs/option-edge-search/Option/UI/PiP.swift)
- [RootView.swift](../../outputs/option-edge-search/Option/UI/RootView.swift)
- [SettingsView.swift](../../outputs/option-edge-search/Option/UI/SettingsView.swift)
- [ShortcutMap.swift](../../outputs/option-edge-search/Option/UI/ShortcutMap.swift)
- [Shortcuts.swift](../../outputs/option-edge-search/Option/UI/Shortcuts.swift)
- [SidebarWidgetStack.swift](../../outputs/option-edge-search/Option/UI/SidebarWidgetStack.swift)
- [Spotlight.swift](../../outputs/option-edge-search/Option/UI/Spotlight.swift)
- [SpotlightCommands.swift](../../outputs/option-edge-search/Option/UI/SpotlightCommands.swift)
- [SpotlightController.swift](../../outputs/option-edge-search/Option/UI/SpotlightController.swift)
- [Style.swift](../../outputs/option-edge-search/Option/UI/Style.swift)
- [Switcher.swift](../../outputs/option-edge-search/Option/UI/Switcher.swift)
- [Toasts.swift](../../outputs/option-edge-search/Option/UI/Toasts.swift)
- [VoiceFlow.swift](../../outputs/option-edge-search/Option/UI/VoiceFlow.swift)
- [WorkspacePlacement.swift](../../outputs/option-edge-search/Option/UI/WorkspacePlacement.swift)

## WebKit, persistence and import

Page lifecycle, protection, reader/capture, passwords, cookies/import, history, clipboard/compression and native essential routing. Browser reference describes reachable page tools. [Feature reference](data-and-networking.md).

- [BrainCapture.swift](../../outputs/option-edge-search/Option/WebKit/BrainCapture.swift)
- [BrowserCatalog.swift](../../outputs/option-edge-search/Option/WebKit/BrowserCatalog.swift)
- [Carryover.swift](../../outputs/option-edge-search/Option/WebKit/Carryover.swift)
- [ChromiumCookies.swift](../../outputs/option-edge-search/Option/WebKit/ChromiumCookies.swift)
- [ChromiumCrypto.swift](../../outputs/option-edge-search/Option/WebKit/ChromiumCrypto.swift)
- [ChromiumPasswords.swift](../../outputs/option-edge-search/Option/WebKit/ChromiumPasswords.swift)
- [ClipboardStore.swift](../../outputs/option-edge-search/Option/WebKit/ClipboardStore.swift)
- [ContentBlocker.swift](../../outputs/option-edge-search/Option/WebKit/ContentBlocker.swift)
- [CookieImporter.swift](../../outputs/option-edge-search/Option/WebKit/CookieImporter.swift)
- [Downloads.swift](../../outputs/option-edge-search/Option/WebKit/Downloads.swift)
- [Essentials.swift](../../outputs/option-edge-search/Option/WebKit/Essentials.swift)
- [HistoryStore.swift](../../outputs/option-edge-search/Option/WebKit/HistoryStore.swift)
- [NativeSurface.swift](../../outputs/option-edge-search/Option/WebKit/NativeSurface.swift)
- [Ncz.swift](../../outputs/option-edge-search/Option/WebKit/Ncz.swift)
- [PasswordManager.swift](../../outputs/option-edge-search/Option/WebKit/PasswordManager.swift)
- [PasswordStore.swift](../../outputs/option-edge-search/Option/WebKit/PasswordStore.swift)
- [Reader.swift](../../outputs/option-edge-search/Option/WebKit/Reader.swift)
- [SafariCookies.swift](../../outputs/option-edge-search/Option/WebKit/SafariCookies.swift)
- [Shield.swift](../../outputs/option-edge-search/Option/WebKit/Shield.swift)
- [WebContentManager.swift](../../outputs/option-edge-search/Option/WebKit/WebContentManager.swift)
- [WebHostView.swift](../../outputs/option-edge-search/Option/WebKit/WebHostView.swift)
- [WebTab.swift](../../outputs/option-edge-search/Option/WebKit/WebTab.swift)

## Extensions

Installers, manifest handling, packaged resource scheme, content/background/popup/API shim and DNR translation. [Feature reference](browser.md).

- [ExtensionAdblock.swift](../../outputs/option-edge-search/Option/Extensions/ExtensionAdblock.swift)
- [ExtensionInstaller.swift](../../outputs/option-edge-search/Option/Extensions/ExtensionInstaller.swift)
- [ExtensionManager.swift](../../outputs/option-edge-search/Option/Extensions/ExtensionManager.swift)
- [ExtensionSchemeHandler.swift](../../outputs/option-edge-search/Option/Extensions/ExtensionSchemeHandler.swift)
- [ExtensionShim.swift](../../outputs/option-edge-search/Option/Extensions/ExtensionShim.swift)
- [ExtensionsUI.swift](../../outputs/option-edge-search/Option/Extensions/ExtensionsUI.swift)
- [Manifest.swift](../../outputs/option-edge-search/Option/Extensions/Manifest.swift)

## AI client and keys

Client/key implementation retained; Edge voice polish callback is unset. [Feature reference](availability.md).

- [AIClient.swift](../../outputs/option-edge-search/Option/AI/AIClient.swift)
- [AIKeyStore.swift](../../outputs/option-edge-search/Option/AI/AIKeyStore.swift)

## Option Notes editor

Canvas/notebook/PDF editing, board model/actions, vault/migration/backups, captures and search. Edge normally hands off to external Option Notes. [Feature reference](companions.md).

- [Anatomy.swift](../../outputs/option-edge-search/Option/Essentials/Notes/Anatomy.swift)
- [CameraSurface.swift](../../outputs/option-edge-search/Option/Essentials/Notes/CameraSurface.swift)
- [CanvasWorld.swift](../../outputs/option-edge-search/Option/Essentials/Notes/CanvasWorld.swift)
- [DailyLog.swift](../../outputs/option-edge-search/Option/Essentials/Notes/DailyLog.swift)
- [ElementView.swift](../../outputs/option-edge-search/Option/Essentials/Notes/ElementView.swift)
- [HeroIcon.swift](../../outputs/option-edge-search/Option/Essentials/Notes/HeroIcon.swift)
- [ImageStore.swift](../../outputs/option-edge-search/Option/Essentials/Notes/ImageStore.swift)
- [Inspector.swift](../../outputs/option-edge-search/Option/Essentials/Notes/Inspector.swift)
- [Layers.swift](../../outputs/option-edge-search/Option/Essentials/Notes/Layers.swift)
- [MathLabel.swift](../../outputs/option-edge-search/Option/Essentials/Notes/MathLabel.swift)
- [Model.swift](../../outputs/option-edge-search/Option/Essentials/Notes/Model.swift)
- [Notebook.swift](../../outputs/option-edge-search/Option/Essentials/Notes/Notebook.swift)
- [NotesPersistence.swift](../../outputs/option-edge-search/Option/Essentials/Notes/NotesPersistence.swift)
- [NotesRootView.swift](../../outputs/option-edge-search/Option/Essentials/Notes/NotesRootView.swift)
- [NotesRuntime.swift](../../outputs/option-edge-search/Option/Essentials/Notes/NotesRuntime.swift)
- [NotesSearchIndex.swift](../../outputs/option-edge-search/Option/Essentials/Notes/NotesSearchIndex.swift)
- [NotesStore.swift](../../outputs/option-edge-search/Option/Essentials/Notes/NotesStore.swift)
- [NotesStoreActions.swift](../../outputs/option-edge-search/Option/Essentials/Notes/NotesStoreActions.swift)
- [NotesWindow.swift](../../outputs/option-edge-search/Option/Essentials/Notes/NotesWindow.swift)
- [PageStore.swift](../../outputs/option-edge-search/Option/Essentials/Notes/PageStore.swift)
- [Seed.swift](../../outputs/option-edge-search/Option/Essentials/Notes/Seed.swift)
- [SelectionOverlay.swift](../../outputs/option-edge-search/Option/Essentials/Notes/SelectionOverlay.swift)
- [Topbar.swift](../../outputs/option-edge-search/Option/Essentials/Notes/Topbar.swift)
- [WikiLink.swift](../../outputs/option-edge-search/Option/Essentials/Notes/WikiLink.swift)

## Games

Library/source search, format-specific launch runtimes, update installer and nearby-party coordination. [Feature reference](companions.md).

- [AndroidRuntime.swift](../../outputs/option-edge-search/Option/Essentials/Games/AndroidRuntime.swift)
- [GameDetailView.swift](../../outputs/option-edge-search/Option/Essentials/Games/GameDetailView.swift)
- [GamesLibraryView.swift](../../outputs/option-edge-search/Option/Essentials/Games/GamesLibraryView.swift)
- [GamesMultiplayer.swift](../../outputs/option-edge-search/Option/Essentials/Games/GamesMultiplayer.swift)
- [GamesRuntime.swift](../../outputs/option-edge-search/Option/Essentials/Games/GamesRuntime.swift)
- [GamesStore.swift](../../outputs/option-edge-search/Option/Essentials/Games/GamesStore.swift)
- [GamesWindow.swift](../../outputs/option-edge-search/Option/Essentials/Games/GamesWindow.swift)

## Journal

Source retained, native registration disabled without PERSONAL_EDITION. Watch diary, capture, rating and Notes review links. [Feature reference](availability.md).

- [JournalCapture.swift](../../outputs/option-edge-search/Option/Essentials/Journal/JournalCapture.swift)
- [JournalRuntime.swift](../../outputs/option-edge-search/Option/Essentials/Journal/JournalRuntime.swift)
- [JournalStore.swift](../../outputs/option-edge-search/Option/Essentials/Journal/JournalStore.swift)
- [JournalView.swift](../../outputs/option-edge-search/Option/Essentials/Journal/JournalView.swift)

## Build, resources and tests

These inputs are outside the app-source count above and explain packaging, optional dependencies, protocol boundaries and developer verification.

- [Surface package/targets](../../outputs/surface-edge/Package.swift)
- [Build/sign/embed/install](../../outputs/surface-edge/build.sh)
- [Companion target/exclusions/conditions](../../outputs/option-edge-search/project.yml)
- [Core FFI header](../../outputs/option-edge-search/Core/option.h)
- [Surface tests](../../outputs/surface-edge/Tests)
- [Companion tests](../../outputs/option-edge-search/Tests)
- [Semantic training/export/shipping artifacts](../../outputs/option-edge-search/Semantics)
- [Companion assets, page scripts, fonts and runtime resources](../../outputs/option-edge-search/Option/Resources)
- [Surface icons/assets](../../outputs/surface-edge/Assets)
- [Networking vendor resources and integrity manifests](../../outputs/surface-edge/Vendor)
- [Helper service registration and notices](../../outputs/surface-edge/ProxyResources)
- [Workspace command implementation notes](../../outputs/surface-edge/WORKSPACE-COMMANDS.md)
- [Networking policy/recovery/testing boundaries](../../outputs/surface-edge/SECURE_PROXY.md)
- [VPN federation proposal and isolated probe results](../../outputs/surface-edge/VPN_FEDERATION.md)
# Bottom strip

[Feature index](../../FEATURES.md) · [Surface Lite](lite.md) · [Notch island](notch.md) · [Other apps' windows](windows.md) · [Settings](settings.md)

Added **October 18, 2026** (branch `strip/dock`). Both editions; **on by default in auto-hide mode**.

Surface's own resting layer at the bottom edge: a Dock-like row that is also the home for live status on every Mac (notch or not, built-in or external display). It is cropped by the screen edge like the Caps bar: glass, an outline open at the bottom, goo where a sheet grows out of it.

```
[ pinned apps ] | [ other running apps ] | [ live sliver ] | [ • • ● • • • • • • ]
                                          dictation, ✓/✗,    workspaces 1…9
                                          timer, track, count
```

## What it shows

| Part | What | Source |
| --- | --- | --- |
| Pinned apps | Your order. On first run imported **once** from the Dock's `persistent-apps` (apps only: spacers, folders, URLs skipped; an app pinned twice counts once). An uninstalled app keeps its pin but has no tile until it's back. | `edge.strip.pins.v1` |
| Running apps | Regular apps not pinned, in launch order. | `NSWorkspace.runningApplications` |
| Tile | The real app icon (`NSWorkspace` icon), no label until hovered. Running: a 4pt dot under it (the app in front: a 10pt white capsule). Hidden (⌘H): dimmed. Launching: breathes until the app is up. | |
| Badge | The red chip Apple's Dock would show (unread mail, messages), read from the Dock's own tiles over Accessibility (`AXStatusLabel` + `AXURL`), so it works while Apple's Dock is hidden. Polled every 3s only while the strip is up, plus once on reveal (~36ms, off the main thread). | `EdgeStrip.readBadges` |
| Live sliver | Only when something is live, in the **island's priority** (`IslandSelection`): Caps chord being held, dictation (4-bar waveform / spinner / ✓ / ⚠), terminal notice (✓ ✗ • in your palette), timers (`4:12`), playing track (artwork in a progress ring), then counts (the paste stack hook). At most four chips. | `EdgeLiveStatus` (shared with the island) |
| Workspace dots | Caps+1…9: the current one in Tiling's cyan, ones holding windows brighter, a tick per window (max 3). Other apps' windows count too while workspaces hold them (`ForeignSpaces.windowCounts`, updated on every state commit, `ForeignSpaces.didChange`). | `EdgeWorkspaces.windowCounts()` |

## Using it

| ID | Entry | Behavior |
| --- | --- | --- |
| ST01 | Pointer presses the bottom edge under the strip (its width + 48pt, never the corners) | After a 60ms dwell (a fly-by doesn't count) it springs up out of the edge with a slight overshoot. It follows you to the display whose edge you press (one strip, like the Dock); a display stacked below doesn't count as an edge. |
| ST02 | Pointer leaves | Slides away 0.35s later; coming back cancels. Never mid-press or mid-drag. |
| ST03 | Bar up (Caps tap) or Caps held | The strip is the **floor of the Caps stack**: it comes up on the bar's display and the bar, its sheet and dictation row ride on it; its top crops them exactly like the screen edge does. The terminal sheet takes the floor's place while it's open. |
| ST04 | Settings → Strip → Show: **Always** | Stays up. |
| ST05 | Apple's Dock still at the bottom (not replaced) | The bottom edge is the Dock's: the strip shows only as the floor (ST03) or with Always. On the left/right the edge is free and ST01 works. |
| ST06 | Click a tile | Activates the app, or launches it; a running app with no windows gets its reopen event (a new window), as from the Dock; ⌘H'd apps unhide. Tiles wobble on press. |
| ST07 | Hover a tile | Its name above it. After 0.32s its windows rise out of the strip as cards (live ScreenCaptureKit thumbnails when Screen Recording is allowed and Settings → Tiling → Window thumbnails is on, else the app icon; minimized ones dimmed with a dash). Sliding along switches the peek at once. Click a card: that window comes forward (unminimized if needed). More than five: a +N card opens the right-click card. |
| ST08 | Scroll over a tile | Its windows come forward one after another (one step per notch / 18pt of trackpad, at most every 0.12s). |
| ST09 | Right-click a tile | A card (not an NSMenu) rises: its windows as rows (click → that window), **Keep in strip / Remove**, **Quit** (running apps). Closes when the pointer leaves it and the strip. |
| ST10 | Drag a tile along the strip | Axis-locked at 6pt: sideways slides it (y locked, x 1:1); the other pinned tiles open a gap where it would land; let go to reorder. A running app slid into the pinned part is pinned there. While dragging, the dots open into numbered drop targets. |
| ST11 | Drag a tile onto a dot | Sends that app's front window to that workspace (`moveAppWindows` → `ForeignSpaces.shared.send(pid:to:)`: a short toss that way, then it parks), like Caps+Shift+digit with it in front. With Settings → Tiling → Workspaces include other apps off, it toasts where to turn it on. |
| ST12 | Drag a pinned tile up and away | Lifted (free 1:1 in both axes); past 1.2 tiles above the strip it tilts and fades, the others close the gap; let go: unpinned with a poof (a running app moves to the running part). Back over the strip it slides again. |
| ST13 | Drop files on a tile | The tile swells; the files open in that app. |
| ST14 | Click a live chip | Dictation: stops and inserts (Caps+D's action). Terminal notice: opens its session. Track: play/pause. |
| ST15 | Click a dot | Switches workspace the way Caps+digit does (`SpotlightController.workspaceDigit`). |
| ST16 | Type in the bar | Tiles whose app name doesn't match (prefix, word prefix, initials, or ≥3 letters in order from a word start) desaturate and dim. `@`, `>` and the like don't dim. |
| ST17 | Empty bar | Live things lead the idle cards: a terminal notice (↵ opens it) and a playing track (the media card). Dictation keeps its own row; timers keep their card. |

## Replace Dock (opt-in)

Asked **once**, in a bar card, the first time you open the bar (not Lite's own open at launch) with the strip on and Apple's Dock at the bottom: *Replace the Dock with the strip*. ↵ accepts; typing, Esc or closing the bar says no (it never asks again). Settings → Strip also has **Replace Dock** / **Restore Dock**.

- **What it does:** sets `com.apple.dock` `autohide = true` and `autohide-delay = 1000` and restarts the Dock (`killall Dock`). Apple's Dock keeps running, so Mission Control, Spaces, ⌘Tab, badges and minimizing keep working; it just never comes up.
- **Backup:** the exact previous values, or "absent" for keys that weren't there (most Macs have no `autohide-delay`; restoring deletes it rather than writing a default). Applying twice never records our own values as "before"; a reinstall that finds it replaced with no record restores Apple's defaults. `edge.strip.dock.backup`.
- **Restored:** by **Restore Dock** (and forgotten), when the Strip module is switched off, and on every clean quit of Surface (re-applied at the next launch). A crash leaves it replaced: `defaults delete com.apple.dock autohide-delay; killall Dock` (and `autohide` if you didn't have it).
- The test copy (`--test-instance`) never writes it, and `tools/strip-test` exercises the same code against a throwaway domain only.

## Notch

The notch island is now **Mirror to notch** (Settings → Strip), **off by default** on every Mac; a Mac that had explicitly switched the Notch module on keeps it. Its code and module are unchanged; it reads the same `EdgeLiveStatus` as the strip.

## How it's built

| Part | File |
| --- | --- |
| Geometry, layout and its motion, hit-testing, sheets, reveal/hover machines, bar filter, Dock pins import, Replace/Restore Dock plans (pure) | [StripCore.swift](../../outputs/option-edge-search/Option/UI/StripCore.swift) |
| Drawing | [StripView.swift](../../outputs/option-edge-search/Option/UI/StripView.swift) |
| Window, pins, running apps, badges, events, actions, Replace Dock | [StripController.swift](../../outputs/option-edge-search/Option/UI/StripController.swift) |
| What's live, read once for the strip and the island | [EdgeLiveStatus.swift](../../outputs/option-edge-search/Option/UI/EdgeLiveStatus.swift) |

- **One strip, on one display at a time**: where you press the edge, or the bar's display. (Not one per display: the Dock behaves the same way, and one set of springs and monitors stays cheap.)
- **Window**: a borderless non-activating panel the display's width, 400pt tall (60 strip + 300 sheet room + 40 under the edge), level floating+4 (above the bar so its top crops the bar; under the window switcher; Apple's Dock at 20 is above it). All Spaces, full-screen auxiliary. `ignoresMouseEvents` stays true except while the pointer is over the strip or its sheet; global + local mouse monitors (moves, drags, ups, scrolls) watch for the edge.
- **Events**: the SwiftUI view takes none. An AppKit view over it gets every click, drag, right-click, scroll and file drop and hit-tests it against the same `StripLayout` the view draws from.
- **Motion**: `GooSpring`s on the display link: presence (up/down, overshoot), morph (layout changes), sheet (rises out of the strip), drop (dots open into targets). Apps launching or quitting move everything from one value: `StripLayout.blend` merges the old and new rows (longest common run), mixes every width and gap and lays the row out again, so neighbours never overlap, the glass is always exactly the row's width, and a change mid-motion continues from where things are. The bar's floor padding animates with the same spring constants.
- **Goo**: the strip, the sheet and a neck over the tile are closed into one body (`GooShape`, 10pt fillet): the sheet melts out of the strip. A lone strip is drawn as a plain rounded rect (no closing pass).
- **Sizes**: 44pt tiles, 8pt gaps and insets (EdgeLayout.gap), 60pt above the edge, 20pt top corners; sheet cards 168×135 at 16pt corners in a 24pt sheet. Too many apps: tiles shrink (never under 28pt).

State: `edge.strip.pins.v1`, `edge.strip.imported`, `edge.strip.mode` (`autoHide`/`always`), `edge.strip.dock.accepted|backup|asked`. The test copy keeps all of them in memory.

## Hooks for other features

- `EdgeStrip.shared.addCountSource { [StripCount] }`: a count-shaped live chip (the paste stack: nothing on this branch makes one).
- `EdgeStrip.shared.moveAppWindows = { pid, n in … }`: tile → workspace (ST11).
- `EdgeLiveStatus.shared.addSource { [IslandTimer] }` (what `EdgeIsland.addSource` now forwards to): timers in both homes.

## Journal

`strip.reveal {by}`, `strip.open {app, running}`, `strip.window`, `strip.cycle`, `strip.pin`, `strip.unpin`, `strip.reorder`, `strip.move {app, number}`, `strip.quit`, `strip.drop_files {app, count}`, `strip.workspace {number}`, `strip.import {count}`, `strip.dock_ask`, `strip.dock_replace`, `strip.dock_restore`; existing `voice.dictate`, `media.toggle`, `edge.open_notice` with `source: strip`.

## Checks

- `tools/strip-test/run.sh` (offline, 103 checks): geometry on mocked displays (offset, stacked), layout order and 8pt spacing, shrinking, every frame of the layout motion (no overlaps, no retarget jump, a moved tile), hit-testing, sheets on the display and within the panel, the reveal/hide machine (dwell, fly-by, grace, press, Dock owns the edge, floor, Always), the hover machine, 15 filter cases, live priority reused from the island, pins parsed from `sample-dock.plist`, Replace/Restore against `studio.surface.edge.test.fakedock` (absent keys deleted, float kept, double apply, lost record).
- `tools/strip-test/run-ui.sh` (`SHOTS=dir`): the real `StripView` rendered offscreen in 14 states (resting, live + badges, hover label, peek, peek rising, right-click card, Caps floor with filter, Caps floor, drag to a workspace, pull-off, poof, mid-morph, rising, 1280pt display). Liquid Glass can't render offscreen: those use a dark stand-in fill.

## Verified on screen (test copy, October 18)

With a `--test-instance --test-visible` copy beside the running app: the strip in real glass with this Mac's Dock pins imported (uninstalled Zen/Windscribe/scrcpy pins hidden), running apps after the hairline, the front app's capsule, workspace dots; with Apple's Dock owning the edge it stayed hidden; the bar opened (debug `show`) sitting on it, cropped by its top. The Dock's AX items expose `AXStatusLabel`/`AXURL` (read in ~36ms); no app had a badge at the time.

## Not verified live

Pointer-driven parts weren't exercised on screen (the developer was using the Mac): edge reveal and hide timing, hover peek with real thumbnails, scroll cycling, right-click card, drags (reorder, pull-off, dots), file drops, a real badge, the Replace Dock flow against the real Dock (only the fake domain), Caps-held floor (the test copy has no Caps), multiple displays, full-screen Spaces.
# Surface

[Feature index](../../FEATURES.md) · [Workspace commands](commands.md) · [Shortcuts](shortcuts.md)

Surface's documents contain content identities and placements of that content. A board organizes live views; a design is a bounded composition; a text note edits Markdown; an image can host canvas annotations. Surface notebooks are ordered collections of these surfaces. They are separate from Option Notes' canvas/PDF notebooks.

## Documents, navigation and collections

| ID | Feature / entry point | Behavior and limits |
| --- | --- | --- |
| S01 | File → New canvas | Create a fresh workspace/root board. Existing edited history is archived before replacement. |
| S02 | Help → Open example canvas | Load the demonstration with poster, notes, nested board and example relationships. First launch without recovery also uses example content. |
| S03 | Open / macOS file-open | Load a `.surface` document, validate format/history/content, restore its current snapshot. Opens the first supplied file. |
| S04 | Save / Save As | Save workspace, embedded images and every history branch into `.surface`; atomic writes and surfaced file errors. Save As chooses a new destination. |
| S05 | Automatic recovery | Save changes to a dedicated recovery file, flush edits at termination, restore navigation/camera preferences per document. [Storage details](data-and-networking.md). |
| S06 | Nested canvas | Insert → Nested canvas places an independent board inside the current canvas. Double-click to enter. |
| S07 | Design surface | Insert → Design surface creates a bounded composition with its own geometry/output size. |
| S08 | Text note | Note button / Insert → Text note creates Markdown content with an initial draft. |
| S09 | Breadcrumbs and Back | Navigate parent surfaces and previous path components; the current crumb offers nested children. Going back at root returns home. |
| S09a | Canvas navigation | View → Canvas navigation / Control-Command-N / toolbar stack button / assignable Navigator Caps action. Uses the Clipboard/Games bottom result sheet, with parent levels stacked above sibling cards. Left/Right centers sibling elements in their parent board; Down expands; Up restores the same placement; Return opens in Surface; Escape closes. Supports notebooks, boards, canvas/text pages and nested surfaces on images. Bare images and ancestor cycles are excluded. Navigation preserves artwork/history; rapid reversals replace the previous camera animation. |
| S10 | Rename surface | Edit → Rename surface, Option-Command-R, context menu, or Properties title field. Renames current/selected content identity. |
| S11 | Linked view | Insert → Linked view places existing content again. All views share source content; geometry of each placement remains local. Ancestor/self placement is refused. |
| S12 | Duplicate placed content | Duplicate a selected placement to create another linked view with an offset. Removing a view removes its placement rather than erasing the shared source. |
| S13 | Outgoing links/backlinks | Properties → Link a surface, remove a reference, navigate linked content, inspect Linked from. Separate from placing a live view. |
| S14 | Notebook creation | Insert → Notebook creates an ordered collection with an initial canvas page. |
| S15 | Notebook pages | Canvas, text and PDF pages stack in a continuous vertical document. Draw or type directly on the sheets. The compact reader bar adds canvas/text pages; long text sheets grow within the document scroll. |
| S16 | Notebook navigation | Open a notebook or any of its pages into the same bounded vertical reader. Current-page counter, previous/next controls and a page-number field provide navigation; Page Up/Down scroll and Home/End jump to the first/last page. Fit-width zoom ranges from 50% to 300%, with bounded horizontal scrolling when enlarged. Only nearby sheets and the current editor remain resident. Nested surfaces still open independently through the existing hierarchy. |
| S17 | Board covers | Properties → Board cover: live contents, title cover or independent chosen image. Cover choice is shared with content identity. |
| S18 | Peek covered board | Hold Option over a covered board in its parent canvas to reveal live contents until released. |

Sources: [main menus](../../outputs/surface-edge/Sources/SurfaceApp/main.swift), [StudioStore](../../outputs/surface-edge/Sources/SurfaceApp/StudioStore.swift), [Notebook and covers](../../outputs/surface-edge/Sources/SurfaceApp/Notebook.swift), [Inspector](../../outputs/surface-edge/Sources/SurfaceApp/Inspector.swift), [Document model](../../outputs/surface-edge/Sources/SurfaceCore/Document.swift).

## Canvas interaction and creation

| ID | Feature / entry point | Behavior and limits |
| --- | --- | --- |
| S19 | Pan | Pan tool, Space-drag, middle-button drag, or normal scrolling. |
| S20 | Zoom / Fit | Pinch, Option/Command-scroll, or toolbar Fit / Command-0. Zoom anchors at pointer and ranges from 0.08× to 6×; camera is remembered. |
| S21 | Select and marquee | Click artwork/view, Shift-click for additive selection, drag empty canvas to marquee; persistent Multi-select toggle adds/removes items. |
| S22 | Cycle overlap targets | Command-click cycles the overlapping hit candidates. Ring holes allow hit routing to underlying content; text hit areas include gaps between letters. |
| S23 | Move / nudge | Drag selection; arrows move 1 pixel, Shift-arrows 10. A 4-point drag threshold prevents small clicks becoming movement. |
| S24 | Independent movement | Option-drag/nudge moves a member independently of movement relationships, changing its rest offset; alignment rules can still apply. |
| S25 | Resize | Selection handles resize using visible geometry. Text/design/image placements receive proportional treatment; Shift can constrain other resize cases. Outlines participate in visible bounds. |
| S26 | Snapping | Magnet toggle snaps nearby visible geometry; Control bypasses. Snap guides appear during manipulation. Preparation begins after drag threshold. |
| S27 | Lock/unlock | Selection menu toggles object/view locks; locked selections are protected against normal move/delete and command edits. |
| S28 | Text creation | Text tool then click; double-click blank canvas; or type printable characters while canvas has focus. Double-click existing text to edit. |
| S29 | Inline note editing | Double-click a placed note to edit in context. Command-Enter/Escape finish the inline edit. |
| S30 | Shape tools | Shapes menu: Rectangle, Ellipse, Ring. Drag to create; the armed shape can be changed from the menu. Commands also create straight paths/lines. |
| S31 | Freehand ink | Draw tool collects stroke samples. Canvas supports both dragged and latched stroke interaction; Escape clears draft ink. |
| S32 | Copy/cut/paste artwork | Preserve selected objects, placements, referenced content and applicable relationships through the app's custom pasteboard payload; regenerate identities on paste. |
| S33 | Paste text, images and PDFs | Plain text becomes canvas text; PNG/TIFF payloads become image content; PDF payloads become ordered, annotatable notebooks. |
| S34 | File drops | Import images, PDFs and UTF-8 `.md`, `.txt`, `.markdown` notes at drop coordinates; multiple files are offset. PDFs become notebooks with one annotatable image surface per page, preserving embedded PDF artwork, crop and rotation. Image/PDF source payloads must be below 50 MB; expanded PDF page assets must total below 100 MB. Malformed and password-locked PDFs show a status message without importing partial pages. |
| S35 | Image annotation | Double-click image to enter its canvas. Add text, shapes, ink, notes, nested surfaces and pasted objects. Clip annotations to the source image in nested views and exports. |
| S36 | Duplicate / delete | Command-D copies drawn objects with fresh IDs/offsets, duplicates placements as linked views. Delete removes selection and cleans relationship membership. |
| S37 | Escape on canvas | Clear selection/draft ink and return to Select. Text editing consumes Escape in its own editor context. |

Sources: [CanvasView](../../outputs/surface-edge/Sources/SurfaceApp/CanvasView.swift), [CanvasInteractions](../../outputs/surface-edge/Sources/SurfaceApp/CanvasInteractions.swift), [StudioStore](../../outputs/surface-edge/Sources/SurfaceApp/StudioStore.swift), [render engines](../../outputs/surface-edge/Sources/SurfaceApp/Engines.swift).

## Properties, typography and relationships

| ID | Feature / entry point | Behavior and limits |
| --- | --- | --- |
| S38 | Following selection tools | Contextual glass bar follows selection; expand Typography, Fill, Outlines, Geometry and Alignment modules; menu contains edit/lock/duplicate/delete/relationships. |
| S39 | Typography | Installed font-family chooser, family styles, font size and direct property wheels. System font fallback and optical sizing preserve rendered text geometry. |
| S40 | Fill | Color picker and selection style changes; supported shared-fill rules propagate changes. |
| S41 | Ring hole | Ring hole ratio changes inner geometry, including transparency/hit behavior. |
| S42 | Object geometry | Edit visible position/size, object name and type-specific dimensions/properties in selection controls/Properties. |
| S43 | Stack order | Move object Forward/Backward in Properties. |
| S44 | Multiple outlines | Add any number of original-geometry-following outlines; each has color, width and enabled state. |
| S45 | Outline ordering | Reorder layers, remove layers; top inspector row paints in front. |
| S46 | Outline scaling | Per-outline fixed width or scale-with-object setting. Resize/scale retains each layer's selected behavior. |
| S47 | Direct property wheels | Scroll, drag or enter values for supported style properties; arrow keys and accessibility increment/decrement also work. Continuous adjustments form one history edit when finished. |
| S48 | One-time visual alignment | Left/center/right/top/middle/bottom: multiple items align relative to selected artwork bounds; a single item can align to canvas. |
| S49 | Move together relationship | Connect two or more objects/views. Moving one follows transitive movement membership and preserves offsets. |
| S50 | Belong together relationship | Record membership/association without turning objects into a containment tree. |
| S51 | Persistent alignment rules | Align left or horizontal centers; rules enforce later edits using visible bounds. Contradictory overlapping horizontal rules replace prior rules. |
| S52 | Share fill / share outlines | Relate selected drawn objects to propagate those property channels. Unspecified channels remain independent. |
| S53 | Relationship visibility/removal | Relate menu, Properties list, selection menu, View → Reveal relationships, or hold Control. Remove existing rules from the list. |
| S54 | Scale selection | Properties scale factor, optional Scale offsets too. Movement-linked objects and opted-in outlines scale; locked members remain protected. |
| S55 | Composition properties | Design width/height, paper color, corner radius, border width/color. |
| S56 | Independent output size | Export pixel dimensions preserve composition ratio and do not move objects or change composition geometry. |
| S57 | Surface/view properties | Rename content, inspect live-view count, edit local view width, enter it, duplicate or remove its placement; source content remains shared. |

Sources: [SelectionModules](../../outputs/surface-edge/Sources/SurfaceApp/SelectionModules.swift), [DirectControls](../../outputs/surface-edge/Sources/SurfaceApp/DirectControls.swift), [Inspector](../../outputs/surface-edge/Sources/SurfaceApp/Inspector.swift), [property relationships](../../outputs/surface-edge/Sources/SurfaceCore/PropertyRelationships.swift), [document relationships](../../outputs/surface-edge/Sources/SurfaceCore/Document.swift).

## Notes, history, export and rendering

| ID | Feature / entry point | Behavior and limits |
| --- | --- | --- |
| S58 | Markdown Write / Read | Surface text-note editor has editable source and rendered read view. Properties manages its explicit references. |
| S59 | Undo / redo | Workspace edits use snapshot history; continuous gestures commit once and no-change edits create no moment. Inline native text editor can consume its own undo/redo first. |
| S60 | History tree | History toolbar / Shift-Command-H shows moments and preserved branches with dates/labels. |
| S61 | Read-only past preview | Select a history moment and navigate it without mutating it. Return to present restores editing. |
| S62 | Branch here | Continue from a selected past moment. Other futures remain in the tree and in saved documents. Redo chooses the most recent child; history UI reaches other branches. |
| S63 | Embedded asset deduplication | `.surface` saves image payloads once by content hash across snapshots and restores them on load. |
| S64 | PNG export | File → Export PNG / Command-E or design Properties. Available for design/image; renderer includes nested content and image clipping. Up to 8192 pixels/side and 40 million pixels total. |
| S65 | Vector editing/rendering | Actual canvas content and exports use the vector renderer; previews are a separate representation. |
| S66 | Large-canvas caches | Bounded geometry cache, 128 MB decoded-image cache and 64 MB nested-preview cache. Edits, descendant/cover changes, history and increased zoom invalidate relevant previews. |
| S67 | Coalesced interaction | Move/resize updates up to 120 times/second and flush on release; freehand drawing keeps every sample. Toolbar updates coalesce at 16 ms; camera writes settle after scrolling. |

Sources: [StudioView](../../outputs/surface-edge/Sources/SurfaceApp/StudioView.swift), [History/file encoding](../../outputs/surface-edge/Sources/SurfaceCore/History.swift), [RecoveryWriter](../../outputs/surface-edge/Sources/SurfaceApp/RecoveryWriter.swift), [StudioStore export](../../outputs/surface-edge/Sources/SurfaceApp/StudioStore.swift), [SurfacePreviewCache](../../outputs/surface-edge/Sources/SurfaceApp/SurfacePreviewCache.swift), [Engines](../../outputs/surface-edge/Sources/SurfaceApp/Engines.swift). Performance facilities describe implementation, not a display-frame-rate guarantee.
# System words

[Feature index](../../FEATURES.md) · [Bar words](bar-words.md) · [Surface Lite](lite.md) · [r/macapps refresh](../scout/macapps-2026-10-18.md)

Added **October 18, 2026**. Three more bar words, picked from the October 18 scout: the Mac's battery and its Bluetooth devices (ToothFairy, AirBattery, docktor's battery widget), what's using the Mac and quitting it (macmon, Stats, force-quit apps), and ejecting drives (Ejectify, the menu-bar eject apps). Both editions. They are bar words in the [bar-words](bar-words.md) sense: a word, a card of tiles, ↵ acts. They live in their own files and plug into the bar-word routing, so `Spotlight.swift` didn't change.

| Module | Words |
| --- | --- |
| Media | `battery`, `bluetooth`, `connect …`, `disconnect …` |
| Answers | `cpu`, `memory`, `top`, `quit …`, `force quit …` |
| Files | `eject`, `eject all`, `eject …` |

Same tiles as the other bar words: mosaic rows, a short last row widens, 8pt between and around, ←/→ walks, the squash-and-tip press. Color roles: Bluetooth blue `#0a84ff` (filled = connected), battery green / charging yellow / low red, CPU orange, memory cyan (yellow, then red, under memory pressure), drives the Files accent, stop red.

## Words

| ID | Typed | Card and ↵ |
| --- | --- | --- |
| SW01 | `bluetooth`, `bt`, `devices` / `battery`, `batteries` | One wide battery tile (a drawn gauge, percent, `2:15 left` / `0:40 to full` / `Charged` / `On power, holding`, system draw in W, adapter W, temperature, health % and cycles on the right; health under 80% is red), then paired Bluetooth devices two per row (`battery` lists only the connected ones, so it stays a readout; `bluetooth` lists them all): connected first, then audio, input, other, phones and computers last. Connected tiles show their battery (`L 80% R 75% case 40%` for AirPods, one level for the rest, ≤20% red). ↵ on the battery opens Battery settings; ↵ on a device connects or disconnects it (not on `battery`, which is a readout) (a spinner while it works; `Couldn't connect` in red if it fails). At most 11 devices. |
| SW02 | `connect sony`, `connect to wh-1000xm5`, `disconnect airpods`, `bluetooth jbl`, `airpods battery`, `battery mouse` | The matching devices only (no battery tile); a name nothing matches lists nothing and ranks under files (`connect to wifi`, `battery percentage`). Names match by word prefix, or anywhere from three letters (`xm5`), and by **maker**: the vendor ID maps to Sony, Bose, Apple, Microsoft Xbox, Logitech, Samsung, JBL, Sennheiser, B&O, Anker Soundcore, Jabra, Nintendo, Valve, Razer, Google, so `connect sony` finds a `WH-1000XM5`. `connect` puts unconnected ones first; `disconnect` shows only connected ones. `bluetooth on/off` is not a word (radio power isn't ours). |
| SW03 | `cpu`, `top`, `processes`, `activity`, `stats` / `memory`, `ram` | A system tile (CPU % across all cores with core count; memory used of total, colored by pressure; ↵ opens Activity Monitor), then the top ten **apps**, two per row: icon, name, CPU %, memory, `×12` when it's several processes. A Chrome renderer counts toward Google Chrome: every process is grouped under the outermost `.app` it runs from, and XPC services with no app of their own (WebKit's GPU, content and networking processes) under the app macOS holds responsible for them, so Safari is one tile. Plain tools (`node` from a terminal) stand alone. Sorted by CPU, or memory for `memory`/`ram`. Refreshes every 1.5 s while the card is up and stops when it closes. ↵ arms a tile (red outline, `↵ quit`), ↵ again quits it like ⌘Q; still open after 2 s (a save prompt, usually) → `Still open`, and ↵ again force quits. |
| SW04 | `quit slack`, `kill node`, `force quit chrome`, `quit`, `force quit` | Only the matches (twelve, quittable ones only; when an app matches, only apps, so `quit safari` doesn't list `com.apple.Safari.History`). With a name typed, ↵ quits the first match at once and the bar goes away, with a toast (`Quit Slack`, or `Slack is still open` with a hint to force quit). `force quit …` force quits. A bare `quit` / `force quit` names nothing (its first tile is whatever is busiest that second), so ↵ arms first and ↵ again quits, as in `cpu`. With no running app by that name the card ranks under files, so `kill bill` still searches. |
| SW05 | `eject`, `eject all`, `eject sandisk`, `unmount backup` | Ejectable volumes (external and removable drives, disk images, network shares; never the startup disk or system volumes; `-nobrowse` images stay hidden as Finder hides them), two per row: icon by kind, name, used bar, `12 GB free of 64 GB`. `Eject all` tile when there are two or more (first and selected for `eject all`). ↵ ejects as Finder does; something holding a file open → `In use · ↵ force`, and the next ↵ on that volume force-unmounts every volume on that disk and ejects it (a disk image detaches). `Eject all` never forces, and neither does a disk that also holds the startup volume. The list follows mounts and unmounts while the card is up. Nothing mounted: a green `Nothing to eject`. |

Ranks: `battery`/`bluetooth`/`cpu`/`memory` sit like `airpods` (rank 2: above apps and files, under answers); `quit …` with a running match is rank 1, without one rank 7. `kill :3000` / `kill port 3000` stay the [port word](bar-words.md).

## Permissions

Reading needs none. The battery is IOKit (`IOPSCopyPowerSourcesInfo` + the `AppleSmartBattery` registry entry); devices come from `system_profiler SPBluetoothDataType -json` (~0.2 s, a separate process, so listing never triggers the Bluetooth prompt); activity is libproc + Mach host statistics. **Connecting and disconnecting** go through IOBluetooth, so the first one asks for Bluetooth access (`NSBluetoothAlwaysUsageDescription` added to both Info plists). Quitting other users' processes isn't possible (no root); they don't show.

## Implementation

- [EdgeSystemWords.swift](../../outputs/option-edge-search/Option/UI/EdgeSystemWords.swift): the parser (`BarWord.readSystem`), pure Foundation. `BarWord` gained three cases (`devices`, `activity`, `eject`) in [EdgeBarWords.swift](../../outputs/option-edge-search/Option/UI/EdgeBarWords.swift).
- [EdgeDevices.swift](../../outputs/option-edge-search/Option/UI/EdgeDevices.swift): battery, the profiler JSON parser (macOS 12+ `device_connected`/`device_not_connected`, and the older `device_title`), ranking, maker table, `setConnected` (IOBluetooth `openConnection`/`closeConnection`).
- [EdgeActivity.swift](../../outputs/option-edge-search/Option/UI/EdgeActivity.swift): two `proc_pid_rusage` samples (CPU time is in Mach ticks on Apple silicon, converted through the timebase), `ri_phys_footprint` for memory, grouping by outermost `.app`, `HOST_CPU_LOAD_INFO`, memory used = app + wired + compressed, `kern.memorystatus_vm_pressure_level`. Quitting: `NSRunningApplication.terminate` / `forceTerminate` for apps, SIGTERM / SIGKILL for plain processes. Never this app, pid ≤ 1, or the session's plumbing (Dock, WindowServer, loginwindow, Finder, …).
- [EdgeVolumes.swift](../../outputs/option-edge-search/Option/UI/EdgeVolumes.swift): `mountedVolumeURLs`, DiskArbitration for disk-image detection and the whole disk, `NSWorkspace.unmountAndEjectDevice`, `diskutil unmountDisk force` + `eject` for force.
- [EdgeSystemWordCards.swift](../../outputs/option-edge-search/Option/UI/EdgeSystemWordCards.swift): routing (module, rank, badge, tile count, ↵), the three models and cards. The shared tile pieces in EdgeBarWordCards.swift were renamed `BarMosaic`, `BarIconBadge`, `BarAccent`, `.barTile` and made internal for this.

## Journal

`devices.connect`, `devices.disconnect` (device kind, result), `process.quit` (app or process, force, process count, result, word or key), `volume.eject` (kind, force, result, source) in `docs/actions/registry.json`. Never device, app or volume names.

## Tests

`tools/system-words-test/run.sh` (77 checks, offline): 37 word forms including neighbours that must not change (`airpods`, `mute`, `kill :3000`, `timer 5m`, `quitting time`, `power`, `connected papers`); the battery registry fixture (cycles, health, telemetry watts, a negative amperage handed back unsigned) and the live battery; profiler JSON in both shapes, duplicates, garbage, ranking, maker matching, and the live profiler list against `system_profiler` itself; activity with a `yes` process of its own (≈100% of a core, top five, `kill yes` finds it, SIGTERM quits it), Chrome-helper grouping, WebKit services counted toward their browser, memory sort, system totals, never itself / Dock / pid 1; eject with a 4 MB `-nobrowse` disk image of its own (listed as an image, hidden from the normal list, found by name, busy with an open file, forced, then a clean eject that also detaches it). Measured here: profiler 220 ms for 18 paired devices; one activity sample of 435 processes 3 ms.

**On screen** (a `--test-instance --test-visible` copy of this build, queries typed through the `held` debug hook, October 18): `battery`, `bluetooth`, `connect sony` (the WH-1000XM5 plus both Sony controllers, by maker), `cpu`, `quit safari` and `eject` render as described; a browsable 4 MB test image was ejected through the real bar with ↵ (`run=1`) and detached. The test copy sat at 0–13% CPU (Debug) with the activity card refreshing.

**Not verified:** connecting or disconnecting (the Bluetooth prompt, real links), quitting a real app (`terminate`, the save-prompt path), ejecting a real USB drive or network share, ←/→ and mouse clicks in the cards. Both targets build.

## Not yet

Pairing new devices; Bluetooth radio on/off; battery notifications (low peripheral battery); a charge limit (macOS 26 has one in Settings; BattOpt-style limits need a privileged helper); per-app energy; `quit all`; ejecting before sleep.
# Window switcher

[Feature index](../../FEATURES.md) · [Layer and workspaces](layer.md) · [Shortcuts](shortcuts.md) · [Search](search.md) · [Surface Lite](lite.md)

Added **October 16, 2026** (branch `overnight/window-switcher`). From the [r/macapps scout](../scout/macapps-2026-10.md), citizen #1.

Every other app's window is a result, not just every app. Two ways in: type in the bar (Windows card), or hold Caps and press Tab (the switcher).

## Using it

| ID | Entry point | Behavior and limits |
| --- | --- | --- |
| W01 | Type two or more characters in the bar | **Windows card**: up to 8 windows whose title or app name match, as a mosaic of live thumbnails (justified rows, at most two, 8pt gaps, no voids). Ranking uses the bar's app tiers (exact, prefix, word prefix, acronym, contains) on titles and app names, titles first at the same tier, words split across both (`safari github`), then a subsequence anchored on a word start (`dwnlds`). Ties keep most-recently-used order. Card rank: a title that names a window ranks with open tabs (2); an app-name hit sits right behind the Apps card (3); acronym/contains 14; subsequence 30. ←/→ walk tiles, ↵ or click raises that window. |
| W02 | Caps + Tab | **Switcher**: every window as one mosaic, centered on the screen under the pointer. Opens on the window behind the one you're in (Cmd-Tab semantics). Tab steps on, Shift+Tab back, ←/→ step, ↑/↓ a row. Releasing Caps raises the selected window; Esc cancels. Releasing Tab does nothing; only Caps up commits. A quick Caps+Tab (released within 120 ms) switches to the previous window without drawing anything. |
| W03 | Letter hints | Each tile in the switcher has a letter: the app's initial first (Safari s, a second Safari a, Slack l), then the title's initials, then any free letter; 26 at most. Letter down jumps the selection there (the tile wobbles); letter up raises it. While the switcher is open, letters are hints only; other Caps chords don't fire. Letters keep their tile for the whole hold even as the list refreshes. |
| W04 | Click a tile | Raises that window (press wobbles; sliding off the tile cancels the click). |
| W05 | The Tab hint in the Caps shortcuts row | Opens the switcher without a held Caps. It stays until a click on a tile, ↵, a letter or Esc; a click anywhere else cancels. |
| W06 | Raising | Unhides the app, restores a minimized window, sets it main, raises it (`kAXRaiseAction`), marks the app frontmost, activates the app, and raises the window again 60 ms later (activation brings the app's own key window up). Our layer (bar, workspaces, tool windows) is put away first, like Caps+S; the next Caps brings it back. |
| W07 | Settings → Tiling | **Window thumbnails** (on by default; shows *Allow Screen Recording* until granted) and **Windows in search** (on: the Windows card). Type `settings switcher` or `settings windows`. |

Tile anatomy: the thumbnail, top-aligned crop; the app icon as the chip in the bottom-left corner; the window title only on the selected tile (on a dark capsule). Without a thumbnail, the app icon is the tile and the title sits under it. Minimized windows and hidden apps' windows are dimmed. Selection is the bar's 2.5pt blue ring. Switcher tiles use 24pt corners in a 32pt glass panel; bar tiles 16pt inside the 24pt card clip.

## Chord choice

**Caps + Tab.** Tab is the switcher key everywhere on the Mac (⌘Tab apps, ⌥Tab the Browser's site groups), so Caps+Tab reads as "the same thing, for windows". It was unused in the Caps grammar, and it keeps the grammar's shape: a key down previews, Caps up commits, Esc cancels. Tab is the one chord whose own key-up doesn't commit, because stepping means pressing it repeatedly. Letters were not an option for the chord: every free letter is a future "Caps+letter opens X" slot, and the switcher needs all 26 as hints.

## Module

Part of **Tiling**, not its own module. It uses the same Accessibility permission and the same AX layer (`ForeignWindows` in `TileEngine.swift`, now with per-window `raise`), Tiling already owns windows (Caps+W, Caps+digit workspaces, the `windows` keyword), and the Settings grid is exactly 12 tiles in 4 columns: a 13th would leave three empty cells. Tiling off: no Windows card, Caps+Tab shows the "Tiling is off" toast. In Lite (it's a bar feature).

## How it works

- **List** (`EdgeWindows.swift`): `CGWindowListCopyWindowInfo` decides which windows exist and their order (on-screen front-to-back is most-recently-used order); minimized, hidden and other-Space windows follow by when we last saw them in front. Accessibility adds titles, minimized state and drops palettes/popovers (subrole not standard or dialog); `_AXUIElementGetWindow` (private, used by AltTab/Rectangle/yabai) maps AX windows to window ids. Screen Recording adds titles of windows on other Spaces. Only regular apps; our own processes are left out. AX calls carry a 150 ms messaging timeout so a hung app can't stall the list.
- **Freshness**: read at launch, re-read off the main thread when the bar opens (if older than 2 s), on every Caps press (if older than 0.5 s), and when the switcher opens; any app activation, hide, unhide or quit marks it stale. The bar re-runs its search only when the list actually changed. When a refresh lands while the switcher is open, the selected window stays selected by id; a quick tap that hasn't drawn yet re-picks "the window behind" from the fresh list.
- **Thumbnails** (`EdgeWindowThumbs.swift`): `SCScreenshotManager` with a desktop-independent window filter, longest edge 640 px, three captures at a time, SCK window listing cached 3 s. Cache rules (`WindowThumbCache`): a thumbnail always shows once captured; it's recaptured when older than 1.5 s while shown, or when the window changed size; one capture per window in flight; closed windows pruned; 80 entries, least recently shown evicted. Minimized windows aren't captured (the last picture stays). A failed capture isn't retried for 10 s. No Screen Recording, or the setting off: nothing is captured, icon tiles.
- **Keys**: Tab joins the Caps chord keys. While the switcher is open, `SpotlightController` registers the remaining letters, arrows, Return and Esc as Carbon hot keys (ids 200+) and drops them when it closes; chord letters already registered for the hold are routed to the switcher as hints.
- **Pure logic** (`EdgeWindowsCore.swift`): ranking, hints, the `WindowSwitchState` machine, the thumbnail cache rules and the mosaic layout, with no AppKit.

## Journal

| Action | When | Params |
| --- | --- | --- |
| `windows.focus` | ↵ / click on the Windows card | window_id, app (bundle id), state (visible/minimized/hidden/offscreen), position, count |
| `windows.switch` | Caps+Tab commit | window_id, app, state, via (tab/hint/click), steps, position, count |
| `windows.switch_cancel` | Esc / outside click | steps, count |

Window titles are not journaled; the bar's usual choice record includes the card text (app and title of the first three) like it does for tabs and files.

## Measured

`tools/window-switcher-test/run.sh` (offline; reads the real window list once, read-only):

- Ranking 120 windows per keystroke: mean 0.14 ms, p99 0.28 ms (file search is ~5 ms). Normalizing a window list happens once per refresh (~0.5 ms for 60 windows), not per keystroke.
- `CGWindowListCopyWindowInfo(.optionAll)`: ~40 ms on the first call in a process, ~2 ms warm; on-screen only ~0.5–5 ms. Always off the main thread except as part of the background refresh.
- 2,861 checks: ranking cases, hints, the state machine alone and against the real `CapsWorkspaceChord` (both release orders, Esc), cache invalidation, mosaic geometry for 1–40 windows at bar and panel sizes.

`tools/window-switcher-test/run-ui.sh [dir]` renders bar and switcher mosaics offscreen with synthetic window pictures and real app icons.

## Not yet confirmed (needs a live session)

- Raising across Spaces: windows on another Space aren't visible to AX, so only their app is activated (macOS then switches to a Space with that app's windows, not necessarily that window). Exact-window raise there would need private CGS/SkyLight calls.
- Hidden apps: whether AX lists a hidden app's windows (if not, they appear only with Screen Recording, by title).
- Activation from the non-activating bar panel under macOS 26's cooperative activation: `activate()` plus AX `kAXFrontmostAttribute` is expected to win; unverified on device.
- Thumbnail behavior for windows on other Spaces and the first Screen Recording grant (macOS applies it after a relaunch).
- The first-launch cost of the list on a busy Mac (logged as `windows refresh n=… ms` under `studio.surface.edge.perf`, category `windows`).
- Edge workspaces (Surface, Browser, terminal, tool windows) are not in the list; Caps+N/B/T and Caps+digit cover them.

Sources: [EdgeWindowsCore](../../outputs/option-edge-search/Option/UI/EdgeWindowsCore.swift), [EdgeWindows](../../outputs/option-edge-search/Option/UI/EdgeWindows.swift), [EdgeWindowThumbs](../../outputs/option-edge-search/Option/UI/EdgeWindowThumbs.swift), [EdgeWindowTiles](../../outputs/option-edge-search/Option/UI/EdgeWindowTiles.swift), [EdgeWindowSwitcher](../../outputs/option-edge-search/Option/UI/EdgeWindowSwitcher.swift), [Caps routing](../../outputs/option-edge-search/Option/UI/SpotlightController.swift), [Windows card](../../outputs/option-edge-search/Option/UI/Spotlight.swift), [ForeignWindows](../../outputs/option-edge-search/Option/UI/TileEngine.swift).
# Other apps' windows: snap keys, grid, auto-tiling

[Feature index](../../FEATURES.md) · [Window switcher](window-switcher.md) · [Layer and workspaces](layer.md) · [Shortcuts](shortcuts.md) · [Surface Lite](lite.md)

Added **October 18, 2026** (branch `night2/windows`). The Rectangle / Loop / AeroSpace territory from the [r/macapps scout](../scout/macapps-2026-10.md): any app's focused window moves from the keyboard, the Caps+W grid places it, and auto-tiling (per display; on for the main display until you choose) lays a display out by itself. Our own windows (Surface, Browser, terminal, tool windows) keep their dwindle tiler (`AutoTiler.swift`); everything here is for other apps.

## Snap keys (any app)

The window is the **focused window of the app in front**. Moves happen on key **down** and [fly there](#animation) (Settings → Tiling → Animate windows off, or Reduce Motion: one jump). Gaps are Settings → Tiling → Gaps (8pt default), to the screen edge and between halves.

| ID | Keys (Caps held) | What happens |
| --- | --- | --- |
| X01 | ← / → | Left / right half. Again: ⅔, then ⅓, then ½. The step comes from where the window is, so it survives relaunches and other tools. |
| X02 | ↑ / ↓ | Top / bottom half, same ½ → ⅔ → ⅓ cycle. |
| X03 | Hold one arrow, press another at right angles | That quarter (← then ↑ = top-left). Either order. |
| X04 | ← + → | Middle third, full height. |
| X05 | ↑ + ↓ | Full height, x and width kept. |
| X06 | Return | Maximize to the gapped visible area. Again: back to your own frame. |
| X07 | Shift + Return | Center, size kept (shrunk only if it doesn't fit). |
| X08 | Shift + arrow | To the display that way (nearest by center; ←/→ wrap around the row, ↑/↓ don't). A snapped window keeps its snap there; any other keeps its relative position and size. |
| X09 | Delete | Back to your own frame from before the first snap (the one Rectangle calls "restore"). |
| X10 | Esc | Undoes every move made during this Caps hold (the chord grammar's cancel). |

"Your own frame" is the last frame we didn't put the window in: snapping a snapped window doesn't overwrite it. Apps with a minimum size keep it; the window hugs the edge the target hugs (a too-narrow right half grows leftward) and stays on screen. Full-screen windows and palettes/panels are left alone. With Tiling off, the keys show the "Tiling is off" toast. While the window switcher is open, arrows and Return keep stepping and committing it.

## Grid (Caps+W, drag)

| ID | Entry | Behavior |
| --- | --- | --- |
| X11 | Caps + W with another app in front | The existing grid picker (`TileEngine`), now on the screen under the pointer, and moved through the AX thread below. Caps+Delete afterwards restores. |
| X12 | Drag another app's window by its title bar | The same grid fades in at the center of that screen (as for our own windows); paint cells by moving over it, drop to snap that exact window. Option-drop, or dropping off the grid, leaves it where you put it. Settings → Tiling → **Drag to grid** (on). Not on auto-tiled displays (there a drag swaps). |

A grid tile saved in an earlier session is only a record: placing a new tile that covers it no longer moves that app's focused window (it could be a different window by now).

## Auto-tiling

**On for the main display until you choose** (since October 18, branch `tile/resize`). While `edge.autotile.displays` has never been written, the menu-bar display tiles and the others don't; **Caps + Shift + W** (the display the front window is on; a toast names it) or Settings → Tiling → **Auto-tile** (Off / Main / All) writes your choice, and it sticks from then on (an empty list is "off", not "never chosen"). Main only, not all displays: a first launch rearranges one screen, not every one, and the other displays are one Caps+Shift+W away. Nothing observes any app until a display tiles and Accessibility is granted (no prompt at launch).

A first launch is undoable: it flies the current Space's ordinary windows on the main display into tiles (floats, dialogs, minimized, hidden and other-Space windows stay put), and **Caps + Shift + W off flies every tile back to where it was before the tiler first moved it** (kept in memory per window; a window you floated with Return keeps where you put it). Settings → Off leaves windows where they are.

| ID | Behavior |
| --- | --- |
| X13 | **Layouts** (Settings → Tiling → Layout): **Split** (dwindle: each window halves what's left along its longer side; slots under 160pt fall back to equal slices) or **Master** (first window 55% wide, the rest stacked). 8pt-rule gaps from the same Gaps setting. |
| X14 | A new window slots in after the focused one; a closed, minimized or hidden one leaves and the rest reflow. Only windows on the current Space count. Each display has its own order. |
| X15 | **Float**: dialogs, sheets, panels, palettes, fixed-size windows, windows under 320×220, full-screen windows, and apps on the float list (System Settings, Calculator, Archive Utility, Photo Booth, Screen Sharing, FaceTime, Stickies, Digital Color Meter, System Information, plus yours: Settings → Tiling → **Always float**). |
| X16 | Caps + arrow: focus the neighbour that way. Caps + Shift + arrow: swap with it; off the display's edge, the window goes to the display that way (tiling there if it's auto-tiled). |
| X17 | Caps + Return: float this window (centered at 65%) / tile it again. Caps + Shift + Return: this app always floats / tiles again (a toast says which; it also overrides the built-in float list, kept in `edge.autotile.tile`). On an auto-tiled display, floating windows still get the snap keys. |
| X18 | Drag a tiled window onto another tile: they trade places (onto another auto-tiled display: it joins there). Dropped anywhere else, it goes back to its slot. |
| X19 | **Resize by dragging a tile's edge** (Hyprland's split ratios): the divider under that edge follows your hand and the tiles on its other side follow **1:1, live** (set straight through their apps' writers on every change, no spring). Only the tiles beside that divider change: every deeper divider stays where it is on screen. A corner moves both dividers. On mouse-up the divider is kept (saved per display) and everything settles into its exact slot along the spring. Clamped: no tile under 160pt (`minimumSlot`) or its known minimum size; dragged past that, your window springs back to the limit. Edges on the display's border, and between stacked Master tiles, have no divider: they spring back. |
| X20 | **Caps + Option + arrow**: the focused tile's border in that direction moves one step (5% of the display's width or height) that way, along the spring; with no divider on that side, its other border moves that way instead (the arrow always says where a border goes). Presses in a row retarget the flight. **Caps + Option + Delete**: this display's dividers even again. Floating windows ignore both. |
| X21 | **Minimum sizes plan the layout.** A window's minimum (learned the first time it refuses a size, by the animator or a one-jump move) is honoured from the next reflow on: it gets at least that along each axis, its siblings shrink for it down to their own minimums (and never under 80pt, `tileFloor`, while a minimum squeezes them), and no tile goes past the display's visible frame. Learning one triggers a reflow at once. **When the minimums can't all fit** (two windows that each need 70% of the width side by side), the window asking the most along the axis that overflows (the later one on a tie) **floats, centered at its minimum** (pinned to the top-left if it's bigger than the display), and the rest tile without it; it tiles again when there's room. Chosen over re-splitting the other way: predictable, and it never reshuffles the windows that fit. |

## Workspaces (other apps' windows, opt-in)

Added **October 18, 2026** (branch `night2/spaces`). **Off by default:** Settings → Tiling → **Workspaces include other apps**. Off, Caps+1…9, 0 hold only Surface's own windows, as before.

| ID | Keys / event | What happens |
| --- | --- | --- |
| X19 | Caps + Shift + digit, another app in front | Its focused window belongs to that workspace now: a short toss that way (animated), then it parks. Focus goes to the frontmost window still here. To the current workspace: only reassigned. |
| X20 | Caps + digit | Every ordinary window on screen that isn't anyone's yet joins the workspace you're leaving (an empty workspace is a clear desk); the other workspaces' windows park; this one's come back to their exact frames, sliding in from the side the workspace is on (higher numbers from the right). The one you left focused there gets focus, unless our own windows open there. On auto-tiled displays the workspace's tiling comes back too: its order and its own splits (dividers you dragged there), kept per workspace while it's away. Parked windows never count as closed: they leave the layout without the safety net treating them as gone, and the tiles that stay fill the space they left. |
| X21 | Activating a parked window (Cmd-Tab, Dock, Caps+Tab) | Goes to its workspace, like pressing its Caps+digit. |
| X22 | Settings → **Bring N back** | Every parked window back on screen, assignments kept (shows while any are parked). |

Workspaces are global (one current workspace across displays). Panels, palettes, full-screen windows, windows on other macOS Spaces and windows an app keeps off screen on purpose are never touched. A relaunched app's windows start unassigned (windows are keyed by pid + CGWindowID).

**Hiding: parked just off a display corner** (AeroSpace's way), not minimized and not app-hidden. Minimize animates every window (~0.5 s, and while it animates the app stops answering AX), fills the Dock and can't be made instant; app hide is per app, so one app's windows couldn't live in two workspaces. Parking is one AX position write, no animation, nothing in the Dock. The spot: the window's own display's bottom-right, then bottom-left corner (full frame, then visible frame), then other displays' corners; a spot that would put any of the window on another display is never used, so parked windows don't peek onto a neighbour. macOS keeps a title bar's strip on screen (measured: y is clamped to 24pt above the display's bottom), so a parked window shows as a 1×24pt sliver at the corner. Each park is read back; a window that won't stay (an app that clamps its frames) is **minimized** instead, and goes straight to minimize after that; one that refuses both stays where it is. Animated work (minimize, un-minimize) runs after every instant move so a slow app can't hold up the rest.

**Never losing a window:**

- The state (`edge.spaces.v1`) is written **before** anything parks; until a restore has happened its window still counts as parked (`SpacesCore.writeAhead`), so a crash at any point of a switch still knows every window that's away.
- Every parked window comes back on **quit**, on **SIGTERM/SIGINT/SIGHUP** (restored, then the signal proceeds), from a **crash** handler (SIGSEGV/BUS/ILL/TRAP/ABRT/FPE: a best-effort restore from the handler with a 2s alarm, then the crash and its report proceed), when the **setting or Tiling** goes off (and assignments are forgotten), from **Bring back**, and at the **next launch whatever the setting says** (covers kill -9 and power loss).
- An app that doesn't answer keeps its records (and its restores are retried 5× over 1.5s); a parked window whose app quits becomes a **rescue** entry: when the app relaunches with a window still off screen (state restoration), it goes back to that home (3 days). A parked window that comes back by other means (the Dock, the app, a display change) belongs to the workspace you're in. A display change re-parks anything macOS moved into view; a home on a display that's gone lands centered on the nearest one.
- The `--test-instance` copy never touches other apps' windows (it shares the defaults domain).

**Fast:** one `CGWindowListCopyWindowInfo` per switch, AX only for windows it hasn't met plus one frame read per window (the window server's bounds trail an AX move by a few ms; reading them caused misassigned windows on quick Caps+1, Caps+2 in testing), all on the AX thread with its timeouts. No polling: activations, launches, quits and display changes are notifications.

Code: decisions in `ForeignSpacesCore.swift` (pure), AX in `ForeignSpaces.swift`, tiler hooks `spacesSwitch`/`spacesParked`/`spacesFly` in `ForeignWindowManager.swift`. Journal: `workspace.switch.apps` (number, windows), `workspace.send.app` (number, app).

## How it works

- **One AX thread** (`AXWorker` in `ForeignWindowManager.swift`): every Accessibility call on another app runs on a dedicated thread with its own run loop and 0.25s messaging timeouts (0.1s for the drag hit-test), so a hung app can't stall the main thread. App elements are cached; the auto-tiler caches window elements and their traits.
- **The landing** (and the whole move when animation is off): `AXEnhancedUserInterface` is switched off for the move (apps animate every frame change while it's on) and restored; position and size are each set only if they differ; across displays position goes first (so the size isn't clamped against the display being left), within one display size first; a clamped size is set once more; a minimum size is fitted (`WindowLayoutMath.fit`).
- **No polling at idle**: the auto-tiler learns from `AXObserver` notifications (window created, focused, (de)miniaturized, moved, resized, destroyed; app hidden/shown) plus workspace notifications (launch, quit, activate, Space change, display change), debounced 20–50ms into one reflow that re-reads only the apps that changed and moves only windows whose slot changed. Our own moves' echoes are ignored for 0.4s. Timers only while something is happening: during a drag of a tiled window, its frame ~120 times a second (60ms once it's clearly a move) until the mouse comes up; and the **removal safety net** below.
- **Windows leaving** (closed, minimized, hidden, quit, last tab closed): destroyed (per window and per app), miniaturized and app-hidden notifications reflow at once; a window that's still on screen for its close animation is looked at again 350ms later; un-minimize / show reflow again after their animation (450ms); a quitting app's windows leave every order immediately. **Safety net**: ~300ms after any app activation, focus change or reflow, and then at most every 2s until 10s after the last change, the tiled set is checked against `CGWindowListCopyWindowInfo` (on-screen windows) — a tiled window that's gone, or an on-screen tiling window that isn't tiled, triggers a reflow (`AutoTileOrder.gone`). Nothing runs at idle.
- **Dividers** (`AutoTileRatios` in `AutoTileOrder`): Split keeps one ratio per dwindle cut (the share of what's left that the window at that position takes; trailing even cuts implicit), Master one (the master column's share). Cuts are positional: a window leaving takes its cut with it and later cuts keep their windows; a new window gets an even cut of its own; a swap keeps the dividers (the windows trade sizes). Which way each cut goes is decided on the even layout, so dragging a divider never flips one. Saved per display in `edge.autotile.ratios`.
- **Which windows exist**: AX for the app's windows and traits, intersected with `CGWindowListCopyWindowInfo(.optionOnScreenOnly)` (drops minimized, hidden and other-Space windows). Tabbed windows count once (AX lists the selected tab).
- **Drag grid**: one global mouse monitor (down/dragged/up; no AX until a press). A press asks the AX thread which window is under it; once the pointer has moved 6pt, one frame read per drag event until the window has moved with its size unchanged (a title-bar drag) or six reads say it hasn't (a drag inside the content) or its size changed (a resize).
- **Pure logic** (`WindowLayoutCore.swift`): snap frames, cycling, arrow combinations, minimum-size fit, display direction and transfer, auto-tile layouts with dividers and minimums (`AutoTileLayout.geometry`), divider drags and nudges (`resized`, `nudged`, `moving`), order, neighbours, float rules, the gone-window check. No AppKit.

State: `edge.autotile.displays` (display UUIDs; never written = the main display), `edge.autotile.layout` (`split`/`master`), `edge.autotile.ratios` (per display UUID: `{split: [Double], master: Double}`), `edge.autotile.float` / `edge.autotile.tile` (bundle ids), `edge.tile.dragGrid`, `edge.tile.animate`, `edge.tile.motion` (`quick`/`snappy`/`smooth`). Restore frames, pre-tiling frames and learned minimum sizes live in memory only.

## Animation

Added **October 18, 2026** (branch `anim/engine`, from the `anim/probe` measurements). Every frame change on this page flies the **real window**: snaps, display hops, Return/Delete, the grid and the drag grid, auto-tile reflows, swaps, float/unfloat, Esc. Its AX position and size are set on every display-link frame along a spring, at the display's own rate (120 Hz on ProMotion). No snapshots, no stand-in layers, and the size moves all the way (it isn't snapped at the end). Our own windows keep their own motion (untouched).

| ID | Behavior |
| --- | --- |
| X19 | **Spring**: fast launch, slight overshoot. Settings → Tiling → **Feel**: Quick (response 0.24 s, damping 0.80), **Snappy** (0.30 s, 0.78: the probe's), Smooth (0.42 s, 0.86). Lands in about 0.37 / 0.45 / 0.62 s. Past its target an edge goes at most 12pt and never beyond the display's visible frame (menu bar, Dock side): a long flight gets just enough extra damping for that, a short one keeps the feel. |
| X20 | **Retargeting**: a command during a flight keeps the window's position and velocity. Spam ← ← ← chains ½ → ⅔ → ⅓ in one flight; commands think from where the window is heading, so cycles, Return-again and Delete work mid-flight. The same target again changes nothing. |
| X21 | **Together**: a reflow's windows, a swap's pair and an Esc's windows leave on the same display frame. |
| X22 | **Slow apps**: each app has its own writer, so a slow or hung app never delays another app's windows or the clock. An app whose resize can't keep up at the display's rate gets a new size every 2nd, 3rd… frame (at most every 6th) and a position every frame; a frame without a new size is placed so the edges that aren't travelling stay still. |
| X23 | **Landing**: the exact frame (the same order and minimum-size fitting as a jump), read back, retried once. A minimum size the app refuses to go under (two shrinking requests refused with the same size, more than a terminal cell short) or lands at is learned; that flight and later ones aim for the fitted frame instead of correcting at the end. |
| X24 | Settings → Tiling → **Animate windows** (on). Reduce Motion (`accessibilityDisplayShouldReduceMotion`): one jump. |

### How it works

- **Three kinds of thread.** The AX worker asks (`WindowAnimator.animate`, from `ForeignWindowManager.setFrame`). One clock thread runs a `CADisplayLink` (from `NSScreen.displayLink` on the fastest display, 80–120 Hz asked) and the springs, and never touches AX, so no app can make it late. One serial writer per app (pid) does that app's writes: a mailbox holds only the newest frame per window (latest wins: a writer still busy with frame N gets the newest one next, never a backlog; a size not yet sent isn't dropped with the frame that carried it). AX elements keep the 0.25 s messaging timeout.
- **Spring** (`WindowMotionCore.swift`, pure): x, y, width and height on the same spring, so every edge is itself that spring and an edge that shouldn't move doesn't (growing leftward keeps the right edge exactly). Semi-implicit Euler in ≤1 ms substeps at the frame's presentation time. The overshoot bound picks the least damping (feel → critical) whose closed-form swing fits each edge's allowance, then holds the state itself at that wall, so a retarget always starts from exactly what's on screen.
- **Write order per frame**: growing (or crossing displays) moves first, then sizes; shrinking sizes first. The landing uses the jump's order.
- **Size cadence**: each writer times its resizes and moves (moving average; a lone spike counts for at most 4× the average). Size goes every k-th frame for the smallest k where the app's windows fit 90% of a frame; windows of one app take turns.
- **Idle**: no flights, the link is paused and its watchdog timer gone: no wakeups. If no frame arrives for 150 ms (display asleep), everything in flight lands at once; a flight never lasts more than 1.5 s.
- **Echoes**: while an app's windows fly, its moved/resized notifications are ours and skipped before any AX lookup; a landing renews the auto-tiler's 0.4 s echo window. `AXEnhancedUserInterface` is off for the app from its first frame to its last landing.
### Measured (October 18, one 120 Hz display)

`tools/window-anim-test/run.sh` (offline, the real `WindowMotionCore.swift`): 24,084 checks. 3,888 flights (every feel, gaps 0/4/8/16, three display shapes, nine snap targets, random starts, ±10% frame jitter) settle in under 1.5 s and land within 1pt; no edge goes past its overshoot allowance or outside the visible frame. Settling: Quick median 367 ms (max 425), Snappy 450 ms (max 525), Smooth 617 ms (max 708). A 600pt mid-screen move overshoots 11pt. 400 spam runs (3–20 retargets each, 15–250 ms apart): position and velocity unchanged by every retarget, 0.00pt jump at a retarget, no frame moves an edge more than 0.3pt beyond what its velocity carries. Growing leftward keeps the right edge exactly; a same-size move never resizes. Cadence, the spike-proof average and minimum-size learning (a terminal's 14pt cells aren't a minimum).

`tools/window-anim-test/run-live.sh` (live, the real `WindowAnimator.swift` + `ForeignWindowManager.swift`, three throwaway apps: one plain, two whose every resize blocks 10 ms / 25 ms like a slow Electron relayout): 79 checks, all passing. "On beat" = the display frame's window frame was written before the next frame began; "dropped" = frames a busy app never got (a newer one replaced them).

| App | Flights | On beat (median / worst) | Clock on beat | Dropped / frames | Resize (median) | Size every | Sizes per flight | Landing error |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Plain | 12 | 100% / 77% | 100% | 1 / 682 | 0.56 ms | every frame | 46 | 0.00pt |
| 10 ms resizes | 8 | 48% / 26% | 100% | 48 / 441 | 12.1 ms | 2nd–3rd | 23 | 0.00pt |
| 25 ms resizes | 8 | 38% / 17% | 100% | 151 / 441 | 27.7 ms | up to 6th | 11 | 0.00pt |
| Plain, beside the 25 ms one in flight | 1 | 100% | 100% | 0 / 59 | 0.64 ms | every frame | 51 | 0.00pt |

Also live: spam ← ← ← 60 ms apart is one retargeted flight landing on ⅓ (worst edge step between 8 ms samples 74pt, the spring's own speed); Esc mid-flight flies back to where the hold began; animation turned off mid-flight lands at once and nothing moves after; a 60%-minimum-width window hugs the right edge and its next flight aims for the learned width (no retry); turning auto-tiling on flies 3 windows leaving within 0.22 ms of each other, a swap's two within 0.06 ms; idle, the link delivers no frames. Plain app: tick → write done median 0.8 ms. The slow apps' frames are late because the app itself is busy for 10–25 ms per resize; the clock and other apps don't notice. Flight median ~460 ms (Snappy). The probe's real-app costs for comparison: moves ≈0.05 ms everywhere, resizes 0.1–0.7 ms median for most apps, 8–13 ms p90 for Finder, Ghostty and Safari (≈ the 10 ms dummy here).

- Perf log per landing: `log show --predicate 'subsystem == "studio.surface.edge.perf" AND category == "windows"' --info` (`windows flight …`: frames, on time, late, dropped, size cadence, landing error).

## Chord choice

Arrows, Return and Delete are not letters: every free letter stays a future "Caps+letter opens X" slot, and the hints row (`EdgeFolderShelf`) only shows letters that open things, so these aren't in it. They're registered as Carbon hot keys only while Caps is held (chord indices 100+, ids 102+), like every chord. Shift on arrows means "further" (another display / swap), as Shift+digit sends a window to a workspace. Option on arrows means "resize" (as Option-drag resizes from the center in many apps), and Option+Delete "reset the resizing", as Delete alone is "restore"; arrows and Delete are already reserved for the layer (`EdgeChordMap.reserved`), so no letter or rebindable key is taken. Not Caps+Option+= or +0: = is a rebindable chord key and 0 is workspace 10 (digits are registered with every modifier). Caps+Shift+W sits on the tiling key. Esc during the hold doesn't undo resizes (they're layout, not window moves).

## Journal

| Action | When | Params |
| --- | --- | --- |
| `windows.snap` | Any snap key, grid placement | action (`left`, `left-1` for ⅔, `left-2` for ⅓, `topLeft`, `maximize`, `unmaximize`, `center`, `display`, `restore`, `grid`), app (bundle id) |
| `windows.autotile` | Caps+Shift+W | on |
| `windows.tile_resize` | A drag kept, Caps+Option+arrow, Caps+Option+Delete | action (`drag`, `key`, `equalize`) |

## Measured

`tools/autotile-test/run.sh` (offline, the real `WindowLayoutCore.swift`): 5,173 checks. Halves/thirds/quarters meet at exactly one gap for gaps 0/4/8/16 on three display shapes; the stateless cycle; every arrow pair; minimum-size fit for every action; display direction, wrap and transfer; split and master layouts for 1–16 windows on three displays and four gaps (inside the area, no overlap, every inner edge one gap from a neighbour, no voids); insert/remove/swap; neighbours with a way back; float rules. A reflow's math for 40 windows: ~10µs.

`tools/autotile-test/run-live.sh` (live, the real `ForeignWindowManager.swift` with small stubs, against a throwaway app's own windows; needs Accessibility for the shell running it; touches nothing else): 32 checks, all passing on October 18, including swap and per-app float, both animated (it waits for landings) and with `ANIMATE=0` (the jump path). Animated, command → within 1pt of the target: median 225ms (max 416ms), auto-tile first layout ~490ms, a new window ~540ms. Jump path: command → window there median 8ms (max 9ms) on a quiet machine, 14–20ms (max 30–77ms) under heavy load from other builds (2ms polling). Auto-tile: first layout 44–56ms, a new window reflowed in ~100–120ms (includes the 50ms debounce and the app creating it), a closed one in ~16–19ms.

**Resizing, minimums, removal** (October 18, `tile/resize`): offline 24,259 checks (adds divider drags in split and master, corner drags, deeper dividers staying put, clamping and no hidden overdrag, keyboard steps/limits/reset, ratio bookkeeping through insert/remove/swap and storage, minimum sizes: tall in a top/bottom split, wide in a left/right split, siblings' own minimums, master both ways, the impossible case floating centered, bigger than the display, a drag stopping at a neighbour's minimum, a fuzz of random ratios × random minimums × 1–8 windows × 4 gaps × 3 displays (inside, no overlap, every minimum kept, no voids), the gone-window check and no hole after any window leaves). Live 70 checks, all passing animated and with `ANIMATE=0`: tiles checked against the manager's planned slots (inside the area, no overlap, no hole); a simulated edge drag (harness stands in for the mouse button, AX-resizes the test window in 10 steps of 20pt): **the neighbour is there within median 1.4–1.7ms of each step (max 1.8–8.7ms)**, kept and saved on mouse-up, the other tile's left edge moves the same divider, dragged past the limit it clamps; Caps+Option+→ one step, three quick ← retargeted into one landing (~475ms), Caps+Option+Delete evens; a 75%-tall-minimum window gets its height and stays on the display; two 70%-wide minimums: one floats centered at its minimum, the rest tile with no hole. Removal: close button ~520ms to the reflow landing (the flight is ~460ms of that), minimize ~1.1s (macOS's own minimize animation first), hide, a polite quit ~480ms, an app killed with no notice reclaimed ~760ms after the next activation (safety net). Caps+Shift+W off flies a tile back to its opening frame. `tools/window-anim-test/run-live.sh` 79/79.

**Workspaces × auto-tiling** (October 18, `combo/today`): `tools/autotile-test/run.sh` section "workspaces: switch keeps each one's splits", 133 checks on `AutoTileOrder.arriving` (what `spacesSwitch` runs per display): a 1→2→1 round trip gives back each workspace's exact order, splits and frames; a parked middle window takes its cut along; a window that stays tiles after the arriving ones; a saved window parked by now stays out; every park mask for 1–6 windows leaves no overlap, no void and no parked window in the layout. Control: dropping parked ids without the ratio bookkeeping (the night2/spaces code before the merge) misaligns the dividers. Live, the tiling round trip in `tools/spaces-test/run-live.sh` was still skipped (the installed Surface Lite auto-tiles this Mac's main display).

`tools/spaces-test/run.sh` (offline, the real `ForeignSpacesCore.swift`): 213,077 checks. Park spots on 8 display arrangements and 3 sizes never touch another display; home frames when a display goes; switch/send plans; reconcile after closes, quits, pid reuse and AX timeouts; rescue; JSON round trip; a fuzz of 300 runs × 60 operations against a simulated window server (stubborn windows, relaunches, you dragging windows back, our quit, our crash part-way through a switch) checking that every window that's away always has a record that brings it back, and that restore-all puts each at exactly its frame. Breaking the write-ahead makes 327 of those fail. Plan time for 200 windows: 0.15 ms.

`tools/spaces-test/run-live.sh` (live, the real `ForeignSpaces.swift` + `ForeignWindowManager.swift` + animator against three throwaway apps; the app list is narrowed to them and nothing is activated): 33 checks animated, 32 with `ANIMATE=0`, all passing on October 18. Park 4 windows of 2 apps: median 6 ms (max 19) jumping, 14 ms (max 21) with the animator loaded; back home: 6 ms jumping, ~190 ms animated (the flight). Exact frames back, panels untouched, the stubborn window minimized and back, send, Bring back, switching off, and SIGTERM / a crash (trap) / SIGKILL + next launch each bring every window back. The tiling round trip is skipped while an installed Surface app is auto-tiling the Mac (it would tile the dummies too); it was not verified live tonight.

## Not yet confirmed (needs a live session in the installed build)

- Workspaces for other apps in the running app: the chords, the Settings row, follow-on-activation, focus after a switch, real apps (Electron, Chrome, Finder), two displays, and the auto-tiled round trip.

- The chords themselves in the running app (the test copy has no Caps), and the Settings rows on screen.
- The drag grid with a real title-bar drag; drop timing against the window server finishing the drag.
- Display hops on two displays (one display was connected when this was built; the math is tested offline).
- Auto-tiling across Space switches, native full-screen, and apps that resize themselves (Electron, Chrome). Minimum sizes now make room (X21), but only once learned: the first flight to a too-small slot still shows the overlap for that one landing, then the reflow fixes it. A minimum is never unlearned while the window lives (an app whose minimum shrinks keeps the bigger slot).
- Apps that don't answer AX notifications until activated: they're watched on their first activation.
- Edge drags with a real mouse in the installed build: the harness stands in for the mouse button and resizes the test window through AX step by step. Real live-resize event rates (and apps that only report the size at the end of a live resize) not yet seen.
- The Caps+Option chords in the running app.
- Animation in the installed build against real apps: the engine has only flown the throwaway test apps (the `anim/probe` numbers for VS Code, Ghostty, Discord, Safari and Finder were the probe's own loop, not this engine). Not yet seen: Electron's rare 80–90ms resize spikes mid-flight, Chrome/Electron minimum sizes, a terminal's cell grid at landing, flights across two displays (and through a gap between them), a 60Hz external display, the Settings rows on screen.
- Grabbing a window while it's in flight: the flight keeps writing until it lands (up to ~0.5s) and wins over the drag.

## Next steps

- A per-display layout choice; resizing between stacked Master tiles (per-window stack weights); keeping a dragged divider's tile at its exact size when a cut flips on a display change.
- Workspaces for other apps: per-display workspaces (AeroSpace-style), a bar list of what each workspace holds, follow-on-activation with a delay so a stray Cmd-Tab doesn't switch; a window parked by minimizing (the fallback for windows that won't stay in a corner) rejoins its workspace's layout at the end, not in its old slot (it isn't on screen until its un-minimize ends).

Sources: [WindowLayoutCore](../../outputs/option-edge-search/Option/UI/WindowLayoutCore.swift), [WindowMotionCore](../../outputs/option-edge-search/Option/UI/WindowMotionCore.swift), [WindowAnimator](../../outputs/option-edge-search/Option/UI/WindowAnimator.swift), [ForeignWindowManager](../../outputs/option-edge-search/Option/UI/ForeignWindowManager.swift), [chord routing](../../outputs/option-edge-search/Option/UI/SpotlightController.swift), [grid](../../outputs/option-edge-search/Option/UI/TileEngine.swift), [settings](../../outputs/option-edge-search/Option/UI/EdgeSettingsPages.swift), [harness](../../tools/autotile-test), [animation harness](../../tools/window-anim-test).
# Nardo feature reference

Last source audit: **September 30, 2026**. Start here when you forget what the app can do.

This catalog covers the current Surface executable, its embedded Nardo Search companion, and the related capabilities present in their source. It records implementation and entry points, not release readiness or successful end-to-end testing. Availability is based on `outputs/surface-edge/build.sh` and `outputs/option-edge-search/project.yml`; old prototypes and build backups under `work/` are not the app.

The audit records **402 feature, storage, availability and developer-tool entries**, plus a shortcut reference and a coverage map of **159 Swift source files**. Some availability entries describe alternate routes for capabilities listed elsewhere.

## Find a feature

| Area | Reference | Includes |
| --- | --- | --- |
| Desktop layer | [Layer and workspaces](docs/features/layer.md) | Caps Lock, pinned actions, command shelf, docking, floating, resizing, persisted layouts |
| Drawing and documents | [Surface](docs/features/surface.md) | Boards, designs, text, ink, images, notebooks, linked views, relationships, outlines, branching history, import/export |
| Search and quick tools | [Search and utilities](docs/features/search.md) | Apps, files, clipboard, web/tab/history search, explicit feature tags, calculator, conversions, colors, encodings, random tools |
| Experimental editing through language | [Workspace commands](docs/features/commands.md) | Removed from Spotlight; retained typed executor and native research adapters |
| Browsing | [Browser](docs/features/browser.md) | Spaces, tab trees, containers, split view, Glance, reader, find, downloads, media, passwords, extensions, protection |
| Companion capabilities | [Notes, Games and voice](docs/features/companions.md) | Option Notes handoff and editor, game discovery/launch/update/search, local parties, dictation |
| Privacy and storage | [Networking, data and recovery](docs/features/data-and-networking.md) | Free VPN, advanced HTTPS proxy, kill switch, local stores, permissions, restore paths |
| Hidden and developer features | [Availability and developer tools](docs/features/availability.md) | Unwired settings/import routes, optional runtimes, excluded Journal, experiments, verification, builds |
| Keyboard reference | [Shortcuts](docs/features/shortcuts.md) | Current defaults, context differences, gestures, command examples |
| Calendar | [Calendar](docs/features/calendar.md) | Week and day card, `add …` events, live countdowns, one-press meeting join (`join`), reminders row and `remind me … 5pm`, time-zone answers |
| Mail | [Mail](docs/features/mail.md) | Needs-you card, search, Caps+E, mail window and reader, archive/snooze/undo, Gmail sync |
| Windows | [Other apps' windows](docs/features/windows.md) | Caps+arrows snap any app's window (½ ⅔ ⅓ cycle, quarters, maximize/restore, center, display hops, Esc undoes); the Caps+W grid by dragging title bars; opt-in per-display auto-tiling (split / master, floats dialogs and small windows) |
| Window switcher | [Window switcher](docs/features/window-switcher.md) | Caps+Tab mosaic of every other app's window (Tab steps, letter hints, Caps up raises, Esc cancels); typing finds windows by title or app in the bar's Windows card; live thumbnails with Screen Recording |
| Dictation | [Dictation](docs/features/dictation.md) | Caps+D into the field you were in, in any app (A07: Accessibility, paste with the clipboard put back, or typed keys, per app); per-app tone; command mode not done |
| Held items | [Held items](docs/features/held.md) | Hold a file above the bar (⌘↑ or drop), then type verbs at it: `under 100mb` squeezes into a new version (original untouched, ⌘Z steps back), `mail susan` composes and sends through Gmail, `done` lets go |
| Text from images | [OCR](docs/features/ocr.md) | Caps+S then T (or `ocr` in the bar): drag over screen text, it lands on the clipboard; clipboard search matches text inside image clips, with the matched line on the card |
| Bottom strip | [Bottom strip](docs/features/strip.md) | Surface's dock-like resting row at the bottom edge: pinned apps (imported once from the Dock), running apps, Dock badges, live sliver (dictation, terminal ✓/✗, timers, track), workspace dots; window peeks, right-click card, drag to reorder / unpin / workspace, file drops; the Caps stack's floor; opt-in Replace Dock with exact restore |
| Notch island | [Notch island](docs/features/notch.md) | Dictation, a terminal waiting for you, now playing and Caps chords around the MacBook notch; hover or click opens them as cards whose buttons are the existing actions; Notch module |
| Menu search | [Menu search and shortcut cheat sheet](docs/features/menu-search.md) | The app in front's menu items searched from the bar and performed (AXPress), `keys` / Caps+/ cheat sheet of its shortcuts, `menus.perform` for any caller |
| File actions | [File actions](docs/features/file-actions.md) | `to png`, `resize 50%`, `to mp4`, `to m4a`, `pdf`, `zip`, `rename …` on held files, dropped files or Finder's selection; new files next to the source, never overwritten; progress and result tiles on the held card; ⌘Z steps back |
| Bar words | [Bar words](docs/features/bar-words.md) | `timer 25m` / `pomodoro` live tiles + notification, `mute`, `airpods` / `output` switch, `define`, `translate`, `color` pick, `:emoji`, `kill :3000` |
| Settings, keys, first run | [Settings, keys and first run](docs/features/settings.md) | Modules grid, Caps chord rebinding, permission asks in the bar, debug-hook gating, update plumbing |
| System words | [System words](docs/features/system-words.md) | `battery` + Bluetooth devices with batteries, `connect sony` / `disconnect airpods`, `cpu` / `memory` per app, `quit slack` / `force quit`, `eject` / `eject all` |
| Nardo Lite build | [Nardo Lite](docs/features/lite.md) | The bar-only first release: separate target, `Edition.lite` gates, what's in/out, rules for shared edits |
| What belongs | [Citizens and guests](docs/features/citizenship.md) | Which features compose with other layers vs. which become optional modules; the test for new ideas |
| Implementation lookup | [Source coverage map](docs/features/source-map.md) | Every first-party Swift source in the app targets, grouped by feature area |

## Things particularly easy to forget

- **Surface history is a tree.** Preview any moment, then branch there; the other futures remain saved.
- **Duplicating a placed surface creates another live view.** Duplicating drawn objects makes independent copies.
- **Images are editable canvases.** Double-click an imported image and annotate it; the image clips annotations in previews and exports.
- **Relationships are rules.** Movement, alignment, fill and outline sharing survive later edits; Option-drag changes a movement offset.
- **Outlines stack.** Each outline has its own width, color, enabled state, order and scaling behavior.
- **Covered boards have a peek gesture.** Hold Option over the board to temporarily see live contents.
- **Caps Lock + number commits when the number is released.** Release Caps first to cancel. Caps + 0 toggles the input alone.
- **The command field changes purpose.** Command-L edits the active Browser address; Command-T creates a tab; Option-T scopes it to the focused site.
- **Search contains offline developer tools and toys.** Try `sha256 hello`, `base64 hello`, `password 24`, `roll 2d6`, `days until 2026-12-31`, or `#ff6600 to hsl`.
- **Notes has two meanings.** Surface's own text notes live in `.surface` documents; `@notes` hands off to the separately installed Option Notes app.
- **Games is more than Steam.** The source discovers several libraries and launch formats, with external runtime requirements.
- **Free VPN is Tor today.** It carries TCP, blocks UDP/LAN traffic, and leaves its firewall guard armed on failure or quit. Disconnect restores ordinary networking.

## Availability labels

**Integrated** means a current Nardo entry point is wired. **External** requires a separate app/runtime/data source. **Source only** means implementation exists but no normal entry point was found in the current target. **Experimental** requires an explicit developer route. **Excluded / proposed** is not enabled by this build.

These distinctions are part of the feature reference. For example, a working `SettingsView` in the source does not mean this companion's Settings scene presents it. The [availability reference](docs/features/availability.md) records those gaps rather than silently treating them as shipped behavior.

## Keep this complete

Add a row to the relevant reference whenever you add or change a user-visible capability. Preserve its feature ID when renaming it. Record the entry point, behavior, dependencies/limits and source link; update shortcuts when routing changes. For removed or unwired features, update availability instead of deleting the only reminder that the capability exists.

Before a release, use these rows as the inventory for deciding what to expose, test, explain or remove. Re-audit menu items, command registries, keyboard handlers, settings, build flags and external handoffs; a source audit alone is not a release sign-off.
# Nardo

[**Feature reference — everything the app can do**](FEATURES.md)

**Two products build from this tree:** full Nardo and [**Nardo Lite**](docs/features/lite.md), the bar-only first release. Read the Lite doc before changing shared companion code.

- [Build, launch and project notes](outputs/surface-edge/README.md)
- [Workspace command details](outputs/surface-edge/WORKSPACE-COMMANDS.md)
- [Free VPN / secure proxy details](outputs/surface-edge/SECURE_PROXY.md)

Current application sources are in `outputs/surface-edge` and `outputs/option-edge-search`. `work/` contains development scripts, experiments, verification output and older copies.
