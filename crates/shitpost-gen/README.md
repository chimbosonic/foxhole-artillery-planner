# shitpost-gen

CLI tool that fetches artillery planner stats from the GraphQL API and generates a pro-Warden Foxhole subreddit end-of-war report via Claude Code.

## Prerequisites

- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) installed and authenticated (`claude login`)
- If your SSH key is in the macOS Keychain, unlock it first: `ssh-add`

## Usage

```bash
# Build
cargo build -p shitpost-gen

# Generate an end-of-war report (pipe stats into claude)
cargo run -p shitpost-gen -- --name YourName --clan YourClan | claude -p

# Use a custom GraphQL endpoint (e.g. local backend)
cargo run -p shitpost-gen -- --name YourName --clan YourClan --url http://localhost:3000/graphql | claude -p
```

### Required arguments

- `--name <in-game-name>` — Your Foxhole in-game name
- `--clan <clan-tag>` — Your clan tag

### Optional arguments

- `--url <graphql-url>` — GraphQL endpoint (default: `https://arty.dp42.dev/graphql`)

The tool outputs the system prompt + formatted stats to stdout. Status messages go to stderr. Pipe stdout into `claude -p` to generate the post.

## What it fetches

- Gun placements by weapon and faction (Warden / Colonial / Both)
- Gun placement totals per faction
- Target and spotter marker placement counts
