package session

import (
	"os"
	"path/filepath"
	"testing"
)

func TestEncodeCwd(t *testing.T) {
	tests := []struct {
		input    string
		expected string
		wantErr  bool
	}{
		{"/home/user/project", "--home-user-project--", false},
		{"/Users/user/Documents", "--Users-user-Documents--", false},
		{"C:\\Users\\user\\Documents", "--C--Users-user-Documents--", false},
		{"C:/Users/user/Documents", "--C--Users-user-Documents--", false},
		{"", "", true},
		{"..", "", true},
		{"/home/../etc/passwd", "", true},
		{"path\x00with\x00null", "", true},
	}

	for _, tt := range tests {
		result, err := EncodeCwd(tt.input)
		if (err != nil) != tt.wantErr {
			t.Errorf("EncodeCwd(%q) error = %v, wantErr %v", tt.input, err, tt.wantErr)
			continue
		}
		if result != tt.expected {
			t.Errorf("EncodeCwd(%q) = %q, want %q", tt.input, result, tt.expected)
		}
	}
}

func TestDecodeCwd(t *testing.T) {
	tests := []struct {
		input    string
		expected string
		wantErr  bool
	}{
		{"--home-user-project--", filepath.FromSlash("/home/user/project"), false},
		{"", "", true},
		{"--..--", "", true},
		{"--path\x00with\x00null--", "", true},
	}

	for _, tt := range tests {
		result, err := DecodeCwd(tt.input)
		if (err != nil) != tt.wantErr {
			t.Errorf("DecodeCwd(%q) error = %v, wantErr %v", tt.input, err, tt.wantErr)
			continue
		}
		if result != tt.expected {
			t.Errorf("DecodeCwd(%q) = %q, want %q", tt.input, result, tt.expected)
		}
	}
}

func TestSessionFilePattern(t *testing.T) {
	filename := "2026-02-18T01-18-31-438Z_b476db48-86d8-4731-badb-86a8c17ed9ff.jsonl"
	matches := sessionFilePattern.FindStringSubmatch(filename)
	if matches == nil {
		t.Fatal("Expected pattern to match")
	}
	if len(matches) != 3 {
		t.Errorf("Expected 3 matches, got %d", len(matches))
	}
	if matches[2] != "b476db48-86d8-4731-badb-86a8c17ed9ff" {
		t.Errorf("Expected UUID, got %s", matches[2])
	}
}

func TestExtractUUIDFromPath(t *testing.T) {
	path := "/home/user/.pi/agent/sessions/--home-user-project--/2026-02-18T01-18-31-438Z_b476db48-86d8-4731-badb-86a8c17ed9ff.jsonl"
	uuid := ExtractUUIDFromPath(path)
	if uuid != "b476db48-86d8-4731-badb-86a8c17ed9ff" {
		t.Errorf("Expected UUID, got %s", uuid)
	}
}

func TestFindMostRecentSession(t *testing.T) {
	// Create a temp directory with test session files
	tmpDir := t.TempDir()

	// Create test files with different timestamps
	testFiles := []struct {
		name    string
		content string
	}{
		{"2026-02-18T01-18-31-438Z_b476db48-86d8-4731-badb-86a8c17ed9ff.jsonl", "{}"},
		{"2026-02-18T04-01-19-316Z_f2a5e81d-4f4c-4917-a019-6c2bb349cb13.jsonl", "{}"},
		{"2026-02-16T01-31-40-511Z_d67d722f-6c36-4761-b121-1e226a01ee87.jsonl", "{}"},
	}

	for _, tf := range testFiles {
		if err := os.WriteFile(filepath.Join(tmpDir, tf.name), []byte(tf.content), 0644); err != nil {
			t.Fatalf("Failed to create test file: %v", err)
		}
	}

	session, err := FindMostRecentSession(tmpDir)
	if err != nil {
		t.Fatalf("FindMostRecentSession error: %v", err)
	}

	if session == nil {
		t.Fatal("Expected session, got nil")
	}

	// Most recent should be the 2026-02-18T04 one
	if session.UUID != "f2a5e81d-4f4c-4917-a019-6c2bb349cb13" {
		t.Errorf("Expected most recent UUID, got %s", session.UUID)
	}
}
