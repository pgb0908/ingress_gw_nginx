# 용어집

## 핵심 용어

- `dataplane`: 실제 요청을 수신하고 라우팅하며 정책을 집행하는 실행 계층
- `control plane`: 설정 생성, 검증, 배포 승인, revision 관리 책임을 가진 외부 시스템
- `revision`: 특정 시점의 Nginx 설정, Wasm 모듈, 참조 메타데이터를 묶은 배포 단위
- `gateway policy deploy`: 서비스 재배포 없이 gateway 설정과 플러그인 조합만 변경하는 릴리스
- `plugin chain`: 요청 처리 중 순서대로 실행되는 Wasm 플러그인 목록
- `fallback`: 외부 연동 실패 또는 정책 실패 시 적용하는 명시적 대체 동작

## 문서 내 표기 규칙

- 문서 루트는 `doc/`로 통일한다. `docs/` 표기는 사용하지 않는다.
- 플러그인 표기는 `plugin`으로 통일한다. `plug-in` 표기는 사용하지 않는다.
- `route`는 정책 적용과 트래픽 매칭의 논리 단위를 뜻하고, 구체 리소스 이름은 `Router`를 사용한다.
- `reload`는 새 revision 반영을 위한 Nginx 재적용 절차를 뜻한다.
- `rollback`은 이전 정상 revision으로 되돌리는 절차를 뜻한다.
