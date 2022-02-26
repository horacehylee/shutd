package main

import (
	"fmt"
	"os"
	"path"

	"github.com/sirupsen/logrus"
)

type log struct {
	*logrus.Logger
}

func (l *log) Print(message string) {
	l.Logger.Print(message)
}
func (l *log) Trace(message string) {
	l.Logger.Trace(message)
}
func (l *log) Debug(message string) {
	l.Logger.Debug(message)
}
func (l *log) Info(message string) {
	l.Logger.Info(message)
}
func (l *log) Warning(message string) {
	l.Logger.Warning(message)
}
func (l *log) Error(message string) {
	l.Logger.Error(message)
}
func (l *log) Fatal(message string) {
	l.Logger.Fatal(message)
}

func newLogger() (*os.File, *log) {
	l := logrus.New()
	l.Formatter = new(logrus.TextFormatter)
	l.Formatter.(*logrus.TextFormatter).DisableColors = true
	l.Formatter.(*logrus.TextFormatter).FullTimestamp = true
	l.Level = logrus.InfoLevel

	dirname, err := os.UserHomeDir()
	if err != nil {
		l.Fatal(fmt.Errorf("failed to get home dir: %w", err))
	}
	file, err := os.OpenFile(path.Join(dirname, ".shutd.log"), os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0660)
	if err != nil {
		l.Fatal(fmt.Errorf("failed to log to file: %w", err))
	}
	l.Out = file
	ll := &log{Logger: l}
	return file, ll
}
