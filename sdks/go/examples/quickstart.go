// Quickstart against a running http-admin (sak330-d).
package main

import (
	"fmt"
	"log"
	"os"

	sak "github.com/tycheung/swissarmynoife-sdk"
)

func main() {
	base := os.Getenv("SAK_HTTP")
	if base == "" {
		base = "http://127.0.0.1:8787"
	}
	c := sak.NewClient(base)
	health, err := c.Health()
	if err != nil {
		log.Fatal(err)
	}
	modules, err := c.ListModules()
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("health=%v\nmodules=%v\n", health, modules)
}
