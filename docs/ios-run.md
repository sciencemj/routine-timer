# iOS / iPadOS 실행 가이드 (자기 기기용)

Routine Timer의 iOS/iPadOS 빌드를 시뮬레이터·실기기에서 실행하는 절차. App Store 배포가 아닌 **본인 기기 실행**용이다.

## 이미 되어 있는 것 (이 저장소 / 이 맥)

- `tauri ios init` 완료 → Xcode 프로젝트 `src-tauri/gen/apple/`가 커밋되어 있다.
- 이 맥에는 CocoaPods, iOS Rust 타겟(`aarch64-apple-ios`, `aarch64-apple-ios-sim`, `x86_64-apple-ios`)이 설치됨.
- Rust는 iOS 타겟으로 깨끗이 컴파일된다 (`cd src-tauri && cargo check --target aarch64-apple-ios` = 0 errors / 0 warnings).

**다른 맥에서 처음 세팅한다면** 먼저:

```bash
brew install cocoapods
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
```

(Xcode도 필요 — 전체 Xcode.app, Command Line Tools만으로는 부족.)

## 지금 막혀 있는 것: iOS 플랫폼 런타임

`tauri ios dev` / `tauri ios build`가 `iOS platform not installed`로 멈춘다. 원인은 코드가 아니라 **iOS 시뮬레이터 런타임(Xcode 컴포넌트)이 미설치**라서다 (`xcrun simctl list runtimes`가 비어 있음 — 최신 Xcode는 런타임을 기본 포함하지 않고 별도 다운로드한다).

한 번만 설치하면 된다 (수 GB, 몇 분~십수 분):

```bash
xcodebuild -downloadPlatform iOS
```

또는 Xcode ▸ Settings ▸ Components 에서 iOS 런타임 다운로드. 끝나면 확인:

```bash
xcrun simctl list runtimes | grep iOS   # iOS 26.x 가 뜨면 준비 완료
```

## 시뮬레이터에서 실행

```bash
bun run tauri ios dev
```

시뮬레이터가 뜨고 앱이 로드된다. 첫 실행 시 알림 권한 프롬프트 → **허용**.

## 실기기(아이폰/아이패드)에서 실행 — 무료 서명

1. Xcode로 프로젝트 열기: `open src-tauri/gen/apple/tauri-app.xcodeproj`
2. `tauri-app_iOS` 타겟 ▸ **Signing & Capabilities** ▸ Team = 본인 Apple ID(개인 팀). Bundle Identifier는 `com.minjun.dailyroutinetimer` (충돌 시 살짝 변경).
3. 아이폰을 케이블로 연결(첫 연결 시 기기에서 "이 컴퓨터를 신뢰" + 개발자 모드 켜기).
4. Xcode 상단에서 기기 선택 후 **Run** (▶). 또는 CLI:

```bash
bun run tauri ios dev            # 연결된 기기/시뮬 중 선택
# 또는 릴리즈 설치본:
bun run tauri ios build
```

무료(개인) 서명 앱은 **7일**마다 재설치가 필요하다. 기기에서 처음 실행 시 설정 ▸ 일반 ▸ VPN 및 기기 관리에서 개발자 앱을 신뢰해야 열린다.

## 검증 체크리스트 (수동 — 시뮬/실기기)

백그라운드 정확성이 이 포트의 핵심이므로 반드시 확인:

- [ ] 포모도로 루틴 시작(빠른 확인용으로 focus/break를 짧게 만들거나 25/5 그대로).
- [ ] 홈으로 나가 **백그라운드** 전환(시뮬: Device ▸ Home). focus 종료 시각에 "집중 완료", break 종료 시각에 "휴식 끝" 알림이 뜨는지.
- [ ] 앱으로 **복귀** 시 남은 시간이 실제 경과를 반영하는지(백그라운드 동안 줄어든 만큼 보정).
- [ ] 여러 phase를 백그라운드로 넘겨도 각 경계마다 알림이 오는지(체인 예약).
- [ ] iPhone·iPad 각각에서 상단바가 노치와 안 겹치고 좌측 traffic-light 여백이 없는지(safe-area).
- [ ] 원형 타이머·탭바가 화면 폭에 맞는지.

## 알려진 한계 (설계상 수용)

- 예약 알림 체인 상한 48개 / 24시간: 초장기 세션이나 `목표=0`(무제한) 루틴은 이후 경계 알림이 누락될 수 있음.
- 복귀 보정 gap은 24시간으로 클램프.
- App Store 미배포: 무료 서명은 7일마다 재설치.
- 메뉴바 카운트다운·팝오버·afplay 알람은 macOS 전용(iOS엔 해당 없음); iOS 알람은 예약 로컬 알림이 담당.

## 문제 해결

- `iOS platform not installed` → 위 "런타임 다운로드" 안 함. `xcrun simctl list runtimes`로 확인.
- 알림이 안 옴 → 첫 실행 권한 프롬프트를 거부했을 수 있음. 설정 ▸ 알림 ▸ Routine Timer에서 허용, 앱 재실행.
- 서명 오류 → Xcode에서 Team 미선택. Signing & Capabilities에서 개인 팀 지정.
