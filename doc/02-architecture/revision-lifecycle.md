# 리비전 구조

revision은 특정 시점의 Nginx 설정, Wasm 모듈, 참조 메타데이터를 묶은 config 단위다.
revision의 버전 관리, 활성화 결정, 롤백 판단은 control plane의 책임이다.
data-plane은 Admin API를 통해 현재 적용할 config를 전달받고, 그것만 처리한다.

## 번들 구조

revision 번들은 하나의 디렉토리다.

```
<revision-id>/
├── revision.json         # 필수: revision 메타데이터 및 plugin 선언
├── gateway.json          # 필수: Gateway 전역 설정
├── listener.json         # 필수: Listener 설정
├── plugin-chain.json     # 필수: plugin 실행 순서 정의
├── router-<name>.json    # 필수 (1개 이상): Router 리소스
├── service-<name>.json   # 필수 (1개 이상): Service 리소스
├── policy-<name>.json    # 선택: Policy 리소스 (보안, 트래픽 등)
└── plugins/
    └── <name>.wasm       # revision.json에 선언된 wasm 모듈 파일
```

## revision.json 구조

```json
{
  "revision": "prod-2026-04-28-001",
  "created_at": "2026-04-28T10:00:00+09:00",
  "runtime_compat": "python-local-v1",
  "plugins": [
    {
      "name": "auth-filter",
      "version": "1.2.0",
      "wasm_path": "plugins/auth-filter.wasm",
      "sha256": "abc123...",
      "failure_mode": "fail-close",
      "hooks": ["on_request_headers"]
    }
  ]
}
```

필드 설명:

| 필드 | 필수 | 설명 |
|------|------|------|
| `revision` | O | 고유 revision ID |
| `created_at` | O | ISO 8601 타임스탬프 |
| `runtime_compat` | O | 런타임 호환성 식별자 |
| `plugins[].name` | O | plugin 이름 (plugin-catalog에 정의된 이름) |
| `plugins[].version` | O | semver 버전 |
| `plugins[].wasm_path` | O | 번들 루트 기준 상대 경로 |
| `plugins[].sha256` | O | wasm 파일 무결성 검증용 해시 |
| `plugins[].failure_mode` | O | `fail-close`, `fail-open`, `fail-static-response`, `configurable` |
| `plugins[].hooks` | O | 이 plugin이 실행될 lifecycle 훅 목록 |

## config 전달 방식

data-plane은 두 가지 방법으로 config를 받는다. 어느 방식이든 data-plane은 전달받은 config를 현재 상태로 적용하기만 한다.

### 방식 A: 디렉토리 로드

Admin API가 지정된 디렉토리 위치에서 revision 번들을 읽는다.

```
Control Plane
  → revision 번들을 지정 디렉토리에 배치
  → Admin API에 로드 요청

Admin API
  → 디렉토리에서 번들 파일들을 읽음
  → Nginx 설정 반영
```

### 방식 B: API push

Control Plane이 Admin API 엔드포인트로 config를 직접 전송한다.

```
Control Plane
  → Admin API에 config payload 전송

Admin API
  → 수신한 config를 적용
  → Nginx 설정 반영
```

## 책임 경계

| 역할 | 담당 |
|------|------|
| revision ID 부여 및 버전 관리 | Control Plane |
| 어떤 revision을 활성화할지 결정 | Control Plane |
| 롤백 대상 revision 선택 | Control Plane |
| 전달받은 config를 Nginx에 반영 | Data Plane (Admin API) |
| 현재 config 기준으로 트래픽 처리 | Data Plane (nginx + WASM) |

## 관련 문서

- [데이터 플레인 아키텍처](dataplane-architecture.md)
- [플러그인 런타임 계약](../03-runtime-contracts/plugin-runtime-contract.md)
- [배포 검증과 롤백 절차](../05-operations/deployment-and-rollback.md)
