// Package models provides model lists for each supported provider.
package models

import "context"

type Models map[string][]string

func ForProvider(ctx context.Context, provider string) []string {
	all, err := GetModels(ctx)
	if err != nil {
		return []string{}
	}
	return all[provider]
}

func GetAll() Models {
	all, err := GetModels(context.Background())
	if err != nil {
		return Models{}
	}
	return all
}
