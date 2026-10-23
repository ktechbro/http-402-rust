#!/usr/bin/env bash
# Intentionally forgeable teaching payload, not a wallet signature.
set -euo pipefail
base_url="${BASE_URL:-http://127.0.0.1:3000}"
tmp_dir="$(mktemp -d)"
trap 'rm -rf -- "$tmp_dir"' EXIT

status=$(curl --silent --show-error --max-time 10 \
    --dump-header "$tmp_dir/headers" --output "$tmp_dir/body" \
    --write-out '%{http_code}' "$base_url/premium")
if [[ "$status" != 402 ]]; then
    echo "Expected HTTP 402; got $status" >&2
    exit 1
fi
echo '1. Unpaid request: HTTP 402 Payment Required'
encoded=$(awk 'tolower($1) == "payment-required:" {gsub("\r", "", $2); print $2}' "$tmp_dir/headers")
printf '%s' "$encoded" | base64 --decode > "$tmp_dir/challenge"
echo '2. Decoded PAYMENT-REQUIRED:'
jq . "$tmp_dir/challenge"

payment=$(jq -c '{x402Version: 2, resource: .resource, accepted: .accepts[0], payload: {demoToken: "tutorial-only"}}' "$tmp_dir/challenge")
signature=$(printf '%s' "$payment" | base64 --wrap=0)
status=$(curl --silent --show-error --max-time 10 \
    --header "PAYMENT-SIGNATURE: $signature" --output "$tmp_dir/result" \
    --write-out '%{http_code}' "$base_url/premium")
if [[ "$status" != 200 ]]; then
    echo "Expected HTTP 200 on retry; got $status" >&2
    cat "$tmp_dir/result" >&2
    exit 1
fi
jq -e '.simulation == true and (.data | type == "string")' "$tmp_dir/result" > /dev/null
echo '3. Retry with simulated payment: HTTP 200 OK'
jq . "$tmp_dir/result"
echo 'No funds moved. This token is public, reusable, and has no monetary value.'
