Vessel {LABEL} — Windows 10 (1703 이후) / 11, x64

[한국어]
- 설치판: vessel-{LABEL}-windows-x64-setup.exe 를 실행합니다. 관리자 권한
  없이 사용자 폴더(%LOCALAPPDATA%\Programs\Vessel)에 설치되고, 시작 메뉴에
  "Vessel"이 생깁니다. 제거는 설정 > 앱 > 설치된 앱 > Vessel 에서 합니다.
- 설치 없이 쓰기: vessel-{LABEL}-windows-x64.zip 을 풀고 Vessel\Vessel.exe 를
  실행합니다.
- 코드 서명이 아직 없는 빌드라서 처음 실행할 때 Windows가 "Windows의 PC 보호"
  창을 띄울 수 있습니다. "추가 정보" > "실행"을 누릅니다.
- 처음 열면 머신이 없으므로 "머신 추가"(영문 Windows에서는 "Add a machine")
  화면이 나옵니다. 추가할 머신에서 `vessel node pair` 를 실행해 나온 링크를
  붙여 넣고 Enter 를 누릅니다(링크는 10분 동안 한 번만 쓸 수 있습니다).
  화면에 나온 키가 그 머신이 링크와 함께 출력한 키와 같은지 확인하고,
  미리 채워진 이름을 그대로 두거나 바꾼 뒤 "연결"을 누릅니다. 노드 목록과 기기 자격 증명은 vessel
  CLI와 같은 곳(%APPDATA%\vessel)에 둡니다.
- 창 위쪽 막대(제목 표시줄)는 앱이 직접 그립니다. 끌기·최대화·닫기가
  이상하면 Windows 기본 막대로 실행할 수 있습니다. PowerShell에서
  $env:VESSEL_APP_TITLEBAR="system"; & "$env:LOCALAPPDATA\Programs\Vessel\Vessel.exe"
  (zip으로 받았다면 풀어 둔 폴더의 Vessel.exe 경로를 씁니다. Alt+F4,
  Alt+Space, Win+화살표는 어느 쪽에서도 동작합니다.)
- {UPDATES_KO}

[English]
- Installer: run vessel-{LABEL}-windows-x64-setup.exe. No administrator
  rights; it installs under %LOCALAPPDATA%\Programs\Vessel and adds
  "Vessel" to the Start menu. Remove it from Settings > Apps > Installed
  apps > Vessel.
- Without installing: unzip vessel-{LABEL}-windows-x64.zip and run
  Vessel\Vessel.exe.
- This build is not code-signed yet: on first run Windows may show
  "Windows protected your PC"; choose "More info" > "Run anyway".
- With no machine yet, the app opens on "Add a machine" ("머신 추가" on a
  Korean Windows): run `vessel node pair` on the machine to add, paste the
  link it prints and press Enter (a link works once, for ten minutes).
  Check that the key shown is the key the machine printed with the link,
  keep or change the name filled in for it, and press "Pair". The node book and device credentials
  are the vessel CLI's (%APPDATA%\vessel).
- The title bar is the app's own. If dragging, maximising or closing
  misbehaves, run it with Windows' own bar instead, in PowerShell:
  $env:VESSEL_APP_TITLEBAR="system"; & "$env:LOCALAPPDATA\Programs\Vessel\Vessel.exe"
  (from the zip, use the path of the unpacked Vessel.exe. Alt+F4,
  Alt+Space and Win+arrows work either way.)
- {UPDATES_EN}

Vessel is source-available under PolyForm Strict 1.0.0 (LICENSE.md).
Embedded fonts: Geist and Geist Mono (Vercel), Pretendard; SIL Open Font
License 1.1, texts in licenses\.
