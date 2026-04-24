# 배포 검증과 롤백 절차

## 배포 절차

1. control plane이 생성한 revision 산출물을 수신한다.
2. revision 메타데이터, 필수 파일, plugin 호환성을 검증한다.
3. `nginx -t`로 구문과 참조 무결성을 검증한다.
4. 성공 시 활성 revision을 전환하고 Nginx를 reload 한다.
5. 결과를 로그와 메트릭에 기록한다.

## 실패 시 처리

- 사전 검증 실패: 새 revision 폐기, 현재 revision 유지
- reload 실패: 이전 정상 revision으로 즉시 rollback
- 외부 의존성 실패: plugin별 failure mode에 따라 `fail-open`, `fail-close`, `fail-static-response` 수행

## 운영 체크리스트

- 활성 revision ID를 즉시 확인할 수 있어야 한다.
- 최근 reload 실패 이력을 메트릭과 로그에서 조회할 수 있어야 한다.
- plugin chain과 plugin 버전이 요청 로그에 노출되어야 한다.
- fallback 적용 여부를 요청 단위로 추적할 수 있어야 한다.

## 테스트 기준

- 정상 revision 배포 시 reload 성공
- 잘못된 Nginx 설정 배포 시 활성 revision 유지
- 호환되지 않는 plugin 버전 배포 시 활성화 차단
- reload 실패 시 자동 rollback 성공
