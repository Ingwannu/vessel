# Vessel native app

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

### Sidebar

- Session rows provide an action menu.
- Ended sessions are shown by default.
- The All projects group can be expanded.
- Sidebar spacing uses the shared interface tokens.
