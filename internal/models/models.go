// Package models provides model lists for each supported provider.
package models

import "context"

type Models map[string][]string

// ForProvider returns the list of models for a specific provider.
// It returns an empty slice if the provider is not found or if there's an error.
func ForProvider(ctx context.Context, provider string) []string {
	all, err := GetModels(ctx)
	if err != nil {
		return []string{}
	}
	return all[provider]
}
