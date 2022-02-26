package main

import (
	"os"
	"os/signal"
	"syscall"

	"github.com/horacehylee/shutd"
	"github.com/sirupsen/logrus"
)

func main() {
	logFile, log := newLogger()
	defer logFile.Close()

	log.Info("==========================")
	log.Info("Started")
	log.Info("==========================")

	config := newConfig(log.Logger)
	app := newApp(log)
	s, err := shutd.NewScheduler(config,
		shutd.WithLogger(log.Logger),
		shutd.WithSnoozeNotificationTask(app.newNotificationSnoozeTask()),
	)
	if err != nil {
		log.Fatalf("failed create scheduler: %w", err)
	}

	watchConfig(log.Logger, func(config shutd.Config) {
		err := s.Configure(config)
		if err != nil {
			log.Fatalf("failed to apply updated config: %w", err)
		}
	})

	watchExit(log.Logger)

	err = run(app)
	if err != nil {
		log.Logger.Fatal(err)
	}
	// startSystray(log.Logger, s)
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
