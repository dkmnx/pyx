package prompt

import (
	"os"
	"strings"
	"testing"

	"github.com/spf13/cobra"
)

func TestRandomSuffix(t *testing.T) {
	for i := 0; i < 100; i++ {
		suffix := randomSuffix(4)
		if len(suffix) != 8 {
			t.Errorf("randomSuffix(4) = %v, want length 8", suffix)
		}

		for _, c := range suffix {
			if !((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f')) {
				t.Errorf("randomSuffix() contains invalid char %c", c)
				break
			}
		}
	}
}

func TestRandomSuffixUniqueness(t *testing.T) {
	suffixes := make(map[string]bool)
	for i := 0; i < 100; i++ {
		suffix := randomSuffix(4)
		if suffixes[suffix] {
			t.Logf("Warning: duplicate suffix generated: %s", suffix)
		}
		suffixes[suffix] = true
	}

	if len(suffixes) < 90 {
		t.Errorf("randomSuffix() generated too many duplicates: %d unique out of 100", len(suffixes))
	}
}

func TestReadLine(t *testing.T) {
	origStdin := os.Stdin
	defer func() { os.Stdin = origStdin }()

	r, w, _ := os.Pipe()
	w.WriteString("test input\n")
	w.Close()
	os.Stdin = r

	got, err := ReadLine()
	if err != nil {
		t.Errorf("ReadLine() error = %v", err)
		return
	}
	if got != "test input" {
		t.Errorf("ReadLine() = %q, want %q", got, "test input")
	}
}

func TestReadLine_Empty(t *testing.T) {
	origStdin := os.Stdin
	defer func() { os.Stdin = origStdin }()

	r, w, _ := os.Pipe()
	w.WriteString("\n")
	w.Close()
	os.Stdin = r

	got, err := ReadLine()
	if err != nil {
		t.Errorf("ReadLine() error = %v", err)
		return
	}
	if got != "" {
		t.Errorf("ReadLine() = %q, want empty string", got)
	}
}

func TestReadLine_WithNewline(t *testing.T) {
	origStdin := os.Stdin
	defer func() { os.Stdin = origStdin }()

	r, w, _ := os.Pipe()
	w.WriteString("line1\nline2\n")
	w.Close()
	os.Stdin = r

	got, err := ReadLine()
	if err != nil {
		t.Errorf("ReadLine() error = %v", err)
		return
	}
	if got != "line1" {
		t.Errorf("ReadLine() = %q, want %q", got, "line1")
	}
}

func TestPromptLabel_Default(t *testing.T) {
	origStdin := os.Stdin
	defer func() { os.Stdin = origStdin }()

	r, w, _ := os.Pipe()
	w.WriteString("\n")
	w.Close()
	os.Stdin = r

	cmd := &cobra.Command{}
	got, err := PromptLabel(cmd, "openai")
	if err != nil {
		t.Errorf("PromptLabel() error = %v", err)
		return
	}
	if !strings.HasPrefix(got, "openai-") {
		t.Errorf("PromptLabel() = %q, want prefix %q", got, "openai-")
	}
}

func TestPromptLabel_Custom(t *testing.T) {
	origStdin := os.Stdin
	defer func() { os.Stdin = origStdin }()

	r, w, _ := os.Pipe()
	w.WriteString("my-custom-label\n")
	w.Close()
	os.Stdin = r

	cmd := &cobra.Command{}
	got, err := PromptLabel(cmd, "openai")
	if err != nil {
		t.Errorf("PromptLabel() error = %v", err)
		return
	}
	if got != "my-custom-label" {
		t.Errorf("PromptLabel() = %q, want %q", got, "my-custom-label")
	}
}

func TestPromptLabel_Whitespace(t *testing.T) {
	origStdin := os.Stdin
	defer func() { os.Stdin = origStdin }()

	r, w, _ := os.Pipe()
	w.WriteString("   \n")
	w.Close()
	os.Stdin = r

	cmd := &cobra.Command{}
	got, err := PromptLabel(cmd, "openai")
	if err != nil {
		t.Errorf("PromptLabel() error = %v", err)
		return
	}
	if !strings.HasPrefix(got, "openai-") {
		t.Errorf("PromptLabel() = %q, want prefix %q", got, "openai-")
	}
}
