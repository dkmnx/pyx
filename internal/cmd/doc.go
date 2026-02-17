// Package cmd implements the CLI commands for ply.
//
// # Error Handling Philosophy
//
// This package uses a two-tier error handling approach:
//
//  1. Library code (database, fs, crypto, keys, etc.) returns errors
//     and lets callers decide how to handle them.
//
//  2. Command handlers (runRoot, runSetup, etc.) handle errors
//     directly by printing to stderr and calling os.Exit(1) for fatal
//     errors.
//
// This design is intentional:
//   - Library functions are reusable and testable - they return errors
//   - CLI commands have no recovery point above them - os.Exit() is appropriate
//   - Avoids error propagation through multiple layers in CLI context
//
// # Using RunE vs Run
//
// Most command functions use `Run` (not `RunE`) and handle errors
// internally with os.Exit(). This is simpler for direct error handling
// in CLI context.
//
// If you need Cobra's built-in error handling (e.g., for testing or
// when integrating with other code), use `RunE` instead:
//
//	var myCmd = &cobra.Command{
//	    Use: "mycommand",
//	    RunE: func(cmd *cobra.Command, args []string) error {
//	        if err := something(); err != nil {
//	            return fmt.Errorf("operation failed: %w", err)
//	        }
//	        return nil
//	    },
//	}
//
// # Exit Codes
//
//   - 0: Success
//   - 1: General error (default for all fatal errors)
//   - Child process exit codes are preserved and re-exited
//
// # Error Messages
//
// All errors are printed to stderr with consistent formatting:
//   - Fatal errors: "Error: description\n" or "Error: description: details\n"
//   - Non-fatal messages: "Warning: description\n" or info messages
//   - Success messages: "✓ description\n"
package cmd
