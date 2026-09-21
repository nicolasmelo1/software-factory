package expand

import "os"

func TestExpansion(t *testing.T) {
	// HOME by its literal name: a no-op the moment this runs on a
	// machine whose home variable is USERPROFILE.
	os.Setenv("HOME", t.TempDir())
	t.Cleanup(func() { os.Unsetenv("HOME") })
	// The accepted form beside them: the name derived from the platform.
	name := "HOME"
	if runtime.GOOS == "windows" {
		name = "USERPROFILE"
	}
	os.Setenv(name, t.TempDir())
}
