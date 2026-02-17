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

// warn prints a warning to stderr without exiting.
func warn(err error) {
	fmt.Fprintf(os.Stderr, "Warning: %v\n", err)
}

// warnf formats a warning message and prints to stderr without exiting.
func warnf(format string, args ...interface{}) {
	fmt.Fprintf(os.Stderr, format+"\n", args...)
}

// info prints an informational message to stdout.
func info(message string) {
	fmt.Println(message)
}

// infof formats and prints an informational message to stdout.
func infof(format string, args ...interface{}) {
	fmt.Printf(format+"\n", args...)
}
