package session

import (
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"time"
)

// Info holds information about a pi session
type Info struct {
	Path      string
	UUID      string
	Timestamp time.Time
}

// PiSessionsDir returns the base sessions directory for pi
func PiSessionsDir() (string, error) {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(homeDir, ".pi", "agent", "sessions"), nil
}

// EncodeCwd encodes a working directory path for use in session directory names
func EncodeCwd(cwd string) string {
	// Replace both \ and / with - first (normalize all path separators)
	encoded := strings.ReplaceAll(cwd, "\\", "/")
	// Remove leading slash
	encoded = strings.TrimPrefix(encoded, "/")
	// Replace remaining / and : with -
	encoded = strings.ReplaceAll(encoded, "/", "-")
	encoded = strings.ReplaceAll(encoded, ":", "-")
	return "--" + encoded + "--"
}

// DecodeCwd decodes an encoded directory name back to the original path
func DecodeCwd(encoded string) string {
	// Remove -- prefix and suffix
	decoded := strings.TrimPrefix(encoded, "--")
	decoded = strings.TrimSuffix(decoded, "--")
	// Replace - back to / first, then convert to local separators
	decoded = strings.ReplaceAll(decoded, "-", "/")
	// Convert to local filepath separators
	decoded = filepath.FromSlash(decoded)
	// Add leading separator
	if !strings.HasPrefix(decoded, string(filepath.Separator)) {
		decoded = string(filepath.Separator) + decoded
	}
	return decoded
}

// DirForCwd returns the session directory for a given working directory
func DirForCwd(cwd string) (string, error) {
	sessionsDir, err := PiSessionsDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(sessionsDir, EncodeCwd(cwd)), nil
}

var sessionFilePattern = regexp.MustCompile(`^(\d{4}-\d{2}-\d{2}T\d{2}-\d{2}-\d{2}-\d{3}Z)_([[:xdigit:]]{8}-[[:xdigit:]]{4}-[[:xdigit:]]{4}-[[:xdigit:]]{4}-[[:xdigit:]]{12})\.jsonl$`)

// parseSessionTimestamp parses the non-standard timestamp format used by pi sessions:
// Format: 2026-02-18T04-01-19-316Z (YYYY-MM-DDTHH-MM-SS-MMMZ)
func parseSessionTimestamp(timestampStr string) (time.Time, error) {
	// timestampStr format: 2026-02-18T04-01-19-316Z
	// We need to convert to standard format: 2006-01-02T15:04:05.000Z

	// Match pattern: YYYY-MM-DDTHH-MM-SS-MMMZ
	timestampPattern := regexp.MustCompile(`^(\d{4})-(\d{2})-(\d{2})T(\d{2})-(\d{2})-(\d{2})-(\d{3})Z$`)
	matches := timestampPattern.FindStringSubmatch(timestampStr)
	if matches == nil {
		return time.Time{}, nil
	}

	// Build standard format string
	standard := matches[1] + "-" + matches[2] + "-" + matches[3] + "T" +
		matches[4] + ":" + matches[5] + ":" + matches[6] + "." + matches[7] + "Z"

	return time.Parse(time.RFC3339, standard)
}

// FindMostRecentSession finds the most recent session file in a session directory
func FindMostRecentSession(sessionDir string) (*Info, error) {
	entries, err := os.ReadDir(sessionDir)
	if err != nil {
		return nil, err
	}

	var sessions []Info
	for _, entry := range entries {
		if entry.IsDir() {
			continue
		}
		if !strings.HasSuffix(entry.Name(), ".jsonl") {
			continue
		}

		matches := sessionFilePattern.FindStringSubmatch(entry.Name())
		if matches == nil {
			continue
		}

		timestamp, err := parseSessionTimestamp(matches[1])
		if err != nil {
			continue
		}

		sessions = append(sessions, Info{
			Path:      filepath.Join(sessionDir, entry.Name()),
			UUID:      matches[2],
			Timestamp: timestamp,
		})
	}

	if len(sessions) == 0 {
		return nil, nil
	}

	// Sort by timestamp descending (most recent first)
	sort.Slice(sessions, func(i, j int) bool {
		return sessions[i].Timestamp.After(sessions[j].Timestamp)
	})

	return &sessions[0], nil
}

// ExtractUUIDFromPath extracts the session UUID from a session file path
func ExtractUUIDFromPath(path string) string {
	filename := filepath.Base(path)
	matches := sessionFilePattern.FindStringSubmatch(filename)
	if matches == nil {
		return ""
	}
	return matches[2]
}
