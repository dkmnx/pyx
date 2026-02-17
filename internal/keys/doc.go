// Package keys provides secure storage for the master encryption key.
//
// It uses a tiered approach to secure key storage:
//  1. Primary: OS keyring/keychain (go-keyring)
//  2. Fallback: Password-derived key using Argon2 KDF
//
// This design provides the best security on desktop systems while
// maintaining usability on servers and other environments where
// keyring access may not be available.
package keys
