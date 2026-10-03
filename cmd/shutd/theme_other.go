//go:build !windows
// +build !windows

package main

// appsUseDarkTheme is a stub for non-Windows platforms; app UI theme is
// meaningless there for this app.
func appsUseDarkTheme() bool {
	return false
}

// enableDarkMenus is a no-op stub for non-Windows platforms.
func enableDarkMenus(appsDark bool) {}

// allowDarkModeForTrayWindow is a no-op stub for non-Windows platforms.
func allowDarkModeForTrayWindow() {}

// taskbarUsesLightTheme is a stub for non-Windows platforms and always
// assumes a light taskbar.
func taskbarUsesLightTheme() bool {
	return true
}
