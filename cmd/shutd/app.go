package main

import (
	"context"
	"embed"
	"fmt"
	"time"

	"github.com/horacehylee/shutd"
	"github.com/wailsapp/wails/v2"
	"github.com/wailsapp/wails/v2/pkg/logger"
	"github.com/wailsapp/wails/v2/pkg/options"
	"github.com/wailsapp/wails/v2/pkg/options/mac"
	"github.com/wailsapp/wails/v2/pkg/options/windows"
	"github.com/wailsapp/wails/v2/pkg/runtime"
)

//go:embed frontend/src
var assets embed.FS

//go:embed build/appicon.png
var icon2 []byte

// app struct for containing wails context
type app struct {
	ctx context.Context
	log logger.Logger
}

func newApp(log logger.Logger) *app {
	return &app{
		log: log,
	}
}

func run(a *app) error {
	return wails.Run(&options.App{
		Title:         "Shutd",
		Width:         450,
		Height:        250,
		MinWidth:      225,
		MinHeight:     125,
		MaxWidth:      900,
		MaxHeight:     500,
		DisableResize: true,
		Fullscreen:    false,
		Frameless:     true,
		// StartHidden:       true,
		StartHidden:       false,
		HideWindowOnClose: true,
		// RGBA:              &options.RGBA{R: 33, G: 37, B: 43, A: 255},
		Assets:    assets,
		LogLevel:  logger.DEBUG,
		OnStartup: a.startup,
		// OnDomReady:        a.domReady,
		// OnShutdown:        a.shutdown,
		Bind: []interface{}{
			a,
		},
		// Windows platform specific options
		Windows: &windows.Options{
			WebviewIsTransparent: false,
			WindowIsTranslucent:  false,
			DisableWindowIcon:    false,
		},
		Mac: &mac.Options{
			TitleBar:             mac.TitleBarHiddenInset(),
			WebviewIsTransparent: true,
			WindowIsTranslucent:  true,
			About: &mac.AboutInfo{
				Title:   "Vanilla Template",
				Message: "Part of the Wails projects",
				Icon:    icon2,
			},
		},
	})
}

func (a *app) startup(ctx context.Context) {
	a.ctx = ctx
}

// domReady is called after the front-end dom has been loaded
// func (b *app) domReady(ctx context.Context) {
// 	// Add your action here
// }

// // shutdown is called at application termination
// func (b *app) shutdown(ctx context.Context) {
// 	// Perform your teardown here
// }

func (a *app) Dismiss() {
	a.log.Info("Dismissed")
}

func (a *app) Snooze() {
	a.log.Info("Snooze")
}

func (a *app) Greet(name string) string {
	return fmt.Sprintf("Hello %s, It's show time!", name)
}

func (a *app) newNotificationSnoozeTask() shutd.SchedulerTask {
	return func(s *shutd.Scheduler) error {
		_, err := s.ShutdownTime()
		if err != nil {
			return err
		}
		shutdownTime, err := s.ShutdownTime()
		if err != nil {
			return err
		}
		title := fmt.Sprintf("Shutd - Scheduled Shutdown at %v", shutdownTime.Format("15:04"))
		text := fmt.Sprintf("About to shutdown in %.0f minutes, wanted to snooze for %v minutes?", time.Until(shutdownTime).Minutes(), s.Config().SnoozeInterval)
		runtime.EventsEmit(a.ctx, "setText", map[string]string{
			"title": title,
			"text":  text,
		})
		runtime.WindowShow(a.ctx)

		// ctx, cancel := context.WithTimeout(context.Background(), time.Duration(s.Config().Notification.Duration)*time.Minute)
		// defer cancel()
		// yes, err := question(ctx, title, text)
		// if err != nil && !errors.Is(err, context.Canceled) && !errors.Is(err, context.DeadlineExceeded) {
		// 	return fmt.Errorf("failed to display snooze notification: %v", err)
		// }
		// s.Logger().Infof("snooze is required: %v", yes)
		// if yes {
		// 	err := s.Snooze()
		// 	if err != nil {
		// 		return err
		// 	}
		// }
		return nil
	}
}
