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

## 개발 환경 기동 및 확인

```bash
# 최초 1회: Rust 툴체인과 WasmX Nginx 설치
bin/gateway-dev bootstrap

# 빌드
bin/gateway-dev build

# 기동 (admin 서버 + mock upstream + sample revision 활성화)
bin/gateway-dev up

# 상태 확인
bin/gateway-dev status
```

### 제대로 부팅됐는지 확인하는 방법

**1. admin API 응답 확인**

```bash
curl http://<서버IP>:19080/status
```

정상이면 `current_revision`이 `null`이 아닌 revision 이름을 반환한다.

```json
{
  "current_revision": "local-dev-001",
  "last_validation": { "valid": true, "errors": [] },
  "last_reload_status": { "success": true }
}
```

**2. 게이트웨이 포트 응답 확인**

```bash
curl -i http://127.0.0.1:8080/
```

nginx가 떠 있으면 HTTP 응답(4xx 포함)이 돌아온다. connection refused면 nginx 기동 실패다.

**3. 메트릭 확인**

```bash
curl http://<서버IP>:19080/metrics
```

**4. 종료**

```bash
bin/gateway-dev down
```

## 패키지 빌드 (운영형 tarball)

```bash
bash scripts/package.sh
# → dist/gateway-dev-dist.tar.gz
```

생성물은 `gatewayd`, `nginx`, 실행 스크립트만 포함하는 self-contained tarball이다.
샘플 revision은 포함하지 않으므로, 실제 revision 번들은 별도로 준비해야 한다.

다른 머신이나 경로에서 바로 실행할 수 있다.

```bash
tar -xzf gateway-dev-dist.tar.gz
cd gateway-dev-dist
./run.sh
```

`run.sh`는 admin 서버만 기동한다.
이후 revision 번들을 `revisions/<revision-name>` 아래에 배치하고 명시적으로 활성화한다.

```bash
bin/gatewayd activate-revision --revision-path ./revisions/<revision-name>
```

버전 정보 확인:

```bash
bin/gatewayd version
```

종료:

```bash
./stop.sh
```
