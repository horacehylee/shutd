//go:build windows

package shutd

import (
	"context"
	"os"
	"os/signal"
	"syscall"
	"unsafe"

	"github.com/tadvi/winc"
	"github.com/tadvi/winc/w32"
)

var (
	w          *winc.Form
	titleLab   *winc.Label
	descLab    *winc.Label
	dismissBtn *winc.PushButton
	snoozeBtn  *winc.PushButton
	chanResult chan bool
)

func init() {
	chanResult = make(chan bool, 1)

	w = winc.NewForm(nil)
	w.SetAndClearStyleBits(0, w32.WS_SIZEBOX)
	w.SetAndClearStyleBits(0, w32.WS_BORDER)
	w.SetAndClearStyleBits(w32.WS_POPUP, 0)
	w.SetSize(450, 200)
	w.SetText("Shutd - Snooze notification")
	w.EnableTopMost(true)
	bottomRight(w, 16)

	titleLab = winc.NewLabel(w)
	titleLab.SetText("Shutd - Shutdown at 01:00")
	titleLab.SetFont(winc.NewFont("MS Shell Dlg 2", 16, winc.FontBold))
	titleLab.SetPos(20, 22)
	titleLab.SetSize(450, 30)

	descLab = winc.NewLabel(w)
	descLab.SetText("Shutdown in 10 minutes, snooze for 15 minutes?")
	descLab.SetFont(winc.NewFont("MS Shell Dlg 2", 12, 0))
	descLab.SetPos(20, 80)
	descLab.SetSize(450, 30)

	dismissBtn = winc.NewPushButton(w)
	dismissBtn.SetDefault()
	dismissBtn.SetText("Dismiss")
	dismissBtn.SetFont(winc.NewFont("MS Shell Dlg 2", 10, 0))
	dismissBtn.SetPos(158, 134)
	dismissBtn.SetSize(128, 50)
	dismissBtn.OnClick().Bind(func(e *winc.Event) {
		w.Hide()
		chanResult <- false
	})

	snoozeBtn = winc.NewPushButton(w)
	snoozeBtn.SetText("Snooze")
	snoozeBtn.SetFont(winc.NewFont("MS Shell Dlg 2", 10, 0))
	snoozeBtn.SetPos(300, 134)
	snoozeBtn.SetSize(128, 50)
	snoozeBtn.OnClick().Bind(func(e *winc.Event) {
		w.Hide()
		chanResult <- true
	})

	go func() {
		winc.RunMainLoop()
	}()

	sigs := make(chan os.Signal, 1)
	signal.Notify(sigs, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-sigs
		winc.Exit()
	}()
}

func question(ctx context.Context, title, text string) (bool, error) {
	titleLab.SetText(title)
	descLab.SetText(text)

	w.Show()
	w32.BringWindowToTop(w.Handle())

	select {
	case res := <-chanResult:
		clearChannel(chanResult)
		return res, nil
	case <-ctx.Done():
		w.Hide()
		clearChannel(chanResult)
		return false, ctx.Err()
	}
}

func clearChannel(c chan bool) {
	for len(c) > 0 {
		<-c
	}
}

func bottomRight(w *winc.Form, padding int) {
	info := getMonitorInfo(w.Handle())
	rect := info.RcWork
	width, height := w.Size()
	x := rect.Right - rect.Left - int32(width) - int32(padding)
	y := rect.Bottom - rect.Top - int32(height) - int32(padding)
	w.SetPos(int(x), int(y))
}

func getMonitorInfo(hwnd w32.HWND) *w32.MONITORINFO {
	currentMonitor := w32.MonitorFromWindow(hwnd, w32.MONITOR_DEFAULTTONEAREST)
	var info w32.MONITORINFO
	info.CbSize = uint32(unsafe.Sizeof(info))
	w32.GetMonitorInfo(currentMonitor, &info)
	return &info
}
