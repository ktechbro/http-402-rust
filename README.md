# Tutorial 1 — Understand HTTP 402 in Rust

A small Axum API that teaches the request → payment challenge → retry → resource lifecycle. Everything runs locally. No wallet, RPC connection, test USDC, or API keys are needed.

**This is a simulation, not a working x402 payment integration.** It uses the v2 HTTP header names and message structure with an invented `demo` scheme. Official clients will not pay this scheme automatically. The public token `tutorial-only` is deliberately forgeable and reusable; no signatures, expiry, replay protection, or settlement are implemented. Never deploy it as a paid service.

## Run

Install Docker Engine with the Compose plugin on Ubuntu, or Docker Desktop using Linux containers on Windows/macOS. No host Rust installation or PowerShell scripts are needed. Both build and runtime images use Ubuntu 24.04; the build installs Rust 1.90.0.

From the repository directory:

```sh
docker compose up --build -d --wait api
docker compose run --build --rm demo
```

The demo runs Bash, curl, and jq inside a separate Ubuntu container and reaches the API over the Compose network. To inspect responses inside the API container:

```sh
docker compose exec api curl -i http://127.0.0.1:3000/health
docker compose exec api curl -i http://127.0.0.1:3000/premium
```

The first call returns `200 OK`. The second returns `402 Payment Required` with a base64 JSON `PAYMENT-REQUIRED` header and a readable JSON body. The demo decodes the header, builds a demo authorization, and retries with `PAYMENT-SIGNATURE`; the successful response explicitly says no funds moved.

The API binds to `0.0.0.0:3000` inside its container and is published only on the host's `127.0.0.1:3000`. The advertised resource URL remains that host URL; the demo copies it from the challenge while sending requests to `http://api:3000`. If host port 3000 is occupied, stop the other service first.

```sh
docker compose logs api
docker compose down
```

To explore Ubuntu interactively, run `docker compose exec api bash`. The runtime runs as an unprivileged user. Rebuild with `docker compose up --build -d --wait api` after editing source; source files are copied during build, not mounted live.

## What 402 means

An HTTP status code tells the client what happened. `200` means success; `402` signals that payment is required. A 402 status alone neither transfers funds nor defines how to pay. x402 adds a protocol for advertising payment requirements and returning payment authorizations.

| Step | Client | Server |
| --- | --- | --- |
| 1 | `GET /premium` without a payment | Returns 402, challenge, and no protected data |
| 2 | Decodes `PAYMENT-REQUIRED` | Advertises resource and accepted payment requirements |
| 3 | Selects requirements and constructs a demo payload | Waits for the retry |
| 4 | Retries with base64 JSON in `PAYMENT-SIGNATURE` | Checks version, resource, terms, and demo token |
| 5 | Reads the resource | Returns 200 only after the demo checks pass |

Invalid or malformed demo authorizations receive another 402. Protected-route responses include `Cache-Control: no-store` so caches should not retain the challenge or unlocked response.

## Read the challenge

```json
{
  "x402Version": 2,
  "error": "PAYMENT-SIGNATURE header is required",
  "resource": {
    "url": "http://127.0.0.1:3000/premium",
    "description": "A local teaching resource; no real payment is accepted",
    "mimeType": "application/json"
  },
  "accepts": [{
    "scheme": "demo",
    "network": "demo:local",
    "amount": "1",
    "asset": "DEMO",
    "payTo": "tutorial-server",
    "maxTimeoutSeconds": 60
  }]
}
```

`resource` identifies what is being purchased. `accepts` lists available payment options. A client copies its selected option into `accepted` in the retry payload. Here, `amount` means one imaginary DEMO unit; `maxTimeoutSeconds` is illustrative and is not enforced. Base64 is an encoding, not encryption or proof of payment.

## Where the code lives

- `src/main.rs`: starts the HTTP server and handles Ctrl+C / Linux SIGTERM.
- `src/lib.rs`: routes, challenge construction, and explicit demo verification.
- `scripts/demo.sh`: Linux client that decodes the challenge and performs the retry.
- `Dockerfile` / `compose.yaml`: Ubuntu build, runtime, demo, and verification services.
- `tests/lifecycle.rs`: checks public access, challenge encoding, rejection of malformed or mismatched authorizations, and successful demo access.

## Verify

```sh
docker compose run --build --rm checks
```

## Exercises

1. Send an invalid header: `docker compose exec api curl -i -H "PAYMENT-SIGNATURE: invalid" http://127.0.0.1:3000/premium`. Explain why it stays locked.
2. Change the client's token or selected amount in the script and observe the rejection.
3. Explain why sending the same demo token twice succeeds and why a real payment implementation needs replay protection.
4. Explain why a client should decode the canonical header rather than rely on a specific error body.

Checkpoint: you can explain why the initial request returns 402, how the client discovers the requirements, and how a retry differs from the initial request.

## What changes with real payments?

A real payment client uses a wallet to form a scheme-specific authorization. The resource server verifies it, often through a facilitator. In the default authorization flow, the server verifies before executing the resource and settles afterward, then returns the resource with a `PAYMENT-RESPONSE` settlement header. This demo has no facilitator and emits no settlement receipt. Copying terms and supplying a string is not equivalent to verifying a cryptographic authorization.

Tutorials 2 and 3 introduce a testnet wallet and real paid API integration. Treat Rust payment implementation as separate work; this repository does not wrap an official Rust SDK.

## Primary references

- [x402 v2 core specification](https://github.com/x402-foundation/x402/blob/main/specs/x402-specification-v2.md)
- [x402 v2 HTTP transport](https://github.com/x402-foundation/x402/blob/main/specs/transports-v2/http.md)
- [Axum documentation](https://docs.rs/axum/latest/axum/)
