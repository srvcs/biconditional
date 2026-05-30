# srvcs-biconditional

The biconditional orchestrator of the srvcs.cloud distributed standard library.

Its single concern: **does `a` hold if and only if `b` holds?** It does no logic
of its own. A biconditional `a <-> b` is the conjunction of the two implications:

```
a <-> b  ==  (a -> b) AND (b -> a)
```

It asks [`srvcs-implication`](https://github.com/srvcs/implication) for each
direction — `{"a": a, "b": b}` then `{"a": b, "b": a}` — and asks
[`srvcs-and`](https://github.com/srvcs/and) to combine the two verdicts.

## API

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/` | Service identity, concern, and dependency list |
| `POST` | `/` | Does `a` hold if and only if `b`? |
| `GET` | `/healthz` `/readyz` `/metrics` `/openapi.json` | srvcs service standard surface |

```sh
curl -s -X POST localhost:8080/ -H 'content-type: application/json' -d '{"a": true, "b": true}'
# {"a":true,"b":true,"result":true}
```

Responses:

- `200 {"a": x, "b": y, "result": true | false}` — evaluated.
- `422` — invalid input, forwarded from a leaf dependency.
- `503` — a dependency is unavailable.

## Dependencies

- [`srvcs-implication`](https://github.com/srvcs/implication)
- [`srvcs-and`](https://github.com/srvcs/and)

This is an orchestrator over boolean leaf services; its operands are booleans.
Input validation propagates from the leaf dependencies via their `422`
responses — this service does not validate operands itself.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `SRVCS_BIND_ADDR` | `0.0.0.0:8080` | Bind address |
| `SRVCS_IMPLICATION_URL` | `http://127.0.0.1:8080` | Base URL of `srvcs-implication` |
| `SRVCS_AND_URL` | `http://127.0.0.1:8080` | Base URL of `srvcs-and` |
| `SRVCS_ENV` | `development` | Environment label for logs |
| `RUST_LOG` | `info,tower_http=info` | Tracing filter |

## Local checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Orchestration tests stand up mock `srvcs-implication` and `srvcs-and` services
in-process, covering the truth table, a degraded dependency (`503`), and a
forwarded `422`. See [`srvcs/platform`](https://github.com/srvcs/platform) for
the shared standard.

> Note: the `cargoHash` in `flake.nix` is inherited from the template and must be
> refreshed with a `nix build` before the Nix gates pass.
