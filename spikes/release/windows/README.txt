Vessel {LABEL} — Windows 10 (1703 이후) / 11, x64

[한국어]
- 설치판: vessel-{LABEL}-windows-x64-setup.exe 를 실행합니다. 관리자 권한
  없이 사용자 폴더(%LOCALAPPDATA%\Programs\Vessel)에 설치되고, 시작 메뉴에
  "Vessel"이 생깁니다. 제거는 설정 > 앱 > 설치된 앱 > Vessel 에서 합니다.
- 설치 없이 쓰기: vessel-{LABEL}-windows-x64.zip 을 풀고 Vessel\Vessel.exe 를
  실행합니다.
- 코드 서명이 아직 없는 빌드라서 처음 실행할 때 Windows가 "Windows의 PC 보호"
  창을 띄울 수 있습니다. "추가 정보" > "실행"을 누릅니다.
- 처음 열면 기계가 없으므로 "Add a machine" 화면이 나옵니다. 추가할 기계에서
  `vessel node pair` 를 실행해 나온 링크를 그 칸에 붙여 넣고 Enter, 키가
  그 기계의 `vessel node status` 와 같은지 확인한 뒤 이름을 붙여 Pair 를
  누릅니다. 노드 목록과 기기 자격 증명은 vessel CLI와 같은 곳에 둡니다.
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
- With no machine yet, the app opens on "Add a machine": run
  `vessel node pair` on the machine to add, paste the link it prints into
  the field and press Enter, check the key against `vessel node status`
  there, name it and press Pair. The node book and device credentials are
  the vessel CLI's.
- {UPDATES_EN}

Vessel is source-available under PolyForm Strict 1.0.0 (LICENSE.md).
