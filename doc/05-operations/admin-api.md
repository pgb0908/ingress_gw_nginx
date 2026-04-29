# API 레퍼런스

게이트웨이는 두 개의 HTTP 인터페이스를 노출한다.

| 인터페이스 | 포트 | 목적 |
|-----------|------|------|
| **Admin API** (`gatewayd`) | 19080 | 상태 조회, 메트릭, config 적용, 리소스 push |
| **Nginx 내부 위치** | 8080 (data-plane 포트) | Admin API를 외부에 투명하게 노출 |

Nginx 내부 위치는 별도 라우팅 없이 Admin API로 `proxy_pass` 된다.  
control plane이 config를 push할 때는 Admin API(19080)를 직접 호출한다.

---

## Admin API

### `GET /status`

현재 런타임 상태와 적용된 config 스냅샷을 반환한다.

**응답 `200 application/json`**

```json
{
  "current_revision": "local-dev-001",
  "current_revision_path": "/data/revisions/local-dev-001",
  "last_validation": {
    "revision": "local-dev-001",
    "valid": true,
    "errors": [],
    "warnings": []
  },
  "last_reload_status": {
    "success": true,
    "message": "ok"
  },
  "metrics": {
    "gateway_reload_total": 3,
    "gateway_reload_failures_total": 0,
    "gateway_requests_total": 1024,
    "gateway_request_duration_ms": 512,
    "gateway_plugin_executions_total": 5120,
    "gateway_plugin_failures_total": 0,
    "gateway_policy_denied_total": 12,
    "gateway_rate_limit_denied_total": 5
  },
  "config": {
    "revision": "local-dev-001",
    "created_at": "2026-04-24T10:30:00+09:00",
    "runtime_compat": "v1",
    "plugin_chain": ["tenant-filter", "auth-filter", "header-filter", "rate-limit-filter", "observe-filter"],
    "listener": {
      "protocol": "HTTP",
      "host": "0.0.0.0",
      "port": 8080,
      "allowed_hostnames": ["api.example.com"]
    },
    "routers": [
      {
        "name": "orders-route",
        "rules": [{ "path": "^/api/orders(/.*)?$", "methods": ["GET", "POST"] }],
        "destination": "orders-svc"
      }
    ],
    "services": {
      "orders-svc": {
        "targets": [{ "host": "10.0.0.10", "port": 8080, "weight": 100 }]
      }
    },
    "plugins": [
      {
        "name": "tenant-filter",
        "version": "1.0.0",
        "wasm_path": "plugins/tenant-filter.wasm",
        "sha256": "abc123",
        "failure_mode": "fail-close",
        "hooks": ["on_request_headers"]
      }
    ]
  }
}
```

**필드 설명**

| 필드 | 설명 |
|------|------|
| `current_revision` | 현재 적용된 revision 식별자 |
| `current_revision_path` | 번들 디렉토리 절대 경로 |
| `last_validation` | 마지막 validation 결과 (valid, errors, warnings) |
| `last_reload_status` | 마지막 nginx reload 성공 여부 |
| `metrics` | 누적 카운터 (프로세스 재시작 시 초기화) |
| `config` | 현재 번들에서 로드한 config 스냅샷. revision_path가 없거나 번들 로드에 실패하면 생략됨 |

```bash
curl http://127.0.0.1:19080/status
```

---

### `GET /metrics`

Prometheus exposition format으로 게이트웨이 메트릭을 반환한다.

**응답 `200 text/plain; version=0.0.4`**

```
# HELP gateway_reload_total Total nginx reload attempts
# TYPE gateway_reload_total counter
gateway_reload_total 3
# HELP gateway_reload_failures_total Total nginx reload failures
# TYPE gateway_reload_failures_total counter
gateway_reload_failures_total 0
# HELP gateway_requests_total Total number of requests processed
# TYPE gateway_requests_total counter
gateway_requests_total 1024
# HELP gateway_request_duration_ms Request duration in milliseconds
# TYPE gateway_request_duration_ms counter
gateway_request_duration_ms 512
# HELP gateway_plugin_executions_total Total plugin executions
# TYPE gateway_plugin_executions_total counter
gateway_plugin_executions_total 5120
# HELP gateway_plugin_failures_total Total plugin failures
# TYPE gateway_plugin_failures_total counter
gateway_plugin_failures_total 0
# HELP gateway_policy_denied_total Total requests denied by policy
# TYPE gateway_policy_denied_total counter
gateway_policy_denied_total 12
# HELP gateway_rate_limit_denied_total Total requests denied by rate limit
# TYPE gateway_rate_limit_denied_total counter
gateway_rate_limit_denied_total 5
```

```bash
curl http://127.0.0.1:19080/metrics
```

---

### `POST /deploy`

개별 리소스 JSON을 직접 push해 config를 업데이트한다.  
control plane이 Listener, Router, Service 등 리소스를 하나씩 전달할 때 사용한다.

수신한 리소스는 live 디렉토리(`runtime/dataplane/live/`)에 저장된다.  
번들이 완전해지면 자동으로 nginx config를 생성하고 reload한다.

**요청 `application/json`**

```json
{
  "apiVersion": "iip.gateway/v1alpha1",
  "kind": "Listener",
  "metadata": { "name": "main" },
  "spec": { "protocol": "HTTP", "port": 8080, "host": "0.0.0.0" }
}
```

지원 `kind`:

| `kind` | 저장 파일명 |
|--------|-------------|
| `Gateway` | `gateway.json` |
| `Listener` | `listener.json` |
| `Router` | `router-{name}.json` |
| `Service` | `service-{name}.json` |
| `Policy` | `policy-{name}.json` |

**응답**

| HTTP | `status` 값 | 의미 |
|------|-------------|------|
| 200 | `"applied"` | 번들 완전, nginx reload 성공 |
| 202 | `"staged"` | 저장됨, 번들 아직 불완전 |
| 400 | — | 잘못된 kind, 필드 누락, JSON 파싱 오류 |
| 500 | `"failed"` | 번들 유효하나 nginx reload 실패 |

```json
{
  "kind": "Listener",
  "name": "main",
  "status": "applied",
  "message": "ok",
  "validation": {
    "revision": "live-1745900000",
    "valid": true,
    "rendered_conf": "/runtime/dataplane/nginx/generated/live-1745900000/nginx.conf",
    "errors": [],
    "warnings": []
  }
}
```

**staged 예시** (필수 리소스 일부 누락)

```json
{
  "kind": "Listener",
  "name": "main",
  "status": "staged",
  "message": "resource saved; bundle not yet complete",
  "validation": {
    "revision": "unknown",
    "valid": false,
    "rendered_conf": null,
    "errors": ["missing router resource", "missing service resource"],
    "warnings": []
  }
}
```

**400 예시** (지원하지 않는 kind)

```json
{ "error": "unsupported kind: Unknown" }
```

**순차 배포 예시** — 리소스를 하나씩 push해 완전한 번들 구성:

```bash
curl -X POST http://127.0.0.1:19080/deploy \
  -H 'Content-Type: application/json' \
  -d '{"kind":"Gateway","metadata":{"name":"main"},"spec":{}}'

curl -X POST http://127.0.0.1:19080/deploy \
  -H 'Content-Type: application/json' \
  -d '{"kind":"Listener","metadata":{"name":"main"},"spec":{"protocol":"HTTP","port":8080,"host":"0.0.0.0"}}'

curl -X POST http://127.0.0.1:19080/deploy \
  -H 'Content-Type: application/json' \
  -d '{"kind":"Router","metadata":{"name":"main"},"spec":{"targetRef":{"kind":"Listener","name":"main"},"rules":[{"path":"^/.*$"}],"config":{"destinations":[{"destinationRef":{"kind":"Service","name":"backend"}}]}}}'

# 마지막 리소스 → 번들 완전 → 200 applied
curl -X POST http://127.0.0.1:19080/deploy \
  -H 'Content-Type: application/json' \
  -d '{"kind":"Service","metadata":{"name":"backend"},"spec":{"loadBalancing":{"targets":[{"host":"10.0.0.10","port":8080,"weight":100}]}}}'
```

> **live 디렉토리 초기화**: 첫 번째 deploy 요청 시 `runtime/dataplane/live/`가 없으면 자동 생성된다.  
> `revision.json`(`live-{unix_epoch_secs}`)과 기본 `plugin-chain.json`이 생성되고,  
> 현재 활성 revision의 `plugins/`, `data/` 디렉토리가 복사된다(wasm 바이너리 재사용).

---

### `POST /admin/revisions/load`

지정한 경로의 revision 번들을 읽어 nginx config를 생성하고 reload한다.  
control plane이 새 revision을 배포할 때 호출하는 주 엔드포인트이다.

**요청 `application/json`**

```json
{ "path": "/data/revisions/local-dev-002" }
```

| 필드 | 타입 | 설명 |
|------|------|------|
| `path` | string | revision 번들 디렉토리 절대 경로 |

**응답**

| HTTP | `status` 값 | 의미 |
|------|-------------|------|
| 200 | `"loaded"` | validation 통과, nginx reload 성공 |
| 400 | `"validation_failed"` | 번들 파일 누락 또는 nginx `-t` 검증 실패 |
| 400 | `"reload_failed"` | validation 통과 후 nginx reload 실패 |

```json
{
  "revision": "local-dev-002",
  "status": "loaded",
  "message": "ok",
  "validation": {
    "revision": "local-dev-002",
    "valid": true,
    "rendered_conf": "/runtime/dataplane/nginx/generated/local-dev-002/nginx.conf",
    "errors": [],
    "warnings": ["plugin rate-limit-filter uses pre-1.0 version 0.9.0"]
  }
}
```

**validation_failed 예시**

```json
{
  "revision": "local-dev-002",
  "status": "validation_failed",
  "message": "revision did not pass validation",
  "validation": {
    "revision": "local-dev-002",
    "valid": false,
    "rendered_conf": null,
    "errors": [
      "missing required file: revision.json",
      "missing router resource"
    ],
    "warnings": []
  }
}
```

```bash
curl -X POST http://127.0.0.1:19080/admin/revisions/load \
  -H 'Content-Type: application/json' \
  -d '{"path": "/data/revisions/local-dev-002"}'
```

---

### `POST /admin/config`

`POST /admin/revisions/load`의 별칭이다. 동일하게 동작한다.

```bash
curl -X POST http://127.0.0.1:19080/admin/config \
  -H 'Content-Type: application/json' \
  -d '{"path": "/data/revisions/local-dev-002"}'
```

---

## Nginx 내부 위치

data-plane nginx(8080)는 아래 두 경로를 Admin API로 투명하게 프록시한다.  
외부 모니터링 시스템이 별도 포트 없이 data-plane 포트 하나로 상태를 조회할 때 사용한다.

| 경로 | 프록시 대상 | 설명 |
|------|------------|------|
| `GET /metrics` | `http://127.0.0.1:19080/metrics` | Prometheus 스크레이프 엔드포인트 |
| `GET /__gateway_status` | `http://127.0.0.1:19080/status` | 게이트웨이 상태 및 config 스냅샷 |

> `/metrics` 경로는 `gateway.json`의 `spec.metrics.path` 값으로 변경할 수 있다.  
> 기본값은 `/metrics`이다.

```bash
# data-plane 포트를 통한 조회
curl http://localhost:8080/__gateway_status
curl http://localhost:8080/metrics
```

---

## 에러 응답

라우팅 실패(경로 미정의) 시 Admin API는 아래를 반환한다.

```json
{ "error": "not found" }
```

요청 본문 파싱 실패 시 500 에러와 함께 프로세스 stderr에 오류가 기록된다.

---

## 관련 문서

- [배포 및 롤백 절차](deployment-and-rollback.md)
- [옵저버빌리티](observability.md)
- [데이터 플레인 아키텍처](../02-architecture/dataplane-architecture.md)
