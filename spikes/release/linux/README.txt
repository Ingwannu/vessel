Vessel {LABEL} — Linux x86-64 (glibc)

[한국어]
- 압축을 풀고 ./vessel-app 을 실행합니다. X11 또는 Wayland 데스크톱과
  Vulkan 드라이버(Mesa 것으로 충분)가 필요합니다.
- 처음 열면 "Add a machine" 화면이 나옵니다. 추가할 기계에서
  `vessel node pair` 를 실행해 나온 링크를 붙여 넣고 Enter, 키를 확인한 뒤
  이름을 붙여 Pair 를 누릅니다. 노드 목록과 기기 자격 증명은 vessel CLI와
  같은 곳(~/.config/vessel)에 둡니다.
- {UPDATES_KO}

[English]
- Unpack and run ./vessel-app. It needs an X11 or Wayland desktop and a
  Vulkan driver (Mesa's is enough).
- With no machine yet, the app opens on "Add a machine": paste the link
  `vessel node pair` prints on the machine to add, press Enter, check the
  key, name it and press Pair. The node book and device credentials are the
  vessel CLI's (~/.config/vessel).
- Environment: VESSEL_APP_THEME=light|dark pins the theme;
  VESSEL_CONFIG_DIR points at another configuration directory.
- {UPDATES_EN}

Vessel is source-available under PolyForm Strict 1.0.0 (LICENSE.md).
Embedded fonts: Geist and Geist Mono (Vercel), Pretendard; SIL Open Font
License 1.1, texts in licenses/.
