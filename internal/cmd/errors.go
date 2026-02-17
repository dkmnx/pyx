package cmd

import (
	"fmt"
	"os"
)

// fatal prints an error to stderr and exits with status 1.
// This is used for non-recoverable errors in command handlers.
func fatal(err error) {
	fmt.Fprintf(os.Stderr, "Error: %v\n", err)
	os.Exit(1)
}

// fatalf formats an error message, prints to stderr, and exits with status 1.
// This is used for non-recoverable errors in command handlers.
func fatalf(format string, args ...interface{}) {
	fmt.Fprintf(os.Stderr, format+"\n", args...)
	os.Exit(1)
}
