package main

import (
	"errors"
	"fmt"

	"github.com/fsnotify/fsnotify"
	"github.com/horacehylee/shutd"
	"github.com/sirupsen/logrus"
	"github.com/spf13/viper"
)

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
