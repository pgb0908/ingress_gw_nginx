# 배포 검증과 롤백 절차

data-plane은 revision의 버전 관리나 활성화 판단을 직접 하지 않는다.
배포와 롤백의 결정은 control plane의 책임이고, data-plane의 Admin API는 전달받은 config를 적용하는 역할만 한다.

## config 전달 방식

Admin API는 두 가지 방법으로 config를 수신한다.

### 방식 A: 디렉토리 로드

control plane이 revision 번들을 지정 디렉토리에 배치한 후 Admin API에 로드를 요청한다.

```
Control Plane
  1. revision 번들을 지정 디렉토리에 배치
  2. Admin API에 로드 요청

Admin API
  3. 디렉토리에서 번들 파일 읽기
  4. 설정 검증
  5. Nginx 설정 반영
```

### 방식 B: API push

control plane이 Admin API 엔드포인트로 config payload를 직접 전송한다.

```
Control Plane
  1. Admin API에 config payload 전송

Admin API
  2. 수신한 config 검증
  3. Nginx 설정 반영
```

## 배포 절차

### 1단계: config 전달

**방식 A (디렉토리):**
```bash
# control plane이 번들 배치 후 로드 요청
curl -X POST http://127.0.0.1:19080/admin/revisions/load \
  -H "Content-Type: application/json" \
  -d '{"path": "/path/to/revision-bundle"}'
```

**방식 B (API push):**
```bash
curl -X POST http://127.0.0.1:19080/admin/config \
  -H "Content-Type: application/json" \
  -d @revision-bundle.json
```

### 2단계: 적용 확인

```bash
curl http://127.0.0.1:19080/status
```

응답에서 확인할 항목:

```json
{
  "current_revision": "prod-2026-04-28-001",
  "last_reload_status": {
    "success": true,
    "message": "nginx reloaded successfully"
  }
}
```

`last_reload_status.success`가 `true`이면 배포 성공이다.

### 3단계: 메트릭 확인

```bash
curl http://127.0.0.1:19080/metrics
```

`gateway_reload_total`이 증가하고 `gateway_reload_failures_total`이 변하지 않으면 정상이다.

## 롤백 절차

롤백은 data-plane에서 직접 처리하지 않는다. control plane이 이전 revision의 config를 Admin API로 재전송하는 방식으로 수행한다.

```
Control Plane
  1. 롤백 결정 (어떤 revision으로 돌아갈지 선택)
  2. 해당 revision의 config를 Admin API로 전달 (배포와 동일한 절차)

Admin API
  3. 수신한 config 검증
  4. Nginx 설정 반영
```

롤백 후에도 동일하게 `/status`로 `current_revision`과 `last_reload_status`를 확인한다.

## 실패 시 대응

| 상황 | 확인 방법 | 대응 |
|------|----------|------|
| Nginx reload 실패 | `last_reload_status.success: false` | `message` 내용 확인 후 config 수정하여 재전달 |
| config 검증 실패 | API 응답의 `errors` 배열 | 오류 항목 수정 후 재전달 |
| plugin wasm 파일 누락 | API 응답의 `errors` 배열 | 번들에 wasm 파일 포함 후 재전달 |

## Admin API 엔드포인트

| 엔드포인트 | 설명 |
|-----------|------|
| `GET /status` | 현재 runtime state 조회 |
| `GET /metrics` | Prometheus 형식 메트릭 조회 |
| `GET /plugin-check` | plugin 호환성 검사 |
| `GET /admin/revisions/*` | revision 목록 및 상세 조회 |

## 관련 문서

- [리비전 구조](../02-architecture/revision-lifecycle.md)
- [운영 관측성](observability.md)
