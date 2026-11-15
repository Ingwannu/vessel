# Vessel native app

## 0.0.0.0.21

메뉴를 다시 열거나 대화상자 위에 메뉴를 겹쳐 연 뒤에도, 앞서 열렸던 메뉴로 보낸 키가 새 메뉴를 바꾸지 않습니다. 대화상자 안에서 연 메뉴를 Esc로 닫으면 그 메뉴만 닫히고 대화상자는 그대로 남습니다.
대화상자를 닫으면 포커스가 그 대화상자를 연 자리로 돌아갑니다.
창 크기를 바꾸면 대화 옆 개요 레일이 바로 따라옵니다(좁아지면 숨고 다시 넓히면 나타남).
업데이트 다운로드가 끊기면 다음 시도에서 받은 데까지 확인한 뒤 나머지만 받습니다. 업데이트 파일은 압축본으로 받습니다(Windows 실행 파일 기준 약 52MB가 20MB 남짓). 두 가지 모두 이 빌드가 다음 빌드를 받을 때부터 적용됩니다. 0.0.0.0.20 이하에서 이 빌드로 올 때는 예전처럼 받습니다.
알려진 문제: 이번 확인(Linux, 실제 노드에 릴레이로 연결)에서 새로 찾은 문제는 없습니다.
화면으로 확인하지 못한 것: 대화상자 위에서 연 메뉴의 Esc 동작과 대화상자를 닫을 때 포커스 복귀(자동 시험으로만 확인), Windows 빌드 실행, 끊긴 업데이트의 이어받기와 압축본 받기(Linux에서 자동 시험으로만 확인), Windows에서의 이어받기.

Menus: keys sent to a menu that was already replaced no longer change the newer menu, also when a menu opens above a dialog. Esc in such a menu closes only the menu; the dialog stays.
Closing a dialog returns focus to what opened it.
The outline rail beside a conversation follows window resizes (it hides when narrow and comes back when wide again).
When an update download breaks off, the next attempt checks what arrived and fetches only the rest. Updates are downloaded as a compressed copy (the Windows executable's 52 MB becomes about 20 MB). Both apply from the next update this build downloads; coming to this build from 0.0.0.0.20 or earlier works as before.
Known problems: none new found in this check (Linux, connected to a real node over the relay).
Not checked on screen: Esc in a menu opened above a dialog and focus returning when a dialog closes (automated tests only), running the Windows build, resuming a broken-off update and downloading the compressed copy (shown by automated tests on Linux only), resuming on Windows.

## 0.0.0.0.20

Ctrl+Alt+T로 테마를 차례로 바꿀 수 있습니다.
입력창이나 터미널에 글자를 입력하는 동안(AltGr, 한글 조합 포함)에는 단축키가 그 입력을 가로채지 않습니다.
앱 언어가 시스템의 선호 언어 목록을 순서대로 따릅니다(앞의 언어를 지원하지 않으면 다음 언어).
창 폭이 918보다 좁으면 대화의 개요 막대가 숨겨지고, 창 폭이 바뀌면 바로 다시 그려집니다(0.0.0.0.19에서 800×600일 때 대화 옆에 남던 '=' 표시).
컨텍스트 고리의 12시 자리에 밝은 점이 더 이상 생기지 않습니다(진행 중처럼 보이던 것).
색을 입힌 테마에서도 터미널은 어두운 터미널 색을 씁니다.
알려진 문제: 설정의 연필 버튼이 옆 요소와 겹쳐 보일 수 있습니다(밝은 테마와 어두운 테마 모두). 터미널 창의 폭을 바꾸면 접혀 있던 긴 줄이 다시 맞춰지지 않을 수 있습니다.
화면으로 확인하지 못한 것: 선호 언어를 여러 개 둔 시스템에서의 언어 선택; 데스크톱이 Ctrl+Alt+T를 먼저 가져가는 경우(Ubuntu에서는 터미널 열기); 첨부의 파일 대화상자, 붙여넣은 파일 목록, 끌어 놓기; 터미널의 실제 터치, 하드웨어 GPU, 실제 전송 실패; 좁은 사이드바(폭 200)의 말줄임과 툴팁; 실제 GitHub와 Windows에서의 PR 화면.

Ctrl+Alt+T cycles through the themes.
While you type in a text field or a terminal (AltGr and composed input such as Hangul included), shortcuts no longer take that input.
The app's language follows the system's preferred languages in order (the next one when an earlier one is not available).
The chat outline rail hides when the window is narrower than 918 and is redrawn as soon as the width changes (the '=' left beside the conversation at 800×600 in 0.0.0.0.19).
The context ring no longer shows a bright point at twelve o'clock (it looked like progress).
In a tinted theme the terminal keeps its dark terminal colours.
Known problems: the pencil button in Settings can overlap the element beside it (light and dark themes). After a terminal's width changes, long lines it had wrapped may not be rejoined.
Not checked on screen: the language choice on a system with several preferred languages; a desktop that takes Ctrl+Alt+T first (Ubuntu opens a terminal with it); the attachment file dialog, the pasted file list, and drag and drop; the terminal with real touch input, a hardware GPU, and a real send failure; the narrow sidebar (width 200) with its ellipsis and tooltips; the PR view against real GitHub and on Windows.

## 0.0.0.0.19

사이드바가 세로로 스크롤되어, 창이 작거나 프로젝트가 많아도 아래쪽(세션·머신·머신 추가·설정)이 보입니다(0.0.0.0.18의 알려진 문제 해결).
터미널 화면이 생겼습니다: 터미널을 읽기 전용으로 보여 주고, 색과 굵게·기울임을 그립니다(코드 글꼴 Geist Mono의 굵게·기울임 추가).
설정에서 연필 버튼으로 머신 이름을 바꿀 수 있고, 긴 이름도 받습니다.
짝지은 기기의 상세가 마우스를 올리지 않아도 보이고, 불러오는 동안 한 줄로 알려 줍니다.
PR 화면에서 리뷰 판정이 한 번만 나오고, 초안 PR에는 병합 준비가 나오지 않습니다.
정보, 업데이트, 되감기, 포크, 설정의 진단·채팅이 번역됐습니다.
입력창의 컨텍스트 고리가 아직 값이 없을 때 닫힌 원으로 그려집니다(0.0.0.0.18에서 보내기 버튼 옆에 계속 도는 로딩 표시처럼 보이던 것).
알려진 문제: 설정의 연필 버튼이 옆 요소와 겹쳐 보일 수 있습니다(밝은 테마와 어두운 테마 모두). 터미널 창의 폭을 바꾸면 접혀 있던 긴 줄이 다시 맞춰지지 않을 수 있습니다.
화면으로 확인하지 못한 것: 첨부의 파일 대화상자, 붙여넣은 파일 목록, 끌어 놓기; 터미널의 실제 터치, 하드웨어 GPU, 실제 전송 실패; 좁은 사이드바(폭 200)의 말줄임과 툴팁; 실제 GitHub와 Windows에서의 PR 화면.

The sidebar scrolls vertically, so its bottom (sessions, machines, Add machine, Settings) stays reachable in a small window or with many projects (fixes the known problem from 0.0.0.0.18).
There is a terminal screen: it shows a terminal read only, with colours, bold and italic (the code face Geist Mono gains its bold and italics).
In Settings, the pencil button renames a machine, and long names fit.
A paired device's details show without hovering, with one line while they load.
The PR view shows the review verdict once, and a draft PR shows no merge readiness.
About, updates, rewind, fork, and Settings' Diagnostics and Chat are translated.
The composer's context ring is drawn as a closed circle while it has no value yet (in 0.0.0.0.18 it looked like a loading mark spinning beside the Send button).
Known problems: the pencil button in Settings can overlap the element beside it (light and dark themes). After a terminal's width changes, long lines it had wrapped may not be rejoined.
Not checked on screen: the attachment file dialog, the pasted file list, and drag and drop; the terminal with real touch input, a hardware GPU, and a real send failure; the narrow sidebar (width 200) with its ellipsis and tooltips; the PR view against real GitHub and on Windows.

## 0.0.0.0.18

사이드바: 세션 행 메뉴(읽음으로 표시, 세션 고정, 세션 보관)가 생기고, 끝난 세션이 기본으로 보이며(0.0.0.0.17의 알려진 문제 해결), All projects를 펼칠 수 있습니다.
갈라 만든 세션이 원래 대화를 불러오는 동안 줄이 보이고, 갈라 온 대화가 비어 있으면 알려 줍니다.
창이 좁아 코드 패널이 대화를 덮으면 대화가 어두워지고, 누르면 대화로 돌아옵니다.
설정에 연결 화면이 생겼습니다.
입력창에 파일을 첨부할 수 있습니다.
PR 화면에서 PR을 병합할 수 있고, 피드백 스위치와 기준 브랜치 선택이 생겼습니다.
정보 화면이 번역되고, 의존성마다 역할이 나옵니다.
알려진 문제: 창이 작거나(예: 800×600) 프로젝트가 많으면 사이드바 아래쪽(세션·머신·머신 추가·설정)이 화면 밖으로 밀리고 스크롤되지 않습니다. 'All projects'를 접으면 일부 돌아옵니다. 다음 빌드에서 고칩니다.
화면으로 확인하지 못한 것: 첨부의 파일 대화상자, 붙여넣은 파일 목록, 끌어 놓기; 이름 편집 연필 버튼의 실제 누르기; 실제 GitHub와 Windows에서의 PR 화면.

Sidebar: a session row menu (Mark as read, Pin session, Archive session); ended sessions are shown by default (fixes the known problem from 0.0.0.0.17); All projects can be unfolded.
A forked session shows a line while it loads the original conversation, and tells you when the forked conversation is empty.
In a narrow window, when the code panel covers the conversation, the conversation is dimmed; press it to return to the conversation.
Settings has a Connections page.
You can attach files in the composer.
The PR view can merge a PR, and has a feedback switch and a base branch choice.
About is translated and shows each dependency's role.
Known problem: in a small window (for example 800×600) or with many projects, the bottom of the sidebar (sessions, machines, Add machine, Settings) is pushed off screen and does not scroll. Folding 'All projects' brings part of it back. The next build fixes this.
Not checked on screen: the attachment file dialog, the pasted file list, and drag and drop; actually pressing the rename pencil button; the PR view against real GitHub and on Windows.

## 0.0.0.0.17

대화를 앞선 메시지로 되감을 수 있고, 대화를 새 세션으로 갈라 앞선 턴을 그대로 이어 볼 수 있습니다.
사이드바를 묶고 정렬할 수 있으며, 보기 메뉴가 생겼습니다.
설정에 채팅, 진단, 에이전트(쉬는 에이전트 해제 스위치)가 생기고, 머신 개요에서 이름을 바꾸거나 머신을 뺄 수 있으며, 정보 화면에 연결된 머신이 나옵니다.
파일 편집기가 생겼고, 디스크에서 파일이 바뀌면 알려 줍니다.
창이 좁으면 오른쪽 패널이 대화 위에 겹쳐 열려서, 0.0.0.0.16의 알려진 문제(대화 칸이 눌림)가 해결됐습니다.
세션이 도는 동안과 커서가 깜박이는 동안 CPU를 덜 씁니다.
알려진 문제: 끝난(중단되거나 종료된) 세션이 사이드바에서 기본으로 숨겨집니다. 머신 줄 오른쪽의 보기 메뉴에서 "끝난 세션 표시"를 켜면 보입니다. 다음 빌드에서 기본값을 켜짐으로 바꿉니다.

You can rewind a conversation to an earlier message, and fork a conversation into a new session that shows the earlier turns.
The sidebar can group and sort sessions, and it has a Display menu.
Settings has Chat, Diagnostics and Agents (with the idle-release switch); a machine's Overview lets you rename or remove it, and About lists the connected machines.
There is a file editor, and it tells you when a file changed on disk.
In a narrow window the right panel opens over the conversation, which fixes the known problem from 0.0.0.0.16 (the squeezed conversation column).
Less CPU while a session runs and while the caret blinks.
Known problem: ended (interrupted or exited) sessions are hidden in the sidebar by default. Turn on "Show ended sessions" in the Display menu at the right of the machine row. The next build shows them by default.

## 0.0.0.0.16

사이드바와 코드 패널의 가장자리를 끌어 폭을 바꿀 수 있습니다.
Gemini는 Google의 공식 마크로 나오고, 사이드바의 에이전트 마크는 흐린 색으로 나옵니다.
설정이 Paseo처럼 앱과 머신 두 묶음으로 나뉘고 머신을 고를 수 있으며, 단축키 페이지가 생겼습니다.
보내기의 기본값이 Steer입니다.
작성창이 초안을 보존하고 지난 메시지를 다시 불러오며, 새 세션에서도 / 메뉴가 열립니다.
git 메뉴(커밋, 푸시, 풀, 브랜치, 임시 보관)와 PR 열기 항목이 생겼습니다.
실패한 턴은 한 줄 경고로 보입니다.
새 세션과 세션 화면에 큰 V와 별이 보이고, 창 버튼이 새로 그려졌습니다.
설정 › 모양에 애니메이션 스위치가 생겼습니다(이전 이름 "움직임").
알려진 문제: 창 폭이 800쯤으로 좁을 때 오른쪽 패널을 열면 대화 칸이 너무 좁아져 글자가 이상하게 줄바꿈됩니다. 패널을 닫으면 돌아오고, 다음 빌드에서 고칩니다.

Drag the edge of the sidebar or the code panel to change its width.
Gemini shows Google's official mark, and the agents' marks in the sidebar are muted.
Settings follow Paseo's layout, in two groups (app and machine) with a machine picker, and there is a Shortcuts page.
Send defaults to Steer.
The composer keeps your draft and recalls earlier messages, and the / menu opens in a new session too.
There is a git menu (commit, push, pull, branch, stash) and an item to open a pull request.
A failed turn shows as a one-line warning.
New-session and session screens show the large V and star, and the window buttons are redrawn.
Settings › Appearance has an Animations switch (formerly "Motion").
Known problem: in a narrow window (about 800 wide) with the right panel open, the conversation column gets too narrow and text wraps badly. Closing the panel restores it; the next build fixes it.

## 0.0.0.0.15

창이 작을 때 업데이트 안내와 아래쪽 줄이 잘리던 것을 고쳤습니다. 안내가 길면 다섯 줄쯤에서 스크롤됩니다.
effort 슬라이더를 끄는 동안 손잡이가 포인터를 그대로 따라가고, 놓으면 가까운 칸에 붙습니다.
Fast 효과와 슬라이더가 초당 30프레임 고정이 아니라 화면 주사율에 맞춰 그려집니다.
움직임을 멈춘 화면에서는 입력창 커서가 깜박이지 않아 CPU를 쓰지 않습니다.

The update notice and the bottom line no longer get cut off when the window is small; long notes scroll after about five lines.
While you drag the effort slider, the thumb follows the pointer and settles on the nearest stop when you let go.
The Fast effect and the slider are drawn at the screen's refresh rate instead of a fixed 30 frames a second.
With motion held, the composer's caret stays still and uses no CPU.

## 0.0.0.0.14

effort 슬라이더의 채움이 알약 모양 안에 들어가서, 맨 아래 칸에서도 모서리가 각지지 않습니다.
카드, 메뉴, 모델 목록, 알림 위에 포인터를 올리면 그 아래 화면이 반응하지 않습니다.
카드의 움직임은 초당 30프레임으로 그리고, 포인터가 떠나면 몇 초 뒤 멈춰서 CPU를 덜 씁니다.
GPU 없이 그리는 환경에서는 앱 전체의 움직임을 멈춥니다. 바뀐 것 없는 상태 읽기로는 창을 다시 그리지 않습니다.
알려진 문제: 입력창에 포커스가 있으면 커서가 깜박일 때마다 창 전체를 다시 그립니다. GPU 없이 그리는 환경에서는 CPU를 많이 씁니다(1280×800에서 약 31%, 1920×1200에서 약 62%). 고칠 예정입니다.

The effort slider's fill stays inside its pill, so its end is round even at the lowest stop.
With the pointer over a card, a menu, the model list or a toast, the screen beneath no longer reacts.
The card animates at 30 frames a second and comes to rest a few seconds after the pointer leaves, using less CPU.
Without a GPU the whole app holds its motion still, and an unchanged status read no longer redraws the window.
Known: with the composer focused, each blink of the caret repaints the whole window. Without a GPU that costs a lot of CPU (about 31% at 1280×800, about 62% at 1920×1200). A fix is planned.

## 0.0.0.0.13

모델 카드와 effort 슬라이더를 Codex 앱처럼 새로 만들었습니다. 멈춤 칸 사이를 미끄러지고 튕기며, 맨 위 칸에서는 경고가 빛납니다.
에이전트 마크를 화면의 실제 픽셀 크기로 그려서 125%와 150% 배율에서도 선명합니다. Gemini는 Google의 Gemini 스파크로 바뀌었습니다.
코드 패널에서 검색하고, 고른 줄을 채팅으로 보내고, 파일 트리에서 새 파일·폴더, 이름 바꾸기, 삭제, 경로 복사를 할 수 있습니다.
끝난 세션에는 안내 카드가 뜹니다. 지금 보고 있는 세션은 읽음으로 표시되고, 찾기는 답한 질문의 글자에서도 찾습니다.
설정 › 기기의 줄이 한 줄로 촘촘해졌고, 페어링 카드의 ID와 지문이 코드 글꼴로 나옵니다.
설정 › 동기화는 다음 빌드에서 돌아옵니다. 앱이 노드 프로토콜 15를 씁니다.

The model card and the effort slider are rebuilt after the Codex app: they glide and spring between stops, and the warning shimmers at the top stop.
Agents' marks are drawn at the screen's own pixels, sharp at 125% and 150%. Gemini now shows Google's Gemini spark.
The code panel searches, sends chosen lines to the chat, and its file tree makes files and folders, renames, deletes and copies paths.
An ended session shows a card saying so. The session in front of you is marked read, and Find also matches the text of answered questions.
Rows in Settings › Devices are one line each, and the pairing card shows the id and fingerprint in the code face.
Settings › Sync comes back in the next build. The app speaks node protocol 15.

## 0.0.0.0.12

설정에 동기화 화면이 생겼습니다: 함께 쓰는 파일이 경로를 제목으로, 종류와 크기를 설명으로 나오고, 머신들의 동기화 상태를 볼 수 있습니다.
설정 › 기기와 동기화의 줄이 더 촘촘해졌고, 페어링 카드의 ID와 지문이 코드 글꼴로 나옵니다.
지금 보고 있는 세션은 읽음으로 표시됩니다.
찾기가 이미 답한 질문의 글자에서도 찾습니다.
앱이 노드 프로토콜 15를 씁니다.

Settings has a Sync page: shared files show the path as the title with kind and size beneath, and you can see the machines' sync status.
Rows in Settings › Devices and Sync are more compact, and the pairing card shows the id and fingerprint in the code face.
The session in front of you is marked read.
Find also matches the text of answered questions.
The app speaks node protocol 15.

## 0.0.0.0.11

설정 › 에이전트가 에이전트마다 준비 상태, 설치, 로그인 단계, 기본값을 보여 줍니다. 실패는 앱 문장으로 나오고, 원문은 툴팁에 있습니다.
대화의 컨텍스트 사용량이 링과 카드로 보입니다(여유, 주의, 가득).
화면 머리글, 버튼, 알림, 상태 표시가 Paseo 모양으로 맞춰졌습니다.
로그 폴더와 로그 파일을 본인만 읽을 수 있게 만듭니다.
앱이 노드 프로토콜 14를 씁니다(대화 되감기와 갈래 만들기 준비, 아직 화면 없음).

Settings › Agents shows each agent's readiness, install, sign-in steps and defaults. A failure reads as the app's sentence, with the original in a tooltip.
A conversation's context use shows as a ring and a card (room, amber, red).
Screen headers, buttons, alerts and status badges follow Paseo's look.
The log folder and the log file are readable by you only.
The app speaks node protocol 14 (rewinding and forking a conversation are prepared, not shown yet).

## 0.0.0.0.10

에이전트마다 공식 로고가 붙었습니다: 모델 목록, 입력칸의 버튼, 사이드바 줄, 보드 카드, 정보 화면의 출처 표시.
언어 선택과 페어링 문구가 번역 파일에서 나옵니다.
에이전트의 계획이 작업 줄로 그려집니다.
대기 중인 메시지를 키보드로 옮길 수 있습니다.
예전 노드(프로토콜 11)와도 그대로 페어링하고 세션을 엽니다.

Each agent shows its official mark: in the model list, the composer's button, the sidebar rows, the board's cards and About's attributions.
The language chooser and the pairing texts come from the translation files.
An agent's plan is drawn as task rows.
Queued messages can be moved with the keyboard.
It still pairs and opens sessions with an older node (protocol 11).

## 0.0.0.0.9

앱이 새 노드 프로토콜(13)을 씁니다. 세션 검색, 대기, 머신 시간대, 일정 미리보기가 준비됐지만 아직 화면에는 보이지 않습니다. 예전 노드와도 그대로 연결됩니다.
떠난 세션에서 늦게 온 답이 지금 보고 있는 세션을 바꾸지 않습니다.
머신 연결 확인 창의 이름 칸이 비어 있고, 비워 두면 머신 자신의 이름을 씁니다.
정보: 설치할 업데이트가 없으면 업데이트 버튼이 꺼져 있습니다.
화면 제목에 가는 글꼴(Geist Light, Pretendard Light)을 씁니다. Linux에서 동작 줄이기가 데스크톱 설정을 따릅니다.
이 빌드는 예전 버전 노드(프로토콜 11)와 페어링·연결·세션 열기가 되는 것을 확인했습니다. 새 노드 기능(세션 검색, 쉬는 에이전트 내려놓기, 일정 미리보기)은 노드를 올린 뒤에 쓸 수 있고, 아직 화면이 없습니다.
확인하지 못한 것: 실제 노드에서의 권한 카드(실제 에이전트에 프롬프트를 보내지 않음), Windows 빌드 실행, Windows 한글 입력.
알려진 것: 설정 › 기기의 행이 세 줄이고 모든 행에 '페어링됨'이 붙습니다. 정보의 번역 라이브러리 설명이 '데이터 읽고 쓰기'입니다.

The app speaks the new node protocol (13). Session search, parking, a machine's time zone and schedule previews are ready but not shown yet. It still connects to older nodes.
A late answer from a session you have left no longer changes the session you are looking at.
The pairing confirmation's name field starts empty; left empty, the machine's own name is used.
About: the update button is off when there is nothing to install.
Screen titles use light faces (Geist Light, Pretendard Light). On Linux, reduce motion follows the desktop's setting.
This build was checked to pair, connect and open sessions with an older node (protocol 11). The new node features (session search, parking idle agents, schedule previews) work once the node is upgraded, and have no screens yet.
Not checked: the permission card on a real node (no prompt is sent to a real agent), running the Windows build, Korean input on Windows.
Known: in Settings › Devices each row takes three lines and every row says "paired". In About, the translation libraries are described as "data reading and writing".

## 0.0.0.0.8

입력칸의 모델과 노력 단계가 버튼 하나로 합쳐졌습니다. 누르면 노력 슬라이더(Claude Code는 Ultracode까지, Codex는 Ultra까지), 번개 모양의 빠른 모드, 모델 목록을 여는 모델 이름이 나옵니다. 따로 있던 "Agent" 버튼은 없어지고, 그런 옵션은 모델 목록 맨 위에 쉬운 이름으로 있습니다.
보드가 생겼습니다(네 칸, 카드, 보관, 확인, 알림).
세션 옆 코드 패널(변경 사항과 파일별 차이, 파일 목록과 보기)과 사이드바의 프로젝트가 생겼습니다.
세션의 대기열, 카드로 묻는 에이전트의 질문, 사용자의 언어로 된 권한 버튼이 생겼습니다.
권한 카드를 Paseo의 카드처럼 다시 그렸습니다(제목, 명령, "어떻게 진행할까요?", 답 버튼, 여러 개가 기다리면 1/N).
설정이 실제로 적용됩니다: 보내기 단축키, 글자 크기, 내용 너비, 답하는 중에 Enter가 하는 일. 설정에 기기 항목이 생겼습니다.
사이드바 토글이 제목 표시줄에 항상 있습니다.
세션 중 연결이 끊기면 한 번만 알려 줍니다.
Linux X11에서 창이 다시 포커스를 받아도 한글 입력 상태가 유지됩니다.
파일 오류가 파일 내용을 보여 주지 않고, 설정 파일과 노드 목록은 본인만 읽을 수 있게 저장됩니다.
고친 것: 긴 출력 아래에서도 대화 상자의 버튼이 보입니다. 접힌 줄의 한글이 시스템 글꼴로 바뀌지 않습니다.
Windows에서 한글 조합 중 Enter로 보냈을 때 마지막 글자가 들어가는지 확인 부탁

The composer's model and effort are one button. It opens a card with an effort slider (Claude Code up to Ultracode, Codex up to Ultra), fast mode as the lightning, and the model's name, which opens the model list. The separate "Agent" button is gone; those options are at the top of the model list under plain names.
The board: four lanes, cards, archive, confirmations and notices.
A code panel beside a session (changes with each file's diff, files and a viewer), and projects in the sidebar.
A session's queue, an agent's questions as a card, and permission buttons in your language.
The permission card is redrawn as Paseo's (title, command, "How should it proceed?", answer buttons, 1/N when several wait).
Settings now take effect: the send shortcut, text sizes, content width, and what Enter does while a turn runs. Settings has a Devices page.
The sidebar toggle is always in the title bar.
A dropped connection during a session is told once.
On Linux X11, Korean input keeps its state when the window gets focus back.
A file error no longer shows the file's contents, and the settings file and the node list are saved readable by you only.
Fixes: a dialog's buttons stay in view under long output; Korean text in folded rows no longer falls back to the system font.
On Windows, please check that the last Korean syllable is sent when you press Enter while still composing it.

## 0.0.0.0.7

입력칸 아래 버튼에 기능 이름과 툴팁이 붙었습니다.
설정 화면이 생겼습니다(일반, 모양, 정보). 테마를 바로 바꿀 수 있습니다.
머신 추가가 두 단계 창으로 바뀌었습니다.
선택 목록이 버튼 쪽으로 열리고, 긴 글자는 자르지 않습니다.
대화: 내 메시지는 오른쪽 말풍선, 코드·표·출력이 상자 안에서 스크롤됩니다. 제목을 눌러 바로 고칠 수 있습니다.
정보: 업데이트 전 확인, 새로운 소식, 로그 경로 복사.

## 0.0.0.0.6

선택 목록의 긴 설명이 겹치지 않고 한 줄로 나옵니다.
기능(Fast mode 등)은 항목마다 따로 열립니다.
정보 화면을 설정 배치로 바꾸고 자동 업데이트 스위치와 로그 열기를 넣었습니다.
업데이트 실패 시 다시 시도하고 로그를 남깁니다.

## 0.0.0.0.5

끝났거나 중단된 세션에 글을 보내면 그 대화로 다시 시작합니다.
설치되지 않은 에이전트도 설치 방법과 함께 목록에 나옵니다.

## 0.0.0.0.4

창의 최소화·최대화·닫기 버튼이 한 번 클릭으로 동작합니다.
입력창이 카드 자체가 되었습니다.
모델 선택에 모든 에이전트가 나옵니다.
고친 로고를 씁니다.
