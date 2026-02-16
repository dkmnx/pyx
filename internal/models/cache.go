package models

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"time"

	"github.com/dkmnx/ply/internal/fs"
)

const (
	modelsCacheFile = "models.json"
	versionFile     = "models-version.json"
)

var ErrNoCache = errors.New("no models cache found")

type Cache struct {
	Version   string    `json:"version"`
	UpdatedAt time.Time `json:"updated_at"`
	Models    Models    `json:"models"`
}

type VersionInfo struct {
	Tag       string    `json:"tag"`
	UpdatedAt time.Time `json:"updated_at"`
}

func cachePath() (string, error) {
	dataDir, err := fs.DataDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(dataDir, modelsCacheFile), nil
}

func versionPath() (string, error) {
	dataDir, err := fs.DataDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(dataDir, versionFile), nil
}

func LoadCache() (*Cache, error) {
	path, err := cachePath()
	if err != nil {
		return nil, err
	}

	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, ErrNoCache
		}
		return nil, fmt.Errorf("failed to read models cache: %w", err)
	}

	var cache Cache
	if err := json.Unmarshal(data, &cache); err != nil {
		return nil, fmt.Errorf("failed to parse models cache: %w", err)
	}

	return &cache, nil
}

func SaveCache(models Models, version string) error {
	dataDir, err := fs.EnsureDataDir()
	if err != nil {
		return err
	}

	cache := Cache{
		Version:   version,
		UpdatedAt: time.Now().UTC(),
		Models:    models,
	}

	data, err := json.MarshalIndent(cache, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal models cache: %w", err)
	}

	cachePath := filepath.Join(dataDir, modelsCacheFile)
	if err := os.WriteFile(cachePath, data, 0600); err != nil {
		return fmt.Errorf("failed to write models cache: %w", err)
	}

	versionInfo := VersionInfo{
		Tag:       version,
		UpdatedAt: time.Now().UTC(),
	}
	versionData, err := json.MarshalIndent(versionInfo, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal version info: %w", err)
	}

	versionPath := filepath.Join(dataDir, versionFile)
	if err := os.WriteFile(versionPath, versionData, 0600); err != nil {
		return fmt.Errorf("failed to write version info: %w", err)
	}

	return nil
}

func LoadVersion() (*VersionInfo, error) {
	path, err := versionPath()
	if err != nil {
		return nil, err
	}

	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, ErrNoCache
		}
		return nil, fmt.Errorf("failed to read version info: %w", err)
	}

	var version VersionInfo
	if err := json.Unmarshal(data, &version); err != nil {
		return nil, fmt.Errorf("failed to parse version info: %w", err)
	}

	return &version, nil
}

func CacheExists() bool {
	path, err := cachePath()
	if err != nil {
		return false
	}
	_, err = os.Stat(path)
	return err == nil
}

func ClearCache() error {
	cachePath, err := cachePath()
	if err != nil {
		return err
	}
	if err := os.Remove(cachePath); err != nil && !os.IsNotExist(err) {
		return fmt.Errorf("failed to remove cache: %w", err)
	}

	versionPath, err := versionPath()
	if err != nil {
		return err
	}
	if err := os.Remove(versionPath); err != nil && !os.IsNotExist(err) {
		return fmt.Errorf("failed to remove version file: %w", err)
	}

	return nil
}
