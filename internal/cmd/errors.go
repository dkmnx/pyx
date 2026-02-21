package cmd

// Error handling utilities are now inlined in command handlers.
// See runRoot and other commands for error handling patterns:
// - Print error to stderr: fmt.Fprintf(os.Stderr, "Error: %v\n", err)
// - Exit with status 1: os.Exit(1)
