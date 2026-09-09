# Codex usage bubble

Hover an opaque pet pixel to show the Codex quota bubble above its head. Dragging or leaving the pet hides it. The bubble is a separate transparent, non-focusable, click-through window, so it does not resize the pet or intercept desktop input. Its position is constrained to the monitor work area.

The display uses **used** percentage, followed by the next reset in the system's local timezone, with a Chinese weekday: `09/14（一） 21:03`. It selects `rateLimitsByLimitId.codex.primary`, then that bucket's secondary window when primary is absent. Older responses may use `rateLimits`. Missing data displays `—`, never a fabricated zero.

The Rust backend launches `codex app-server` over stdio, completes the `initialize` / `initialized` handshake, and reads:

- `account/rateLimits/read`: quota buckets, window lengths, used percentages, reset timestamps, credits, plan, limit state, available reset credits, and account identifier when provided.
- `account/read`: account authentication type, email, plan, and authentication requirement.
- `account/usage/read`: token activity summary, daily token buckets, and any additional fields returned by the installed CLI.

All response fields are retained in the `get_codex_usage` Tauri command result. Optional account/activity API errors are retained without preventing the quota display. The connection has a 25-second overall timeout. The child process is killed and reaped after each query, including failures. Queries do not start a model turn or consume a reset credit.

The UI refreshes 60 seconds after each query finishes. The backend shares a 60-second cache across callers. On refresh failure it preserves the last reading with an update-failure label and the last fetch time. With no successful reading it displays an unavailable state.

Install and sign in to Codex CLI before launching the pet. Executable discovery checks `CODEX_BIN`, `PATH`, then common Homebrew and macOS app locations; `CODEX_BIN` must name a single executable path. Authentication is managed by Codex itself.

To inspect all data using the same Rust implementation:

```sh
cargo run -p desktop-pet --example codex_usage
```

To save a local snapshot without adding account data to version control:

```sh
mkdir -p test-results
cargo run -p desktop-pet --example codex_usage > test-results/codex-usage.json
```

The API is documented but experimental; response shapes can change. Reference: https://learn.chatgpt.com/docs/app-server
