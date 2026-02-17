// Package models provides model lists for each supported provider.
package models

import (
	_ "embed"
	"encoding/json"
	"os"
)

type Models map[string][]string

//go:embed embedded.json
var embeddedJSON []byte

func EmbeddedModels() (Models, error) {
	cache, err := LoadCache()
	if err == nil {
		return cache.Models, nil
	}

	if !os.IsNotExist(err) {
		return nil, err
	}

	var models Models
	if err := json.Unmarshal(embeddedJSON, &models); err != nil {
		return nil, err
	}
	return models, nil
}

func ForProvider(provider string) []string {
	all, err := GetModels()
	if err == nil {
		if models, ok := all[provider]; ok {
			return models
		}
	}

	embedded, err := EmbeddedModels()
	if err == nil {
		if models, ok := embedded[provider]; ok {
			return models
		}
	}
	return nil
}

func GetAll() Models {
	all, err := GetModels()
	if err == nil {
		return all
	}
	embedded, err := EmbeddedModels()
	if err == nil {
		return embedded
	}
	return nil
}
