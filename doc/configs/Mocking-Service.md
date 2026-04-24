# Mocking Service

**개요**

백엔드 서비스 없이 API 응답을 시뮬레이션하기 위한 설정입니다.

**필수 필드**

필드명 | 필수 | 설명
---|---|---
(필수 없음) | No | 필요한 기능만 선택적으로 사용

**타입별 가이드**

유형 | 주요 필드 | 사용 시나리오
---|---|---
기본 응답 | stub_payload | 고정 응답 반환
지연/오류 | random_delay_range, error_simulation_rate | 지연 및 오류 시뮬레이션
조건부 응답 | mock_rules | 조건별 응답 분기

**스키마**

```json
{
  "type": "object",
  "properties": {
    "mock_rules": {
      "type": "array",
      "items": { "type": "object" }
    },
    "stub_payload": { "type": ["object", "string"] },
    "random_delay_range": {
      "type": "object",
      "properties": {
        "min": { "type": "integer", "minimum": 0 },
        "max": { "type": "integer", "minimum": 0 }
      }
    },
    "error_simulation_rate": { "type": "number", "minimum": 0.0, "maximum": 1.0 }
  }
}
```

**예시**

```json
{
  "stub_payload": {
    "message": "This is a mocked response",
    "status": "success"
  },
  "random_delay_range": {
    "min": 100,
    "max": 500
  },
  "error_simulation_rate": 0.05
}
```
