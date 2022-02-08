package main

import (
	"errors"
	"fmt"
	"os"
	"os/signal"
	"path"
	"syscall"

	"github.com/fsnotify/fsnotify"
	"github.com/getlantern/systray"
	"github.com/horacehylee/shutd"
	"github.com/horacehylee/shutd/cmd/shutd/icon"
	"github.com/sirupsen/logrus"
	"github.com/spf13/viper"
)

func main() {
	logFile, log := newLogger()
	defer logFile.Close()

	log.Info("==========================")
	log.Info("Started")
	log.Info("==========================")

	config := newConfig(log)
	s, err := shutd.NewScheduler(config, shutd.WithLogger(log))
	if err != nil {
		log.Fatalf("failed create scheduler: %w", err)
	}

	watchConfig(log, func(config shutd.Config) {
		err := s.Configure(config)
		if err != nil {
			log.Fatalf("failed to apply updated config: %w", err)
		}
	})

	watchExit(log)

	startSystray(log, s)
}

func newLogger() (*os.File, *logrus.Logger) {
	log := logrus.New()
	log.Formatter = new(logrus.TextFormatter)
	log.Formatter.(*logrus.TextFormatter).DisableColors = true
	log.Formatter.(*logrus.TextFormatter).FullTimestamp = true
	log.Level = logrus.InfoLevel

	dirname, err := os.UserHomeDir()
	if err != nil {
		log.Fatal(fmt.Errorf("failed to get home dir: %w", err))
	}
	file, err := os.OpenFile(path.Join(dirname, ".shutd.log"), os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0660)
	if err != nil {
		log.Fatal(fmt.Errorf("failed to log to file: %w", err))
	}
	log.Out = file
	return file, log
}

func newConfig(log *logrus.Logger) shutd.Config {
	viper.SetConfigName(".shutd")
	viper.SetConfigType("yaml")
	viper.AddConfigPath("$HOME")

	viper.SetDefault("startTime", "01:00")
	viper.SetDefault("snoozeInterval", 15)
	viper.SetDefault("notification.before", 10)
	viper.SetDefault("notification.duration", 10)

	err := viper.ReadInConfig()
	if err != nil {
		var e *viper.ConfigFileNotFoundError
		if !errors.As(err, &e) {
			log.Fatal(fmt.Errorf("could not read config: %w", err))
		}
	}
	config := parseConfig(log)

	err = viper.SafeWriteConfig()
	if err != nil {
		var e viper.ConfigFileAlreadyExistsError
		if !errors.As(err, &e) {
			log.Fatal(fmt.Errorf("failed to write config: %w", err))
		}
	}
	return config
}

func watchConfig(log *logrus.Logger, configFunc func(config shutd.Config)) {
	viper.OnConfigChange(func(e fsnotify.Event) {
		log.Info("==========================")
		log.Info("Config file changed:", e.Name)
		log.Info("==========================")
		config := parseConfig(log)
		configFunc(config)
	})
	viper.WatchConfig()
}

func parseConfig(log *logrus.Logger) shutd.Config {
	var config shutd.Config
	err := viper.Unmarshal(&config)
	if err != nil {
		log.Fatal(fmt.Errorf("failed to parse config: %w", err))
	}
	return config
}

func exit(log *logrus.Logger) {
	log.Info("==========================")
	log.Info("Exited")
	log.Info("==========================")
}

func watchExit(log *logrus.Logger) {
	sigs := make(chan os.Signal, 1)
	signal.Notify(sigs, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-sigs
		exit(log)
		os.Exit(0)
	}()
}

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
