package main

import (
	"fmt"

	"github.com/getlantern/systray"
	"github.com/horacehylee/shutd"
	"github.com/horacehylee/shutd/cmd/shutd/icon"
	"github.com/sirupsen/logrus"
)

func startSystray(log *logrus.Logger, s *shutd.Scheduler) {
	onReady := func() {
		systray.SetTemplateIcon(icon.Data, icon.Data)
		systray.SetTitle("Shutd")
		systray.SetTooltip("Shutd")
		shutdownTimeItem := systray.AddMenuItem("Shutdown at ?", "Shutdown at ?")
		systray.AddSeparator()
		snoozeItem := systray.AddMenuItem("Snooze", "Snooze shutdown")
		quitItem := systray.AddMenuItem("Quit", "Quit the whole app")

		shutdownTimeItem.Disable()

		go func() {
			for {
				select {
				case t := <-s.ShutdownTimeChangedChan():
					title := fmt.Sprintf("Shutdown at %v", t.Format("15:04"))
					shutdownTimeItem.SetTitle(title)
					shutdownTimeItem.SetTooltip(title)
				case <-snoozeItem.ClickedCh:
					s.Snooze()
				case <-quitItem.ClickedCh:
					systray.Quit()
					return
				}
			}
		}()
	}

	onExit := func() {
		exit(log)
	}
	systray.Run(onReady, onExit)
}
