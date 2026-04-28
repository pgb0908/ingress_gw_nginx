# ingress-gateway

이 저장소는 로컬 개발 기준으로 아래 축으로 나뉜다.

- `app/`: 버전 관리되는 구현 코드
- `env/`: 개발환경 정의와 로컬 설치물
- `fixtures/`: 샘플 revision과 개발용 fixture
- `runtime/`: 실행 중 생성되는 상태와 Nginx 작업 디렉토리
- `scripts/`: 개발용 쉘 진입점
- `bin/`: 사용자 진입 명령
- `doc/`: 설계와 운영 문서

가장 자주 쓰는 명령은 `bin/gateway-dev bootstrap`, `build`, `up`, `status`, `down`이다.

빠른 구조 이해:

- `app/gatewayd`: Rust dataplane 본체
- `fixtures/revisions`: 샘플 revision과 fixture 입력
- `runtime/revisions`, `runtime/current`: 실행 시 stage/activate 되는 입력
- `runtime/dataplane`: Nginx conf, state, logs 같은 런타임 산출물
- `runtime/process`: PID와 프로세스 로그
- `env/local`: Rust toolchain과 WasmX Nginx
- `scripts/dev`: `gateway-dev`가 내부에서 호출하는 스크립트

상세 설계와 운영 문서는 [doc/README.md](/home/bong/RustroverProjects/ingress-gw-nginx/doc/README.md:1)를 기준으로 본다.
