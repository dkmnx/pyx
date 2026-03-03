# Password Error Recovery Design

## Problem

When `ply` fails to decrypt the master key (wrong password, no password set, encryption corrupted), the current behavior is confusing with migration prompts and password retries. Users need a clearer path to recovery.

## Solution

### `ply` (main command)

When `loadMasterKey` fails for any reason (wrong password, no password, encryption corrupted):
- Print: `Could not decrypt your API keys. Run 'ply init' to recreate your configuration.`
- Exit with error code 1
- No password prompts, no migration attempts

### `ply init` (recovery mode)

When master key already exists but can't be loaded:
1. Detect the failure
2. Read existing `database.json` to get list of configured providers
3. Prompt user: "Your configuration needs to be recreated. Provider(s) found: `anthropic`, `openai`. Re-enter API keys?"
4. If yes: for each provider, prompt for new API key, re-encrypt with fresh master key
5. If no: cancel and exit

When master key doesn't exist at all:
- Keep current behavior (create fresh master key and prompt for password)

## Components to Modify

### `internal/cmd/root.go`

Simplify `loadMasterKey` function:
- Remove migration logic
- Remove password retry prompts
- Return simple error when decryption fails
- Caller prints generic message and exits

### `internal/cmd/init.go`

Enhance `runInit` to handle recovery:
- Check if master key exists but can't be loaded
- If so, enter recovery mode:
  - Load database to get provider list
  - Prompt user to re-enter API keys
  - Create fresh master key
  - Re-encrypt all providers

### `internal/keys/keys.go`

Add new error type or function:
- `CanLoad()` method to check if master key can be decrypted
- Used by init to detect recovery scenario

## Error Handling

| Scenario | `ply` behavior | `ply init` behavior |
|----------|---------------|---------------------|
| Master key doesn't exist | "master key not found, run 'ply init'" | Create fresh |
| Master key exists, wrong password | "Could not decrypt your API keys. Run 'ply init' to recreate your configuration." | Recovery mode |
| Master key exists, no password set | Same as above | Recovery mode |
| Master key exists, encryption corrupted | Same as above | Recovery mode |

## Testing

- Test `ply` with corrupted master key
- Test `ply` with wrong password
- Test `ply init` recovery mode with existing providers
- Test `ply init` recovery mode with empty database
- Test `ply init` fresh install (no master key)