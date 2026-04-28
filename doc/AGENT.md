# ingress-gateway 개발 문서

이 디렉토리는 ingress-gateway 데이터 플레인 구현을 위한 작업 문서 모음이다.
설명용 개요가 아니라, 개발 에이전트가 구현과 테스트를 바로 시작할 수 있도록 문서를 역할별로 분리했다.

## 먼저 읽을 문서

1. [프로젝트 개요](01-overview/project-overview.md)
2. [데이터 플레인 아키텍처](02-architecture/dataplane-architecture.md)
3. [리비전 배포와 롤백](02-architecture/revision-lifecycle.md)
4. [플러그인 런타임 계약](03-runtime-contracts/plugin-runtime-contract.md)
5. [설정 리소스 모델](04-config-models/README.md)
6. [운영 관측성](05-operations/observability.md)
7. [배포 검증과 롤백 절차](05-operations/deployment-and-rollback.md)

## 디렉토리 구성

- [01-overview](01-overview/project-overview.md): 목표, 구현 범위, 핵심 원칙, 용어
- [02-architecture](02-architecture/dataplane-architecture.md): 데이터 플레인 책임, control plane 경계, revision 흐름
- [03-runtime-contracts](03-runtime-contracts/README.md): 플러그인 계약, 공통 헤더, 실패 정책, 기본 플러그인 목록
- [04-config-models](04-config-models/README.md): Gateway, Listener, Router, Service, Policy 리소스 스펙
- [05-operations](05-operations/observability.md): 관측성, 배포 검증, reload, rollback 운영 절차

## 개발 에이전트 기준

- 구현 시작 전에는 `01-overview`와 `02-architecture`를 읽고 책임 경계를 먼저 고정한다.
- 플러그인 또는 Nginx 연동 구현 전에는 `03-runtime-contracts`를 기준 계약으로 삼는다.
- 리소스 파싱, 검증, 렌더링은 `04-config-models`를 기준으로 구현한다.
- 로그, 메트릭, 트레이싱, reload/rollback 처리는 `05-operations`를 acceptance 기준으로 사용한다.
