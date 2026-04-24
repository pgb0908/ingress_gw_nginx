# 리비전 배포와 롤백

## 리비전 단위

데이터 플레인은 설정 파일과 Wasm 모듈을 revision 단위 디렉터리로 취급한다.
활성 상태는 `current -> revisions/<revision>` 같은 심볼릭 링크 또는 동등한 포인터로 표현한다.

리비전에는 최소한 아래 내용이 포함되어야 한다.

- Nginx 설정 산출물
- plugin chain 정의
- Wasm 모듈 및 버전 정보
- 참조 메타데이터
  - revision ID
  - 생성 시각
  - 호환 가능한 runtime 버전

## 반영 절차

1. 새 revision 산출물을 staging 위치에 배치한다.
2. 필수 파일 존재와 버전 호환성을 검증한다.
3. `nginx -t`로 최종 설정 유효성을 검증한다.
4. 검증 성공 시에만 활성 revision 포인터를 새 revision으로 전환한다.
5. reload 결과를 기록하고 관측성 이벤트를 남긴다.

## 실패 처리

- 사전 검증 실패 시 현재 활성 revision을 유지한다.
- reload 실패 시 즉시 이전 정상 revision으로 복귀한다.
- rollback 결과는 로그와 메트릭에 남긴다.
- 실패 원인은 운영자가 문서만 보고도 재현 가능하도록 구조화된 필드로 기록한다.

## 운영 acceptance 기준

- 새 revision은 `nginx -t` 성공 없이는 활성화되지 않는다.
- 활성 revision은 항상 하나만 존재한다.
- 마지막 정상 revision은 rollback 가능한 형태로 보존된다.
- reload와 rollback 성공/실패는 모두 메트릭과 로그에서 구분 가능해야 한다.

## 관련 문서

- [운영 관측성](../05-operations/observability.md)
- [배포 검증과 롤백 절차](../05-operations/deployment-and-rollback.md)
