package cmd

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"sort"
	"time"

	"github.com/dkmnx/ply/internal/models"
	"github.com/spf13/cobra"
)

var (
	modelsProvider string
	modelsJSON     bool
)

var modelsCmd = &cobra.Command{
	Use:   "models",
	Short: "List supported AI models",
	Long:  `List all supported AI models from providers. Use --update to fetch the latest models from the remote repository.`,
	Run:   runModels,
}

var modelsUpdateCmd = &cobra.Command{
	Use:   "update",
	Short: "Update models from remote",
	Long:  `Fetch the latest models from the pi-mono repository and update the local cache.`,
	Run:   runModelsUpdate,
}

func init() {
	modelsCmd.Flags().StringVarP(&modelsProvider, "provider", "p", "", "Filter models by provider")
	modelsCmd.Flags().BoolVar(&modelsJSON, "json", false, "Output as JSON")
	modelsCmd.AddCommand(modelsUpdateCmd)
	rootCmd.AddCommand(modelsCmd)
}

func runModels(cmd *cobra.Command, args []string) {
	ctx, cancel := context.WithTimeout(context.Background(), 60*time.Second)
	defer cancel()

	modelsData, err := models.GetModels(ctx)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error loading models: %v\n", err)
		os.Exit(1)
	}

	if modelsJSON {
		outputJSON(modelsData)
		return
	}

	outputTable(modelsData)
}

func runModelsUpdate(cmd *cobra.Command, args []string) {
	ctx, cancel := context.WithTimeout(context.Background(), 90*time.Second)
	defer cancel()

	fmt.Println("Fetching latest models from pi-mono repository...")

	if err := models.FetchAndCache(ctx); err != nil {
		fmt.Fprintf(os.Stderr, "Error updating models: %v\n", err)
		os.Exit(1)
	}

	version, err := models.LoadCacheVersion()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Warning: could not load version info: %v\n", err)
		fmt.Println("Models updated successfully!")
		return
	}

	fmt.Printf("Models updated to version %s!\n", version)
}

func outputTable(modelsData models.Models) {
	providers := make([]string, 0, len(modelsData))
	for p := range modelsData {
		providers = append(providers, p)
	}
	sort.Strings(providers)

	if modelsProvider != "" {
		filteredProviders := []string{modelsProvider}
		if _, ok := modelsData[modelsProvider]; !ok {
			fmt.Fprintf(os.Stderr, "Provider '%s' not found\n", modelsProvider)
			os.Exit(1)
		}
		providers = filteredProviders
	}

	fmt.Println("Supported models:")
	fmt.Println()

	for _, provider := range providers {
		modelList := modelsData[provider]
		if len(modelList) == 0 {
			continue
		}

		fmt.Printf("  %s (%d models)\n", provider, len(modelList))
		for _, model := range modelList {
			fmt.Printf("    - %s\n", model)
		}
		fmt.Println()
	}

	totalModels := 0
	for _, provider := range providers {
		totalModels += len(modelsData[provider])
	}
	fmt.Printf("Total: %d providers, %d models\n", len(providers), totalModels)
}

func outputJSON(modelsData models.Models) {
	data := make(map[string][]string)
	for provider, modelList := range modelsData {
		if modelsProvider != "" && provider != modelsProvider {
			continue
		}
		data[provider] = modelList
	}

	output, err := json.MarshalIndent(data, "", "  ")
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error formatting JSON: %v\n", err)
		os.Exit(1)
	}

	fmt.Println(string(output))
}
