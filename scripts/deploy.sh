#!/usr/bin/env bash
# Deploy mark_perp to devnet and print the program id.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
export PATH="${HOME}/.local/share/solana/install/active_release/bin:${PATH}"

ADMIN_KEYPAIR="${ADMIN_KEYPAIR:-$root/keys/admin.json}"
PROGRAM_KEYPAIR="${PROGRAM_KEYPAIR:-$root/keys/mark_perp-keypair.json}"
mkdir -p target/deploy
[[ -f "$ADMIN_KEYPAIR" ]] || { echo "missing admin key: $ADMIN_KEYPAIR" >&2; exit 1; }
[[ -f "$PROGRAM_KEYPAIR" ]] || { echo "missing program key: $PROGRAM_KEYPAIR" >&2; exit 1; }
cp "$PROGRAM_KEYPAIR" target/deploy/mark_perp-keypair.json

solana config set --url devnet >/dev/null
solana config set --keypair "$ADMIN_KEYPAIR" >/dev/null

echo "Admin: $(solana address)"
solana balance

anchor build
anchor deploy --provider.cluster devnet --program-keypair "$PROGRAM_KEYPAIR"
echo "Program: $(solana-keygen pubkey "$PROGRAM_KEYPAIR")"
echo "Next: cd app && npx tsx scripts/init_markets.ts"
