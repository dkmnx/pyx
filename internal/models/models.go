// Package models provides model lists for each supported provider.
package models

type Models map[string][]string

func ForProvider(provider string) []string {
	all, err := GetModels()
	if err != nil {
		return nil
	}
	return all[provider]
}

func GetAll() Models {
	all, err := GetModels()
	if err != nil {
		return nil
	}
	return all
}
