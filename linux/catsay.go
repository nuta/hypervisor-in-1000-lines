package main

import (
	"fmt"
	"time"
)

func main() {
	// ASCII art cat saying "Hello World!"
	fmt.Println()
	fmt.Println("\033[33m     /\\_/\\  \033[0m")
	fmt.Println("\033[33m    ( \033[36mo.o\033[33m ) \033[0m")
	fmt.Println("\033[33m     > ^ <\033[0m")
	fmt.Println()
	fmt.Println("\033[32m   Hello World!\033[0m")
	fmt.Println()

	// Print messages every second
	counter := 1
	for {
		time.Sleep(1 * time.Second)
		fmt.Printf("\033[35mMessage #%d: The cat is still here!\033[0m\n", counter)
		counter++
	}
}