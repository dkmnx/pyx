Crypto Compatibility Test Fixtures
=====================================

These fixtures are for testing Rust's ability to decrypt Go-generated encrypted data.

Test Data:
----------
- Passphrase: test-passphrase
- Master key: 32 random bytes (encrypted in master.key)
- Providers: openai, anthropic (API keys encrypted with master key)

Testing Instructions:
---------------------
1. Rust implementation should decrypt master.key using passphrase "test-passphrase"
2. Use decrypted master key to decrypt provider ciphers in database.json
3. Verify decrypted API keys match:
   - openai: sk-openai-test-key-12345
   - anthropic: sk-ant-test-key-67890

File Formats:
-------------
All JSON files use the same format as the production application.
The master.key file contains a base64-encoded age ciphertext.

Generated: 2026-03-12T21:08:44+08:00
