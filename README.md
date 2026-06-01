# srvcs-biconditional

## Name

| Field | Value |
| --- | --- |
| Service | `srvcs-biconditional` |
| Slug | `biconditional` |
| Repository | `srvcs/biconditional` |
| Package | `srvcs-biconditional` |
| Kind | `orchestrator` |

## Function

logic: a if and only if b

## Dependencies

| Dependency | Repository |
| --- | --- |
| `srvcs-implication` | [srvcs/implication](https://github.com/srvcs/implication) |
| `srvcs-and` | [srvcs/and](https://github.com/srvcs/and) |

## API

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/` | Service identity |
| `POST` | `/` | Evaluate the service function |
| `GET` | `/healthz` | Liveness probe |
| `GET` | `/readyz` | Readiness probe |
| `GET` | `/metrics` | Prometheus metrics |
| `GET` | `/openapi.json` | OpenAPI document |

## Inputs

| Name | Type | Required |
| --- | --- | --- |
| `a` | `json` | yes |
| `b` | `json` | yes |

## Outputs

| Name | Type |
| --- | --- |
| `a` | `json` |
| `b` | `json` |
| `result` | `boolean` |

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `SRVCS_BIND_ADDR` | `0.0.0.0:8080` | Bind address |
| `SRVCS_ENV` | `development` | Environment label for logs |
| `RUST_LOG` | `info,tower_http=info` | Tracing filter |
| `SRVCS_AND_URL` | `http://127.0.0.1:8080` | Base URL for srvcs-and |
| `SRVCS_IMPLICATION_URL` | `http://127.0.0.1:8080` | Base URL for srvcs-implication |

## Error Behavior

- `422` means the request could not be evaluated for the documented input shape.
- `503` means a required dependency was unavailable or returned an unexpected response.
- Dependency validation errors are forwarded when this service delegates validation.

## Local Checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

See the [srvcs service standard](https://github.com/srvcs/platform/blob/main/STANDARD.md) for the full operational contract.

## Metadata

Machine-readable service metadata lives in `srvcs.yaml`. Keep it aligned with this README when the service contract changes.
