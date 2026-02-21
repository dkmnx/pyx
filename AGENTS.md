# AGENTS.md

Please read through this before writing your first line of production code.

---

## 1. Where to Put Your Code

We follow the community-standard project layout. When you create new features,
please place them in the correct directory:

- **`/cmd`**: Put your main application entry points here. If you're building a binary, it lives here.
- **`/internal`**: This is for code **private to this repo**. Do not import
  `internal` packages from outside this project. If you aren't sure if
  something should be public, put it here by default.
- **`/pkg`**: Only use this for library code you intentionally want other external projects to import.
- **`/configs`**: Store your configuration templates here.

### 2. Code Style & Formatting

- **Let the Tools Decide:** We do not debate formatting. Run `gofmt` or
  `goimports` on every file before committing. If the formatter changes it,
  that's the correct style.
- **Linting is Mandatory:** We use `golangci-lint`. Configure your IDE to
  run it on save, and ensure it passes locally before you push.
- **Keep it Simple:** Write code for the person who will read it next
  (likely you in six months). Avoid clever one-liners if a clear three-liner
  works better.
- **Naming:** Use `MixedCaps` for names. Keep them short but clear. Avoid
  abbreviations unless everyone knows them (e.g., use `ID`, not `Ident`).

### 3. Handling Errors

- **Never Ignore Errors:** In Go, errors are values. If a function returns
  an error, you must check it.
  - _Wrong:_ `doSomething()`
  - _Right:_ `if err := doSomething(); err != nil { return err }`
- **Add Context:** When passing an error up the stack, wrap it with context
  so we know where it failed. Use `fmt.Errorf("context: %w", err)`.
- **No Panics:** Do not use `panic` for normal error handling. Reserve it for unrecoverable states only.

### 4. Concurrency & Context

- **Pass Context:** Any function that does I/O, networking, or long-running
  work must accept `context.Context` as its **first argument**. This allows us
  to cancel operations cleanly.
- **Watch Your Goroutines:** Never start a goroutine without knowing how it
  stops. Leaked goroutines are a major cause of memory issues.
- **Race Detector:** When testing concurrent code, always run `go test -race`. If the race detector complains, fix it.

### 5. Testing Standards

- **Test Files:** Name them `*_test.go`.
- **Table-Driven Tests:** We prefer table-driven tests for unit testing. They make adding new cases easy.
- **Parallelize:** Use `t.Parallel()` in your tests whenever possible to speed up the CI pipeline.
- **Mocks:** Mock external dependencies (databases, APIs) using interfaces. Don't hit real services in unit tests.
- **Coverage:** We care about critical paths. Don't chase 100% coverage if
  it means writing trivial tests, but ensure all logic branches are covered.

### 6. Dependencies & Modules

- **Go Modules:** We use Go Modules. Always commit your `go.mod` and `go.sum`.
- **Think Before Importing:** Check the standard library first. Do we really
  need a new dependency? Every package we add is a security and maintenance
  surface.
- **Updates:** Keep dependencies up to date. If you see a security warning, prioritize fixing it.

### 7. Logging & Configuration

- **Structured Logs:** Do not use `fmt.Println`. Use our standard structured logger (e.g., `zap` or `log/slog`).
- **Log Levels:** Use `Info` for normal operations, `Warn` for recoverable
  issues, and `Error` for failures. Never log sensitive data like passwords
  or tokens.
- **12-Factor App:** Configure the app via Environment Variables. Provide
  sensible defaults in the code so the app runs out of the box.

### 8. Before You Commit (Checklist)

Before you open a Pull Request, please verify the following:

- [ ] Did you run `go mod tidy`?
- [ ] Did you run `golangci-lint` locally?
- [ ] Did you run `go test -race ./...`?
- [ ] Did you add comments to all exported functions?
- [ ] Is your `README.md` updated if you changed how to run the app?

### 9. Documentation

- **Godoc:** Every exported function, type, or constant must have a comment
  starting with its name (e.g., `// GetUser returns...`). This generates our
  documentation.
- **Why, Not What:** Comment _why_ you made a complex decision, not _what_
  the code is doing. The code should explain itself.
