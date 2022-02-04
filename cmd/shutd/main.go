package main

import (
	"errors"
	"fmt"
	"log"
	"os"
	"path"

	"github.com/fsnotify/fsnotify"
	"github.com/horacehylee/shutd/pkg/shutdown"
	"github.com/sirupsen/logrus"
	"github.com/spf13/viper"
)

func main() {
	logFile, log := newLogger()
	defer logFile.Close()

	log.Info("==========================")
	log.Info("Started")
	log.Info("==========================")

	s := shutdown.NewScheduler(shutdown.WithLogger(log))
	config := newConfig(log)
	err := s.Config(config)
	if err != nil {
		log.Fatalf("failed to apply config: %w", err)
	}

	watchConfig(log, func(config shutdown.Config) {
		err := s.Config(config)
		if err != nil {
			log.Fatalf("failed to apply config: %w", err)
		}
	})

	// Block
	<-make(chan bool)
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

func newConfig(log *logrus.Logger) shutdown.Config {
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
	config := parseConfig()

	err = viper.SafeWriteConfig()
	if err != nil {
		var e viper.ConfigFileAlreadyExistsError
		if !errors.As(err, &e) {
			log.Fatal(fmt.Errorf("failed to write config: %w", err))
		}
	}
	return config
}

func watchConfig(log *logrus.Logger, configFunc func(config shutdown.Config)) {
	viper.OnConfigChange(func(e fsnotify.Event) {
		log.Info("==========================")
		log.Info("Config file changed:", e.Name)
		log.Info("==========================")
		config := parseConfig()
		configFunc(config)
	})
	viper.WatchConfig()
}

func parseConfig() shutdown.Config {
	var config shutdown.Config
	err := viper.Unmarshal(&config)
	if err != nil {
		log.Fatal(fmt.Errorf("failed to parse config: %w", err))
	}
	return config
}
