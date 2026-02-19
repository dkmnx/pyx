package models

import (
	"fmt"
	"regexp"
	"strings"
)

func ParseModels(content string) (Models, error) {
	result := make(Models)

	providerFieldPattern := regexp.MustCompile(`provider:\s*"([^"]+)"`)
	modelIDPattern := regexp.MustCompile(`id:\s*"([^"]+)"`)
	providerSectionPattern := regexp.MustCompile(`^"([a-z][a-z0-9-]*)":\s*\{\s*$`)

	lines := strings.Split(content, "\n")
	currentProvider := ""

	for _, line := range lines {
		trimmed := strings.TrimSpace(line)

		if strings.HasPrefix(trimmed, "//") || strings.HasPrefix(trimmed, "import") || strings.HasPrefix(trimmed, "export") {
			continue
		}

		sectionMatch := providerSectionPattern.FindStringSubmatch(trimmed)
		if sectionMatch != nil {
			key := sectionMatch[1]
			currentProvider = key
			if result[currentProvider] == nil {
				result[currentProvider] = []string{}
			}
			continue
		}

		providerMatch := providerFieldPattern.FindStringSubmatch(trimmed)
		if providerMatch != nil {
			currentProvider = providerMatch[1]
			if result[currentProvider] == nil {
				result[currentProvider] = []string{}
			}
			continue
		}

		if currentProvider != "" {
			modelMatch := modelIDPattern.FindStringSubmatch(trimmed)
			if modelMatch != nil {
				modelID := modelMatch[1]
				exists := false
				for _, m := range result[currentProvider] {
					if m == modelID {
						exists = true
						break
					}
				}
				if !exists {
					result[currentProvider] = append(result[currentProvider], modelID)
				}
			}
		}
	}

	for provider := range result {
		if len(result[provider]) == 0 {
			delete(result, provider)
		}
	}

	if len(result) == 0 {
		return nil, fmt.Errorf("no models found in content")
	}

	return result, nil
}
