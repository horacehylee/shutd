//go:build windows
// +build windows

package main

import (
	"golang.org/x/sys/windows/registry"
	"syscall"
	"unsafe"
)

// Dark menu support relies on undocumented uxtheme ordinal exports. They are
// stable since Windows 10 1809 and widely used (Files, Everything,
// TranslucentTB):
//
//	133 RefreshImmersiveColorPolicyState
//	134 AllowDarkModeForWindow
//	135 SetPreferredAppMode (AllowDarkModeForApp before 1903)
//	136 FlushMenuThemes
const (
	uxthemeRefreshColorPolicy = 133
	uxthemeAllowDarkForWindow = 134
	uxthemeSetPreferredMode   = 135
	uxthemeFlushMenuThemes    = 136
)

// PreferredAppMode values for SetPreferredAppMode (ordinal 135). ForceDark
// and ForceLight make the menu theme deterministic instead of relying on the
// unreliable ShouldAppsUseDarkMode policy query.
const (
	preferredAppModeForceDark  = 2
	preferredAppModeForceLight = 3
)

var (
	kernel32           = syscall.NewLazyDLL("kernel32.dll")
	procLoadLibraryW   = kernel32.NewProc("LoadLibraryW")
	procGetProcAddress = kernel32.NewProc("GetProcAddress")
	uxtheme            uintptr
)

func init() {
	name, err := syscall.UTF16PtrFromString("uxtheme.dll")
	if err != nil {
		return
	}
	uxtheme, _, _ = procLoadLibraryW.Call(uintptr(unsafe.Pointer(name)))
}

// callUxtheme invokes the uxtheme export at the given ordinal if it exists.
// Go's syscall does not support "#ordinal" procedure names, so resolve the
// address manually: GetProcAddress treats a numeric lpProcName as an ordinal.
func callUxtheme(ordinal uintptr, args ...uintptr) {
	if uxtheme == 0 {
		return
	}
	addr, _, _ := procGetProcAddress.Call(uxtheme, ordinal)
	if addr == 0 {
		return
	}
	syscall.SyscallN(addr, args...)
}

// appsUseDarkTheme reports whether Win32 menus and app UI should use dark
// mode, per the user's system app theme setting.
func appsUseDarkTheme() bool {
	k, err := registry.OpenKey(registry.CURRENT_USER, `SOFTWARE\Microsoft\Windows\CurrentVersion\Themes\Personalize`, registry.QUERY_VALUE)
	if err != nil {
		return false
	}
	defer k.Close()
	v, _, err := k.GetIntegerValue("AppsUseLightTheme")
	if err != nil {
		return false
	}
	return v == 0
}

// enableDarkMenus makes Win32 popup menus (e.g. the systray context menu)
// match the system app theme.
func enableDarkMenus(appsDark bool) {
	callUxtheme(uxthemeRefreshColorPolicy)
	mode := uintptr(preferredAppModeForceLight)
	if appsDark {
		mode = uintptr(preferredAppModeForceDark)
	}
	callUxtheme(uxthemeSetPreferredMode, mode)
	callUxtheme(uxthemeFlushMenuThemes)
}

// allowDarkModeForTrayWindow flags the systray message window for dark mode
// so the popup menus it owns render with the dark theme. Newer Windows builds
// gate menu theming on the owner window's dark flag, so this must be called
// after the window exists (e.g. from the systray onReady callback).
func allowDarkModeForTrayWindow() {
	cls, err := syscall.UTF16PtrFromString("SystrayClass")
	if err != nil {
		return
	}
	user32 := syscall.NewLazyDLL("user32.dll")
	findWindow := user32.NewProc("FindWindowW")
	hwnd, _, _ := findWindow.Call(uintptr(unsafe.Pointer(cls)), 0)
	if hwnd == 0 {
		return
	}
	callUxtheme(uxthemeAllowDarkForWindow, hwnd, 1)
	callUxtheme(uxthemeFlushMenuThemes)
}

// taskbarUsesLightTheme returns whether the taskbar (system theme) uses the
// light theme, so the proper tray icon variant can be picked at startup.
// It reads SystemUsesLightTheme instead of AppsUseLightTheme because the tray
// icon is drawn on the taskbar, whose color follows the system theme.
func taskbarUsesLightTheme() bool {
	k, err := registry.OpenKey(registry.CURRENT_USER, `SOFTWARE\Microsoft\Windows\CurrentVersion\Themes\Personalize`, registry.QUERY_VALUE)
	if err != nil {
		return true
	}
	defer k.Close()
	v, _, err := k.GetIntegerValue("SystemUsesLightTheme")
	if err != nil {
		return true
	}
	return v != 0
}
