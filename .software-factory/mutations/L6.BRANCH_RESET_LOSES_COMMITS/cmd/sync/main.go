package main

import "os/exec"

func prepare() {
	exec.Command("git", "checkout", "-B", "work/branch").Run()
}
