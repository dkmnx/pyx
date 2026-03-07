package validation

import (
	"os"
	"path/filepath"
	"runtime"
	"testing"
)

func TestIsValidPath(t *testing.T) {
	tests := []struct {
		name    string
		path    string
		wantErr bool
	}{
		{
			name:    "empty path",
			path:    "",
			wantErr: true,
		},
		{
			name:    "null byte",
			path:    "test\x00path",
			wantErr: true,
		},
		{
			name:    "path traversal",
			path:    "/home/user/../etc/passwd",
			wantErr: true,
		},
		{
			name:    "valid path",
			path:    "/home/user/.local/share/ply",
			wantErr: false,
		},
		{
			name:    "path too long",
			path:    string(make([]byte, 5000)),
			wantErr: true,
		},
		{
			name:    "simple valid path",
			path:    "/home/user",
			wantErr: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := IsValidPath(tt.path)
			if (err != nil) != tt.wantErr {
				t.Errorf("IsValidPath() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestValidateAndResolveHome(t *testing.T) {
	// Save original HOME environment variable
	origHome := os.Getenv("HOME")
	origUserProf := os.Getenv("USERPROFILE")
	defer func() {
		if origHome != "" {
			os.Setenv("HOME", origHome)
		} else {
			os.Unsetenv("HOME")
		}
		if origUserProf != "" {
			os.Setenv("USERPROFILE", origUserProf)
		} else {
			os.Unsetenv("USERPROFILE")
		}
	}()

	tests := []struct {
		name    string
		setup   func()
		wantErr bool
	}{
		{
			name: "valid HOME directory",
			setup: func() {
				os.Setenv("HOME", os.Getenv("PWD"))
				os.Unsetenv("USERPROFILE")
			},
			wantErr: false,
		},
		{
			name: "valid USERPROFILE (Windows)",
			setup: func() {
				os.Unsetenv("HOME")
				os.Setenv("USERPROFILE", os.Getenv("PWD"))
			},
			wantErr: false,
		},
		{
			name: "no home directory set",
			setup: func() {
				os.Unsetenv("HOME")
				os.Unsetenv("USERPROFILE")
			},
			wantErr: true,
		},
		{
			name: "invalid home with traversal",
			setup: func() {
				os.Setenv("HOME", "/home/user/../etc")
				os.Unsetenv("USERPROFILE")
			},
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			tt.setup()
			_, err := ValidateAndResolveHome()
			if (err != nil) != tt.wantErr {
				t.Errorf("ValidateAndResolveHome() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestValidateDataDir(t *testing.T) {
	tests := []struct {
		name    string
		dataDir string
		wantErr bool
	}{
		{
			name:    "empty data dir",
			dataDir: "",
			wantErr: true,
		},
		{
			name:    "valid absolute path",
			dataDir: "/home/user/.local/share/ply",
			wantErr: false,
		},
		{
			name:    "path traversal attempt",
			dataDir: "/home/user/../etc/ply",
			wantErr: true,
		},
		{
			name:    "null byte in path",
			dataDir: "/home/user/\x00ply",
			wantErr: true,
		},
		{
			name:    "relative path",
			dataDir: "relative/path",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, err := ValidateDataDir(tt.dataDir)
			if (err != nil) != tt.wantErr {
				t.Errorf("ValidateDataDir() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestSanitizeShellArg(t *testing.T) {
	tests := []struct {
		name    string
		arg     string
		wantErr bool
	}{
		{
			name:    "empty argument",
			arg:     "",
			wantErr: true,
		},
		{
			name:    "valid argument",
			arg:     "bash",
			wantErr: false,
		},
		{
			name:    "argument with spaces",
			arg:     "some argument",
			wantErr: false,
		},
		{
			name:    "argument too long",
			arg:     string(make([]byte, 2000)),
			wantErr: true,
		},
		{
			name:    "null byte",
			arg:     "test\x00arg",
			wantErr: true,
		},
		{
			name:    "valid zsh",
			arg:     "zsh",
			wantErr: false,
		},
		{
			name:    "valid fish",
			arg:     "fish",
			wantErr: false,
		},
		{
			name:    "valid powershell",
			arg:     "powershell",
			wantErr: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := SanitizeShellArg(tt.arg)
			if (err != nil) != tt.wantErr {
				t.Errorf("SanitizeShellArg() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestValidateShellArgs(t *testing.T) {
	tests := []struct {
		name    string
		args    []string
		wantErr bool
	}{
		{
			name:    "valid args",
			args:    []string{"--help", "--version"},
			wantErr: false,
		},
		{
			name:    "empty args",
			args:    []string{},
			wantErr: false,
		},
		{
			name:    "one invalid arg",
			args:    []string{"--help", string(make([]byte, 2000))},
			wantErr: true,
		},
		{
			name:    "arg with null byte",
			args:    []string{"--help", "\x00"},
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := ValidateShellArgs(tt.args)
			if (err != nil) != tt.wantErr {
				t.Errorf("ValidateShellArgs() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestIsValidPathControlCharacters(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("control character check skipped on Windows")
	}

	tests := []struct {
		name    string
		path    string
		wantErr bool
	}{
		{
			name:    "path with control char",
			path:    "/home/user\x01/test",
			wantErr: true,
		},
		{
			name:    "path with bell char",
			path:    "/home/user\x07/test",
			wantErr: true,
		},
		{
			name:    "path with form feed",
			path:    "/home/user\x0c/test",
			wantErr: true,
		},
		{
			name:    "path with escape char",
			path:    "/home/user\x1b/test",
			wantErr: true,
		},
		{
			name:    "valid path with newline",
			path:    "/home/user/test\nfile",
			wantErr: false,
		},
		{
			name:    "valid path with tab",
			path:    "/home/user/test\tfile",
			wantErr: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := IsValidPath(tt.path)
			if (err != nil) != tt.wantErr {
				t.Errorf("IsValidPath() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestValidateAndResolveHomeRealPath(t *testing.T) {
	home, err := ValidateAndResolveHome()
	if err != nil {
		t.Skipf("Cannot validate home directory: %v", err)
		return
	}

	// Verify the path is absolute
	if !filepath.IsAbs(home) {
		t.Errorf("ValidateAndResolveHome() returned non-absolute path: %s", home)
	}

	// Verify it exists
	info, err := os.Stat(home)
	if err != nil {
		t.Errorf("ValidateAndResolveHome() returned non-existent path: %v", err)
		return
	}

	// Verify it's a directory
	if !info.IsDir() {
		t.Errorf("ValidateAndResolveHome() returned non-directory: %s", home)
	}
}
