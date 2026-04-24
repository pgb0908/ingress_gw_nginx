# Local Data Plane

로컬 개발 우선 구조다. 목표는 툴체인과 프로젝트 전용 Nginx/Wasm 바이너리를 먼저 내려받고, Rust 기반 dataplane을 단일 명령으로 부팅하는 것이다.

## 빠른 시작

1. `bin/gateway-dev bootstrap`
2. `bin/gateway-dev doctor`
3. `bin/gateway-dev build`
4. `bin/gateway-dev up`
5. `curl -H 'X-Tenant-Id: tenant-a' -H 'X-Api-Key: tenant-a-dev-key' http://127.0.0.1:8080/api/orders`
6. 종료 시 `bin/gateway-dev down`

## 디렉토리 구조

### 소스와 설정

- `src/gatewayd/`
  - 실제 Rust dataplane 본체다.
  - `src/main.rs`: CLI 진입점
  - `src/server.rs`: admin API, plugin preflight
  - `src/runtime.rs`: revision validate, activate, rollback
  - `src/nginx.rs`: nginx.conf 생성과 Nginx 실행
  - `src/revision.rs`: revision 디렉토리 로드
  - `src/providers.rs`: 파일 기반 secret, rate limit provider
  - `src/bin/mock_upstream.rs`: 로컬 검증용 mock upstream 서버
- `src/runtime-config/`
  - control plane이 넘겨준다고 가정하는 정적 산출물 위치다.
  - `revisions/<revision>/` 아래에 revision별 설정이 들어간다.
  - `current`는 현재 활성 revision을 가리키는 심볼릭 링크다.
- `src/runtime-config/revisions/local-dev-001/`
  - 샘플 revision이다.
  - `gateway.json`, `listener.json`, `router-*.json`, `service-*.json`, `policy-*.json`, `plugin-chain.json`, `revision.json`, `data/`, `plugins/`를 포함한다.
- `src/scripts/`
  - `gateway-dev`가 내부에서 호출하는 스크립트 모음이다.
  - `bootstrap_dev_env.sh`: 개발환경 설치
  - `build_rust.sh`: Rust 워크스페이스 빌드
  - `dev_up.sh`, `dev_down.sh`, `status.sh`: 실행 제어
  - `common.sh`: 공통 환경 변수와 경로 정의

### 실행 중 생성물

- `src/gateway/runtime/`
  - dataplane 런타임이 실제로 사용하는 작업 디렉토리다.
  - `state.json`: 현재 revision, 이전 revision, reload 상태, 메트릭
  - `rate-limit-state.json`: 로컬 rate limit 상태
  - `logs/`: access, error, bootstrap 로그
  - `nginx/conf/nginx.conf`: 현재 활성 Nginx 설정
  - `nginx/generated/<revision>/nginx.conf`: revision별 생성된 Nginx 설정
  - `nginx/logs/nginx.pid`: 현재 실행 중인 Nginx PID

## `gateway-dev`와 생성물 관계

### `bin/gateway-dev bootstrap`

- 생성:
  - `../.local/`
- 역할:
  - Rust toolchain 설치
  - WasmX Nginx 다운로드
- 아직 dataplane 프로세스는 띄우지 않는다.

### `bin/gateway-dev build`

- 생성:
  - `../target/`
- 역할:
  - `src/gatewayd/` Rust 코드 빌드
  - 실행 파일 생성
    - `../target/debug/gatewayd`
    - `../target/debug/mock_upstream`

### `bin/gateway-dev up`

- 사용:
  - `../.local/wasmx/nginx`
  - `../target/debug/gatewayd`
  - `../target/debug/mock_upstream`
  - `src/runtime-config/revisions/local-dev-001/`
- 생성 또는 갱신:
  - `../.run/`
  - `src/runtime-config/current`
  - `src/gateway/runtime/`
- 역할:
  - admin 서버 기동
  - mock upstream 기동
  - revision 검증
  - nginx.conf 생성
  - active revision 전환
  - Nginx start 또는 reload

### `bin/gateway-dev status`

- 읽는 위치:
  - `../.run/*.pid`
  - `src/gateway/runtime/state.json`
- 역할:
  - 프로세스 상태 출력
  - 현재 revision과 메트릭 출력

### `bin/gateway-dev down`

- 역할:
  - admin 서버 종료
  - mock upstream 종료
  - Nginx quit
- 유지되는 것:
  - `../.local/`
  - `../target/`
  - `src/gateway/runtime/`

## 한 줄 요약

- `src/gatewayd`: 구현 코드
- `src/runtime-config`: 입력 설정
- `../.local`: 설치된 도구
- `../target`: 빌드 결과
- `../.run`: 실행 중 프로세스 정보
- `src/gateway/runtime`: 실행 결과물과 상태
