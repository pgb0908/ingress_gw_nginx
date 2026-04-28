# 데이터 플레인 아키텍처

## 책임 경계

이 저장소는 ingress-gateway의 데이터 플레인 구현만 한다. \
control plane은 별도 시스템으로 존재, \
이 저장소는 ingress-gateway의 admin을 통해 특정 디렉토리 위치 파일 혹은 api를 통해 config을 가져와 적용한다. 

데이터 플레인이 담당하는 일:

- config 로드 후 부팅
  - admin을 통해 특정 위치 파일 시스템의 config 로드
  - 혹은 admin api를 통해 
- Listener, Router, Service, Policy 리소스 로드
- Nginx 설정 반영과 reload
- Wasm plugin chain 실행
- 요청 허용, 차단, 헤더 표준화, trace propagation
- rate limit 및 secret 같은 외부 의존 시스템 조회
- 로그, 메트릭, 트레이싱 기록
- 실패 시 fallback 및 rollback 실행

```text
┌───────────────────────────────────────────────────────────┐
│                    ingress-gw Process                     │
│                                                           │
│  ┌──────────────────────┐          ┌────────────────────┐  │
│  │      ngx_wasm_module │          │     Admin API      │  │
│  │    (nginx + WASM)    │          │    (also nginx)    │  │
│  │                      │          │                    │  │
│  │   :18000 / :18443    │          │  :18001 / :18444   │  │
│  └──────────┬───────────┘          └─────────┬──────────┘  │
│             │                                │             │
│             └───────────────┬────────────────┘             │
│                             │                              │
│              ┌──────────────▼──────────────┐               │
│              │      WASM Plugin System     │               │
│              └──────────────┬──────────────┘               │
│                             │                              │
│              ┌──────────────▼──────────────┐               │
│              │        현재 적용된 Config     │               │
│              │  (Admin API로 수신한 최신     │               │
│              │   revision 번들 내용)        │               │
│              └─────────────────────────────┘               │
└─────────────────────────────────────────────────────────────┘
                              ▲
                              │ config 전달
                              │ (디렉토리 로드 또는 API push)
               ┌──────────────┴──────────────┐
               │        Control Plane        │
               │  (revision 버전 관리,        │
               │   활성화/롤백 결정)           │
               └─────────────────────────────┘
```

## 요청 처리 흐름

```text
Client
  -> Listener
  -> Gateway 전역 정책
  -> Router 매칭
  -> Plugin Chain 실행
  -> Service 선택 및 upstream proxy
  -> Access Log / Metrics / Tracing 기록
```



## 관련 문서

- [프로젝트 개요](../01-overview/project-overview.md)
- [리비전 배포와 롤백](revision-lifecycle.md)
- [플러그인 런타임 계약](../03-runtime-contracts/plugin-runtime-contract.md)
- [설정 리소스 모델](../04-config-models/README.md)
